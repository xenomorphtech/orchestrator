use std::io::{self, BufRead, Write};
use std::process::{Command, ExitCode};

/// Workaround: SpacetimeDB publish greps for the literal `println` macro
/// across all .rs files, even CLI-only binaries.
macro_rules! out {
    () => { writeln!(std::io::stdout()).unwrap() };
    ($($arg:tt)*) => { writeln!(std::io::stdout(), $($arg)*).unwrap() };
}

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const DEFAULT_BIOME_TERM_URL: &str = "http://127.0.0.1:3021";
const REPEAT_STUCK_SECONDS: i64 = 600;
const SPINNER_CHARS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏◐◓◑◒";
const STUCK_PATTERNS: [&str; 7] = [
    "traceback",
    "exception",
    "error:",
    "permission denied",
    "command not found",
    "no such file",
    "segmentation fault",
];

#[derive(Parser)]
#[command(name = "harness")]
#[command(about = "Rust CLI for the harness SpacetimeDB module")]
struct Cli {
    #[arg(long, default_value = "orchestrator-harness", env = "HARNESS_DATABASE")]
    database: String,
    #[arg(long, default_value = "http://127.0.0.1:3000", env = "HARNESS_SERVER")]
    server: String,
    #[arg(long, default_value = DEFAULT_BIOME_TERM_URL, env = "HARNESS_BIOME_URL")]
    biome_url: String,
    #[arg(long, env = "HARNESS_BIOME_API_KEY", default_value = "")]
    biome_api_key: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build,
    SeedAgents,
    BootstrapKnownGoals,
    Agents,
    Goals,
    SubGoals,
    Facts,
    Summary,
    DecideActions,
    ResolveActiveSubGoals,
    PollBiome(PollBiomeArgs),
    ExecuteBiome(ExecuteBiomeArgs),
    RunOnceBiome(RunOnceBiomeArgs),
    QueuePrompt(QueuePromptArgs),
    FactSet(FactSetArgs),
    GoalAdd(GoalAddArgs),
    GoalUpdate(GoalUpdateArgs),
    GoalRemove(GoalRemoveArgs),
    GoalSet(GoalSetArgs),
    SubGoalAdd(SubGoalAddArgs),
    SubGoalUpdate(SubGoalUpdateArgs),
    SubGoalRemove(SubGoalRemoveArgs),
    SubGoalSet(SubGoalSetArgs),
    AgentAdd(AgentAddArgs),
    /// Create a biome_term pane, run a command, and register the agent
    AgentCreate(AgentCreateArgs),
    AgentRemove(AgentRemoveArgs),
    Send(SendArgs),
    /// Update a sub-goal's completion report and optionally set status/facts
    Checkpoint(CheckpointArgs),
    /// Set supervisor heartbeat facts for a domain
    Heartbeat(HeartbeatArgs),
    /// Add an episodic memory entry for the current orchestration cycle
    EpisodeAdd(EpisodeAddArgs),
    /// Query recent episodic memory entries
    Episodes(EpisodesArgs),
    /// Update an agent's rolling description
    AgentDescribe(AgentDescribeArgs),
    /// List registered services
    Services,
    /// Register or update a service health check
    ServiceAdd(ServiceAddArgs),
    /// Remove a service
    ServiceRemove(ServiceRemoveArgs),
    /// Poll all services and record health
    PollServices(PollServicesArgs),
    /// Show the current screen of a biome_term pane
    Screen(ScreenArgs),
    /// List all biome_term panes
    Panes,
    /// Dump goals, sub-goals, and facts as JSONL to stdout
    Dump,
    /// Restore goals, sub-goals, and facts from JSONL on stdin
    Restore,
}

struct CliContext {
    database: String,
    server: String,
    biome_url: String,
    biome_api_key: String,
    client: Client,
}

impl CliContext {
    fn biome_get(&self, url: &str) -> reqwest::blocking::RequestBuilder {
        let mut req = self.client.get(url).timeout(std::time::Duration::from_millis(2000));
        if !self.biome_api_key.is_empty() {
            req = req.header("X-Api-Key", &self.biome_api_key);
        }
        req
    }

    fn biome_post(&self, url: &str) -> reqwest::blocking::RequestBuilder {
        let mut req = self.client.post(url).timeout(std::time::Duration::from_millis(2000));
        if !self.biome_api_key.is_empty() {
            req = req.header("X-Api-Key", &self.biome_api_key);
        }
        req
    }

    fn biome_delete(&self, url: &str) -> reqwest::blocking::RequestBuilder {
        let mut req = self.client.delete(url).timeout(std::time::Duration::from_millis(2000));
        if !self.biome_api_key.is_empty() {
            req = req.header("X-Api-Key", &self.biome_api_key);
        }
        req
    }
}

#[derive(Args)]
struct PollBiomeArgs {
    #[arg(long, default_value_t = 20)]
    lines: u32,
    /// Only poll agents whose metadata_json.domain matches
    #[arg(long)]
    domain: Option<String>,
}

#[derive(Args)]
struct ExecuteBiomeArgs {
    #[arg(long, default_value_t = 10)]
    limit: u32,
    /// Only execute actions for agents whose metadata_json.domain matches
    #[arg(long)]
    domain: Option<String>,
    /// Set biome pane group tag on spawn/restart (defaults to domain if set)
    #[arg(long)]
    group: Option<String>,
}

#[derive(Args)]
struct RunOnceBiomeArgs {
    #[arg(long, default_value_t = 20)]
    lines: u32,
    #[arg(long)]
    execute: bool,
    #[arg(long, default_value_t = 10)]
    limit: u32,
    /// Scope poll/decide/execute to agents whose metadata_json.domain matches
    #[arg(long)]
    domain: Option<String>,
    /// Set biome pane group tag on spawn/restart (defaults to domain if set)
    #[arg(long)]
    group: Option<String>,
}

#[derive(Args)]
struct QueuePromptArgs {
    agent_name: String,
    text: String,
}

#[derive(Args)]
struct FactSetArgs {
    key: String,
    value: String,
    #[arg(long, default_value_t = 1.0)]
    confidence: f64,
    #[arg(long, default_value = "manual")]
    source_type: String,
    #[arg(long)]
    source_ref: Option<String>,
    /// JSON metadata (e.g. '{"domain":"nmss"}')
    #[arg(long)]
    metadata: Option<String>,
}

#[derive(Args)]
struct GoalAddArgs {
    goal_key: String,
    title: String,
    #[arg(long, default_value = "")]
    detail: String,
    #[arg(long, default_value = "pending")]
    status: String,
    #[arg(long, default_value_t = 50)]
    priority: u32,
    #[arg(long)]
    depends_on_goal_key: Option<String>,
    #[arg(long)]
    success_fact_key: Option<String>,
    /// JSON metadata (e.g. '{"domain":"nmss"}')
    #[arg(long)]
    metadata: Option<String>,
}

#[derive(Args)]
struct GoalUpdateArgs {
    goal_key: String,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    detail: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    priority: Option<u32>,
    #[arg(long)]
    depends_on_goal_key: Option<String>,
    #[arg(long)]
    success_fact_key: Option<String>,
    #[arg(long)]
    clear_depends: bool,
    #[arg(long)]
    clear_success_fact: bool,
}

#[derive(Args)]
struct GoalRemoveArgs {
    goal_key: String,
    #[arg(long)]
    delete: bool,
    #[arg(long)]
    cascade: bool,
}

#[derive(Args)]
struct GoalSetArgs {
    goal_key: String,
    status: String,
}

#[derive(Args)]
struct SubGoalAddArgs {
    sub_goal_key: String,
    goal_key: String,
    owner_agent: String,
    title: String,
    #[arg(long, default_value = "")]
    detail: String,
    #[arg(long, default_value = "pending")]
    status: String,
    #[arg(long, default_value_t = 50)]
    priority: u32,
    #[arg(long)]
    depends_on_sub_goal_key: Option<String>,
    #[arg(long)]
    success_fact_key: Option<String>,
    #[arg(long)]
    instruction_text: Option<String>,
    #[arg(long)]
    stuck_guidance_text: Option<String>,
    /// JSON metadata (e.g. '{"domain":"nmss","restart_policy":"one_for_one"}')
    #[arg(long)]
    metadata: Option<String>,
}

#[derive(Args)]
struct SubGoalUpdateArgs {
    sub_goal_key: String,
    #[arg(long)]
    goal_key: Option<String>,
    #[arg(long)]
    owner_agent: Option<String>,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    detail: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    priority: Option<u32>,
    #[arg(long)]
    depends_on_sub_goal_key: Option<String>,
    #[arg(long)]
    success_fact_key: Option<String>,
    #[arg(long)]
    instruction_text: Option<String>,
    #[arg(long)]
    stuck_guidance_text: Option<String>,
    #[arg(long)]
    clear_depends: bool,
    #[arg(long)]
    clear_success_fact: bool,
    #[arg(long)]
    clear_instruction: bool,
    #[arg(long)]
    clear_stuck_guidance: bool,
}

#[derive(Args)]
struct SubGoalRemoveArgs {
    sub_goal_key: String,
    #[arg(long)]
    delete: bool,
}

#[derive(Args)]
struct SubGoalSetArgs {
    sub_goal_key: String,
    status: String,
}

#[derive(Args)]
struct AgentAddArgs {
    name: String,
    #[arg(long)]
    biome_pane_id: String,
    #[arg(long)]
    workdir: Option<String>,
    #[arg(long)]
    default_task: Option<String>,
    #[arg(long)]
    tmux_target: Option<String>,
    /// JSON metadata (e.g. '{"domain":"nmss","backend":"claude","role":"worker"}')
    #[arg(long)]
    metadata: Option<String>,
}

#[derive(Args)]
struct AgentCreateArgs {
    /// Agent name (also used as the biome_term pane name)
    name: String,
    /// Command to run in the pane (e.g. "cd /path && claude --dangerously-skip-permissions")
    #[arg(long, short)]
    cmd: Option<String>,
    #[arg(long)]
    workdir: Option<String>,
    #[arg(long)]
    default_task: Option<String>,
    /// Seconds to wait after running cmd before sending the task prompt (default 8)
    #[arg(long, default_value_t = 8)]
    init_wait: u64,
    /// Initial task prompt to send after the agent starts
    #[arg(long, short)]
    task: Option<String>,
}

#[derive(Args)]
struct AgentRemoveArgs {
    name: String,
    #[arg(long)]
    delete: bool,
}

#[derive(Args)]
struct SendArgs {
    /// Pane name or UUID
    pane: String,
    /// Text to send
    text: String,
    /// Delay in ms before sending the trailing carriage return (default 150)
    #[arg(long, default_value_t = 150)]
    delay: u64,
}

#[derive(Args)]
struct CheckpointArgs {
    /// Sub-goal key to checkpoint
    sub_goal_key: String,
    /// Completion report text
    #[arg(long)]
    report: String,
    /// Optionally set status (e.g. "done", "blocked", "failed")
    #[arg(long)]
    status: Option<String>,
    /// Set facts as key=value pairs (repeatable)
    #[arg(long = "fact", value_name = "KEY=VALUE")]
    facts: Vec<String>,
}

#[derive(Args)]
struct HeartbeatArgs {
    /// Domain name (sets <domain>.supervisor.status and .last_heartbeat)
    domain: String,
    /// Optional status override (default: "alive")
    #[arg(long, default_value = "alive")]
    status: String,
}

#[derive(Args)]
struct ScreenArgs {
    /// Pane name or UUID
    pane: String,
    /// Number of trailing lines to show (0 = all)
    #[arg(long, default_value_t = 0)]
    lines: usize,
}

#[derive(Args)]
struct EpisodeAddArgs {
    /// Cycle summary text
    summary: String,
    /// JSON snapshot of agent statuses
    #[arg(long)]
    agent_statuses: String,
    /// JSON of actions taken this cycle
    #[arg(long)]
    actions_taken: String,
    /// JSON of goal progress
    #[arg(long)]
    goal_progress: String,
}

#[derive(Args)]
struct EpisodesArgs {
    /// Number of recent episodes to show (default 5)
    #[arg(long, default_value_t = 5)]
    limit: u32,
}

#[derive(Args)]
struct AgentDescribeArgs {
    /// Agent name
    name: String,
    /// Rolling description text
    description: String,
}

#[derive(Args)]
struct ServiceAddArgs {
    /// Service name (unique identifier)
    name: String,
    /// Service type: systemd, http, tcp, ssh_systemd
    #[arg(long)]
    service_type: String,
    /// Check target: unit name, URL, or host:port
    #[arg(long)]
    check_target: String,
    /// Host for remote checks (default: localhost)
    #[arg(long, default_value = "localhost")]
    host: String,
    /// Restart policy: auto or manual (default: manual)
    #[arg(long, default_value = "manual")]
    restart_policy: String,
    /// Custom restart command
    #[arg(long)]
    restart_command: Option<String>,
    /// JSON metadata
    #[arg(long)]
    metadata: Option<String>,
}

#[derive(Args)]
struct ServiceRemoveArgs {
    name: String,
    #[arg(long)]
    delete: bool,
}

#[derive(Args)]
struct PollServicesArgs {
    /// Timeout in ms for each check (default 5000)
    #[arg(long, default_value_t = 5000)]
    timeout_ms: u64,
}

// ── Biome term HTTP helpers ─────────────────────────────────────────────

#[derive(Deserialize)]
struct BiomeScreen {
    rows: Vec<String>,
}

#[derive(Deserialize)]
struct BiomePaneCreated {
    id: String,
}

fn biome_screen(cli: &CliContext, pane_id: &str, lines: usize) -> Result<String> {
    let resp = cli.biome_get(&format!("{}/panes/{pane_id}/screen", cli.biome_url))
        .send()
        .with_context(|| format!("biome screen request for {pane_id}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("biome screen failed with {}", resp.status()));
    }
    let screen: BiomeScreen = resp.json().context("parsing biome screen")?;
    let len = screen.rows.len();
    let start = len.saturating_sub(lines);
    Ok(screen.rows[start..].join("\n"))
}

fn biome_send_raw(cli: &CliContext, pane_id: &str, data: &[u8]) -> Result<()> {
    let payload = json!({ "data": BASE64_STANDARD.encode(data) });
    let resp = cli.biome_post(&format!("{}/panes/{pane_id}/input", cli.biome_url))
        .json(&payload)
        .send()
        .with_context(|| format!("biome send for {pane_id}"))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("biome input failed with {}", resp.status()))
    }
}

fn biome_send_text(cli: &CliContext, pane_id: &str, text: &str) -> Result<()> {
    biome_send_raw(cli, pane_id, format!("{text}\r").as_bytes())
}

fn biome_send_text_delayed(cli: &CliContext, pane_id: &str, text: &str, delay_ms: u64) -> Result<()> {
    biome_send_raw(cli, pane_id, text.as_bytes())?;
    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    biome_send_raw(cli, pane_id, b"\r")
}

fn biome_resolve_pane(cli: &CliContext, name_or_id: &str) -> Result<String> {
    // If it looks like a UUID, use it directly
    if name_or_id.contains('-') && name_or_id.len() > 30 {
        return Ok(name_or_id.to_string());
    }
    // Otherwise resolve by name
    let resp = cli.biome_get(&format!("{}/panes", cli.biome_url))
        .send()
        .context("listing panes")?;
    let panes: Vec<Value> = resp.json().context("parsing panes list")?;
    for pane in &panes {
        if pane.get("name").and_then(|v| v.as_str()) == Some(name_or_id) {
            return pane
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("pane {name_or_id} has no id"));
        }
    }
    // Try prefix match on id
    for pane in &panes {
        if let Some(id) = pane.get("id").and_then(|v| v.as_str()) {
            if id.starts_with(name_or_id) {
                return Ok(id.to_string());
            }
        }
    }
    Err(anyhow!("no pane matching '{name_or_id}'"))
}

fn biome_create_pane(cli: &CliContext, name: &str, group: Option<&str>) -> Result<String> {
    let mut body = json!({"name": name, "cols": 220, "rows": 50});
    if let Some(g) = group {
        body["group"] = json!(g);
    }
    let resp = cli
        .biome_post(&format!("{}/panes", cli.biome_url))
        .json(&body)
        .send()
        .context("biome create pane")?;
    if !resp.status().is_success() {
        return Err(anyhow!("biome create pane failed with {}", resp.status()));
    }
    let created: BiomePaneCreated = resp.json().context("parsing pane create response")?;
    Ok(created.id)
}

fn biome_delete_pane(cli: &CliContext, pane_id: &str) -> Result<()> {
    let resp = cli.biome_delete(&format!("{}/panes/{pane_id}", cli.biome_url))
        .send()
        .with_context(|| format!("biome delete pane {pane_id}"))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("biome delete pane failed with {}", resp.status()))
    }
}

fn cmd_panes(cli: &CliContext) -> Result<()> {
    let resp = cli.biome_get(&format!("{}/panes", cli.biome_url))
        .send()
        .context("listing panes")?;
    if !resp.status().is_success() {
        return Err(anyhow!("biome panes failed with {}", resp.status()));
    }
    let body: Value = resp.json().context("parsing panes list")?;
    out!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

// ── Capture classification (ported from lib.rs) ─────────────────────────

fn nonempty_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn looks_idle(text: &str) -> bool {
    let lines = nonempty_lines(text);
    let Some(tail) = lines.last() else {
        return false;
    };
    tail.starts_with('❯') || tail == ">" || tail.ends_with(" ❯")
}

fn looks_working(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["thinking", "analyzing", "processing", "working", "running"]
        .iter()
        .any(|token| lower.contains(token))
        || text.chars().any(|ch| SPINNER_CHARS.contains(ch))
}

fn looks_stuck(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    STUCK_PATTERNS.iter().any(|pattern| lower.contains(pattern))
}

fn classify_capture(
    text: Option<&str>,
    previous_hash: Option<&str>,
    previous_seen_at: Option<&str>,
) -> String {
    let Some(text) = text else {
        return "dead".to_string();
    };
    if looks_stuck(text) {
        return "stuck".to_string();
    }
    if looks_idle(text) {
        return "idle".to_string();
    }
    if looks_working(text) {
        return "working".to_string();
    }
    let current_hash = hash_text(text);
    if let (Some(previous_hash), Some(previous_seen_at)) = (previous_hash, previous_seen_at) {
        if current_hash == previous_hash
            && DateTime::parse_from_rfc3339(previous_seen_at)
                .ok()
                .map(|then| {
                    Utc::now()
                        .signed_duration_since(then.with_timezone(&Utc))
                        .num_seconds()
                        >= REPEAT_STUCK_SECONDS
                })
                .unwrap_or(false)
        {
            return "stuck".to_string();
        }
    }
    "working".to_string()
}

// ── SQL query helpers ───────────────────────────────────────────────────

/// Execute SQL and return raw parsed JSON array of result sets.
fn sql_query(cli: &CliContext, query: &str) -> Result<Vec<Value>> {
    let url = format!("{}/v1/database/{}/sql", cli.server, cli.database);
    let resp = cli
        .client
        .post(&url)
        .body(query.to_string())
        .send()
        .with_context(|| format!("failed to run SQL: {query}"))?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("SQL failed (HTTP {status}): {text}"));
    }
    let json: Value = serde_json::from_str(&text).context("parsing SQL response")?;
    Ok(json.as_array().cloned().unwrap_or_default())
}

/// Extract rows from the first result set as Vec<Vec<Value>> (column-indexed arrays).
fn sql_rows(cli: &CliContext, query: &str) -> Result<Vec<Vec<Value>>> {
    let results = sql_query(cli, query)?;
    let Some(first) = results.first() else {
        return Ok(Vec::new());
    };
    let rows = first
        .get("rows")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .filter_map(|row| row.as_array().cloned())
        .collect())
}

/// Decode a BSATN optional value: [0, x] → Some(x), [1, []] → None, other → as-is.
fn bsatn_unwrap(val: &Value) -> Option<String> {
    match val {
        Value::Null => None,
        Value::String(s) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        Value::Array(arr) if arr.len() == 2 => match arr[0].as_u64() {
            Some(0) => {
                let inner = &arr[1];
                match inner {
                    Value::String(s) => Some(s.clone()),
                    _ => Some(inner.to_string()),
                }
            }
            Some(1) => None,
            _ => Some(val.to_string()),
        },
        _ => Some(val.to_string()),
    }
}

fn bsatn_unwrap_or(val: &Value, default: &str) -> String {
    bsatn_unwrap(val).unwrap_or_else(|| default.to_string())
}

fn metadata_matches_domain(metadata_json: &str, domain: &str) -> bool {
    serde_json::from_str::<Value>(metadata_json)
        .ok()
        .and_then(|v| v.get("domain").and_then(|d| d.as_str()).map(|d| d == domain))
        .unwrap_or(false)
}

fn get_agent_metadata(cli: &CliContext, agent_name: &str) -> Result<String> {
    let rows = sql_rows(
        cli,
        &format!(
            "SELECT metadata_json FROM agents WHERE name = '{}'",
            agent_name.replace('\'', "''")
        ),
    )?;
    Ok(rows
        .first()
        .and_then(|row| row.first())
        .map(|v| bsatn_unwrap_or(v, "{}"))
        .unwrap_or_else(|| "{}".to_string()))
}

// ── Biome-aware CLI commands ────────────────────────────────────────────

fn cmd_poll_biome(cli: &CliContext, lines: u32, domain: Option<&str>) -> Result<()> {
    let lines = lines as usize;
    // Fetch agents with biome pane IDs.
    let rows = sql_rows(
        cli,
        "SELECT name, biome_pane_id, last_seen_at, last_capture_hash, metadata_json FROM agents",
    )?;

    let mut results: Vec<Value> = Vec::new();

    for row in &rows {
        let name = bsatn_unwrap_or(&row[0], "");
        let pane_id = match bsatn_unwrap(&row[1]) {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };
        let last_seen_at = bsatn_unwrap(&row[2]);
        let last_capture_hash = bsatn_unwrap(&row[3]);
        let metadata = bsatn_unwrap_or(&row[4], "{}");

        if let Some(domain) = domain {
            if !metadata_matches_domain(&metadata, domain) {
                continue;
            }
        }

        // Read screen from biome_term
        let capture = biome_screen(cli, &pane_id, lines).ok();

        // Classify
        let status = classify_capture(
            capture.as_deref(),
            last_capture_hash.as_deref(),
            last_seen_at.as_deref(),
        );

        let content = capture.unwrap_or_default();
        let content_hash = if content.is_empty() {
            String::new()
        } else {
            hash_text(&content)
        };
        let preview_lines: Vec<_> = nonempty_lines(&content);
        let preview: String = preview_lines
            .iter()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        // Record poll result in SpacetimeDB via agent_poll_record reducer
        call_reducer_silent(
            cli,
            "agent_poll_record",
            Some(vec![json!({
                "agent_name": name,
                "status": status,
                "last_capture_hash": if content_hash.is_empty() { none_json() } else { some_json_string(content_hash.clone()) },
                "last_capture_preview": if preview.is_empty() { none_json() } else { some_json_string(preview.clone()) }
            })]),
        )?;

        // Also record observation
        if !content.is_empty() {
            let _ = call_reducer_silent(
                cli,
                "observation_add",
                Some(vec![
                    Value::String(name.clone()),
                    Value::String("biome_capture".to_string()),
                    Value::String(content),
                    Value::String(content_hash.clone()),
                ]),
            );
        }

        results.push(json!({
            "agent": name,
            "status": status,
            "content_hash": content_hash,
            "preview": preview,
        }));
    }

    out!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

fn cmd_execute_biome(cli: &CliContext, limit: u32, domain: Option<&str>, group: Option<&str>) -> Result<()> {
    let limit = limit as usize;
    // Fetch pending actions
    let rows = sql_rows(
        cli,
        "SELECT id, agent_name, action_type, payload_json, reason FROM actions WHERE status = 'pending'",
    )?;

    let mut results: Vec<Value> = Vec::new();

    for row in rows.into_iter().take(limit) {
        let action_id = row[0].as_u64().unwrap_or(0);
        let agent_name = bsatn_unwrap(&row[1]);
        let action_type = bsatn_unwrap_or(&row[2], "");
        let payload_json_str = bsatn_unwrap_or(&row[3], "{}");

        if let Some(domain) = domain {
            if let Some(ref agent_name) = agent_name {
                let metadata = get_agent_metadata(cli, agent_name).unwrap_or_else(|_| "{}".to_string());
                if !metadata_matches_domain(&metadata, domain) {
                    continue;
                }
            }
        }

        let (status, result_text) = match action_type.as_str() {
            "send_prompt" | "restart_agent" => {
                match agent_name.as_deref().filter(|s| !s.is_empty()) {
                    None => ("failed".to_string(), format!("action {action_id} missing agent_name")),
                    Some(agent_name) if action_type == "send_prompt" => {
                        match get_agent_pane_id(cli, agent_name)? {
                            None => ("failed".to_string(), format!("agent {agent_name} has no biome_pane_id")),
                            Some(pane_id) => {
                                let payload: Value = serde_json::from_str(&payload_json_str)
                                    .unwrap_or_else(|_| json!({}));
                                let text = payload
                                    .get("text")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                match biome_send_text(cli, &pane_id, text) {
                                    Ok(()) => ("done".to_string(), "sent".to_string()),
                                    Err(err) => ("failed".to_string(), format!("{err:#}")),
                                }
                            }
                        }
                    }
                    Some(agent_name) => {
                        execute_restart_agent(cli, agent_name, &payload_json_str, group)
                    }
                }
            }
            "spawn_agent" => execute_spawn_agent(cli, &payload_json_str, group),
            "restart_service" => execute_restart_service(&payload_json_str),
            _ => (
                "failed".to_string(),
                format!("unsupported action type: {action_type}"),
            ),
        };

        // Record result via action_complete reducer
        call_reducer_silent(
            cli,
            "action_complete",
            Some(vec![
                json!(action_id),
                Value::String(status.clone()),
                some_json_string(result_text.clone()),
            ]),
        )?;

        results.push(json!({
            "action_id": action_id,
            "status": status,
            "result": result_text,
        }));
    }

    out!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

fn cmd_run_once_biome(cli: &CliContext, lines: u32, execute: bool, limit: u32, domain: Option<&str>, group: Option<&str>) -> Result<()> {
    cmd_poll_biome(cli, lines, domain)?;
    match domain {
        Some(d) => call_reducer_silent(
            cli,
            "decide_actions_for_domain",
            Some(vec![Value::String(d.to_string())]),
        )?,
        None => call_reducer_silent(cli, "decide_actions", None)?,
    }
    if execute {
        // Use explicit --group if given, otherwise fall back to domain
        let effective_group = group.or(domain);
        cmd_execute_biome(cli, limit, domain, effective_group)?;
    }
    Ok(())
}

fn cmd_poll_services(cli: &CliContext, timeout_ms: u64) -> Result<()> {
    let rows = sql_rows(
        cli,
        "SELECT name, service_type, host, check_target, restart_command FROM services",
    )?;

    let timeout = std::time::Duration::from_millis(timeout_ms);
    let mut results: Vec<Value> = Vec::new();

    for row in &rows {
        let name = bsatn_unwrap_or(&row[0], "");
        let svc_type = bsatn_unwrap_or(&row[1], "");
        let host = bsatn_unwrap_or(&row[2], "localhost");
        let check_target = bsatn_unwrap_or(&row[3], "");

        let start = std::time::Instant::now();
        let (status, detail) = match svc_type.as_str() {
            "systemd" => check_systemd(&check_target, timeout),
            "ssh_systemd" => check_ssh_systemd(&host, &check_target, timeout),
            "http" => check_http(&cli.client, &check_target, timeout),
            "tcp" => check_tcp(&check_target, timeout),
            _ => ("unhealthy".to_string(), format!("unknown service_type: {svc_type}")),
        };
        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Record health via reducer
        let _ = call_reducer_silent(
            cli,
            "service_health_record",
            Some(vec![json!({
                "service_name": name,
                "status": status,
                "detail": optional_json_string(Some(detail.clone())),
                "response_time_ms": some_json_u64(elapsed_ms),
            })]),
        );

        results.push(json!({
            "service": name,
            "type": svc_type,
            "status": status,
            "detail": detail,
            "response_time_ms": elapsed_ms,
        }));
    }

    out!("{}", serde_json::to_string_pretty(&results)?);
    Ok(())
}

fn check_systemd(unit: &str, _timeout: std::time::Duration) -> (String, String) {
    match Command::new("systemctl")
        .args(["is-active", unit])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout == "active" {
                ("healthy".to_string(), "active".to_string())
            } else {
                ("unhealthy".to_string(), stdout)
            }
        }
        Err(err) => ("unhealthy".to_string(), format!("systemctl failed: {err}")),
    }
}

fn check_ssh_systemd(host: &str, unit: &str, timeout: std::time::Duration) -> (String, String) {
    let timeout_secs = (timeout.as_secs()).max(1).to_string();
    match Command::new("ssh")
        .args([
            "-o", &format!("ConnectTimeout={timeout_secs}"),
            "-o", "StrictHostKeyChecking=accept-new",
            host,
            "systemctl", "is-active", unit,
        ])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout == "active" {
                ("healthy".to_string(), "active".to_string())
            } else {
                ("unhealthy".to_string(), stdout)
            }
        }
        Err(err) => ("unhealthy".to_string(), format!("ssh systemctl failed: {err}")),
    }
}

fn check_http(client: &Client, url: &str, timeout: std::time::Duration) -> (String, String) {
    match client.get(url).timeout(timeout).send() {
        Ok(resp) => {
            let status_code = resp.status();
            if status_code.is_success() {
                ("healthy".to_string(), format!("{}", status_code.as_u16()))
            } else {
                ("unhealthy".to_string(), format!("HTTP {}", status_code.as_u16()))
            }
        }
        Err(err) => ("unhealthy".to_string(), format!("{err}")),
    }
}

fn check_tcp(target: &str, timeout: std::time::Duration) -> (String, String) {
    match target.parse::<std::net::SocketAddr>() {
        Ok(addr) => match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => ("healthy".to_string(), "connected".to_string()),
            Err(err) => ("unhealthy".to_string(), format!("{err}")),
        },
        Err(err) => ("unhealthy".to_string(), format!("bad address: {err}")),
    }
}

fn some_json_u64(value: u64) -> Value {
    json!({ "some": value })
}

fn boot_command_for_backend(backend: &str, workdir: &str) -> String {
    match backend {
        "codex" => format!("cd {workdir} && codex --full-auto"),
        "claude" | _ => format!("cd {workdir} && claude --dangerously-skip-permissions"),
    }
}

fn execute_restart_agent(cli: &CliContext, agent_name: &str, payload_json_str: &str, group_override: Option<&str>) -> (String, String) {
    // Get agent's workdir, current pane, and metadata.
    let agent_info = match sql_rows(
        cli,
        &format!(
            "SELECT biome_pane_id, workdir, default_task, metadata_json FROM agents WHERE name = '{}'",
            agent_name.replace('\'', "''")
        ),
    ) {
        Ok(rows) if !rows.is_empty() => rows.into_iter().next().unwrap(),
        Ok(_) => return ("failed".to_string(), format!("agent {agent_name} not found")),
        Err(err) => return ("failed".to_string(), format!("{err:#}")),
    };

    let old_pane_id = bsatn_unwrap(&agent_info[0]);
    let workdir = bsatn_unwrap(&agent_info[1]).unwrap_or_else(|| "~".to_string());
    let default_task = bsatn_unwrap_or(&agent_info[2], "Continue the current task.");
    let metadata_str = bsatn_unwrap_or(&agent_info[3], "{}");

    // Read backend and domain from metadata.
    let metadata_val = serde_json::from_str::<Value>(&metadata_str).unwrap_or(json!({}));
    let backend = metadata_val
        .get("backend")
        .and_then(|b| b.as_str())
        .unwrap_or("claude")
        .to_string();
    let domain_from_meta = metadata_val
        .get("domain")
        .and_then(|d| d.as_str())
        .map(str::to_string);

    // Resolve group: explicit override > domain from metadata.
    let group = group_override
        .map(str::to_string)
        .or(domain_from_meta);

    // Extract task from payload, fall back to default_task.
    let task = serde_json::from_str::<Value>(payload_json_str)
        .ok()
        .and_then(|v| v.get("task").and_then(|t| t.as_str()).map(str::to_string))
        .unwrap_or(default_task);

    // Delete old pane.
    if let Some(old_pane) = old_pane_id.as_deref() {
        let _ = biome_delete_pane(cli, old_pane);
    }

    // Create new pane.
    let new_pane_id = match biome_create_pane(cli, agent_name, group.as_deref()) {
        Ok(id) => id,
        Err(err) => return ("failed".to_string(), format!("{err:#}")),
    };

    // Boot using backend from metadata.
    let boot = boot_command_for_backend(&backend, &workdir);
    if let Err(err) = biome_send_text(cli, &new_pane_id, &boot) {
        return ("failed".to_string(), format!("boot send failed: {err:#}"));
    }

    // Send task
    if let Err(err) = biome_send_text(cli, &new_pane_id, &task) {
        return ("failed".to_string(), format!("task send failed: {err:#}"));
    }

    // Update agent's pane ID in DB
    let _ = call_reducer_silent(
        cli,
        "agent_update_pane_id",
        Some(vec![
            Value::String(agent_name.to_string()),
            Value::String(new_pane_id.clone()),
        ]),
    );

    (
        "done".to_string(),
        format!("restarted ({backend}) with new biome pane {new_pane_id}"),
    )
}

fn execute_spawn_agent(cli: &CliContext, payload_json_str: &str, group_override: Option<&str>) -> (String, String) {
    let payload: Value = match serde_json::from_str(payload_json_str) {
        Ok(v) => v,
        Err(err) => return ("failed".to_string(), format!("bad spawn payload: {err}")),
    };

    let name = match payload.get("name").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => return ("failed".to_string(), "spawn_agent payload missing 'name'".to_string()),
    };
    let workdir = payload
        .get("workdir")
        .and_then(|v| v.as_str())
        .unwrap_or("~")
        .to_string();
    let default_task = payload
        .get("default_task")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let backend = payload
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or("claude");
    let metadata = payload
        .get("metadata")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());
    let task = payload
        .get("task")
        .and_then(|v| v.as_str())
        .unwrap_or("Continue the current task.");

    let domain_from_meta = payload
        .get("metadata")
        .and_then(|v| v.get("domain"))
        .and_then(|d| d.as_str())
        .map(str::to_string);
    let group = group_override
        .map(str::to_string)
        .or(domain_from_meta);

    let pane_id = match biome_create_pane(cli, &name, group.as_deref()) {
        Ok(id) => id,
        Err(err) => return ("failed".to_string(), format!("pane create failed: {err:#}")),
    };

    if let Err(err) = call_reducer_silent(
        cli,
        "agent_add",
        Some(vec![
            Value::String(name.clone()),
            Value::String(pane_id.clone()),
            optional_json_string(Some(workdir.clone())),
            optional_json_string(default_task),
            optional_json_string(None),
            optional_json_string(Some(metadata)),
        ]),
    ) {
        return ("failed".to_string(), format!("agent_add failed: {err:#}"));
    }

    let boot = boot_command_for_backend(backend, &workdir);
    if let Err(err) = biome_send_text(cli, &pane_id, &boot) {
        return ("failed".to_string(), format!("boot send failed: {err:#}"));
    }

    if let Err(err) = biome_send_text(cli, &pane_id, task) {
        return ("failed".to_string(), format!("task send failed: {err:#}"));
    }

    (
        "done".to_string(),
        format!("spawned agent {name} ({backend}) in pane {pane_id}"),
    )
}

fn execute_restart_service(payload_json_str: &str) -> (String, String) {
    let payload: Value = match serde_json::from_str(payload_json_str) {
        Ok(v) => v,
        Err(err) => return ("failed".to_string(), format!("bad payload: {err}")),
    };

    let svc_name = payload.get("service_name").and_then(|v| v.as_str()).unwrap_or("");
    let svc_type = payload.get("service_type").and_then(|v| v.as_str()).unwrap_or("");
    let host = payload.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
    let check_target = payload.get("check_target").and_then(|v| v.as_str()).unwrap_or("");
    let custom_cmd = payload.get("restart_command").and_then(|v| v.as_str());

    let result = if let Some(cmd) = custom_cmd {
        Command::new("sh")
            .args(["-c", cmd])
            .output()
    } else {
        match svc_type {
            "systemd" => Command::new("systemctl")
                .args(["restart", check_target])
                .output(),
            "ssh_systemd" => Command::new("ssh")
                .args([
                    "-o", "ConnectTimeout=10",
                    host,
                    "systemctl", "restart", check_target,
                ])
                .output(),
            _ => return ("failed".to_string(), format!("no restart method for service type: {svc_type}")),
        }
    };

    match result {
        Ok(output) if output.status.success() => {
            ("done".to_string(), format!("restarted service {svc_name}"))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            ("failed".to_string(), format!("restart failed: {stderr}"))
        }
        Err(err) => ("failed".to_string(), format!("restart error: {err}")),
    }
}

fn get_agent_pane_id(cli: &CliContext, agent_name: &str) -> Result<Option<String>> {
    let rows = sql_rows(
        cli,
        &format!(
            "SELECT biome_pane_id FROM agents WHERE name = '{}'",
            agent_name.replace('\'', "''")
        ),
    )?;
    Ok(rows
        .first()
        .and_then(|row| row.first())
        .and_then(|v| bsatn_unwrap(v))
        .filter(|s| !s.is_empty()))
}

// ── Core CLI infrastructure ─────────────────────────────────────────────

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let _ = writeln!(&mut std::io::stderr().lock(), "{err:#}");
            ExitCode::from(1)
        }
    }
}

/// Load harness.toml and set any values as env vars (if not already set),
/// so clap's `env = "..."` picks them up.
#[cfg(feature = "cli")]
fn load_toml_config() {
    use std::collections::HashMap;

    // Search: ./harness.toml, then ~/.config/harness/harness.toml
    let candidates = [
        std::path::PathBuf::from("harness.toml"),
        dirs_next().map(|d| d.join("harness").join("harness.toml")).unwrap_or_default(),
    ];
    let path = candidates.iter().find(|p| p.exists());
    let Some(path) = path else { return };
    let Ok(contents) = std::fs::read_to_string(path) else { return };
    let Ok(table) = contents.parse::<toml::Table>() else {
        let _ = writeln!(
            &mut io::stderr().lock(),
            "warning: failed to parse {}",
            path.display()
        );
        return;
    };

    // Map toml keys to env var names: snake_case -> HARNESS_UPPER_SNAKE
    let mapping: HashMap<&str, &str> = HashMap::from([
        ("database", "HARNESS_DATABASE"),
        ("server", "HARNESS_SERVER"),
        ("biome_url", "HARNESS_BIOME_URL"),
        ("biome_api_key", "HARNESS_BIOME_API_KEY"),
    ]);

    for (toml_key, env_key) in &mapping {
        if std::env::var(env_key).is_ok() {
            continue; // env/CLI already set, don't override
        }
        if let Some(val) = table.get(*toml_key).and_then(|v| v.as_str()) {
            // SAFETY: called before any threads are spawned (single-threaded init)
            unsafe { std::env::set_var(env_key, val); }
        }
    }
}

#[cfg(feature = "cli")]
fn dirs_next() -> Option<std::path::PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
}

fn run() -> Result<()> {
    #[cfg(feature = "cli")]
    {
        dotenvy::dotenv().ok();
        load_toml_config();
    }
    let cli = Cli::parse();
    let biome_url = cli.biome_url.trim_end_matches('/').to_string();
    let context = CliContext {
        database: cli.database,
        server: cli.server,
        biome_url,
        biome_api_key: cli.biome_api_key,
        client: Client::new(),
    };
    match cli.command {
        Commands::Build => run_status(Command::new("spacetime").args(["build", "-p", "harness-rs"])),
        Commands::SeedAgents => call_reducer(&context, "seed_agents", None),
        Commands::BootstrapKnownGoals => call_reducer(&context, "bootstrap_known_goals", None),
        Commands::Agents => sql(&context, "SELECT * FROM agents"),
        Commands::Goals => sql(&context, "SELECT * FROM goals"),
        Commands::SubGoals => sql(&context, "SELECT * FROM sub_goals"),
        Commands::Facts => sql(
            &context,
            "SELECT fact_key, value_json, confidence, source_type, source_ref, updated_at FROM facts",
        ),
        Commands::Summary => {
            sql(&context, "SELECT name, status, current_goal_key, current_sub_goal_key, last_seen_at, last_capture_preview FROM agents")?;
            sql(&context, "SELECT goal_key, status, priority, success_fact_key FROM goals")?;
            sql(&context, "SELECT sub_goal_key, goal_key, owner_agent, status, priority FROM sub_goals")?;
            sql(&context, "SELECT name, service_type, host, status, last_checked_at, consecutive_failures FROM services")?;
            sql(&context, "SELECT id, agent_name, action_type, status, reason FROM actions LIMIT 10")
        }
        Commands::DecideActions => call_reducer(&context, "decide_actions", None),
        Commands::ResolveActiveSubGoals => call_reducer(&context, "resolve_active_sub_goals", None),
        Commands::PollBiome(args) => cmd_poll_biome(&context, args.lines, args.domain.as_deref()),
        Commands::ExecuteBiome(args) => {
            let effective_group = args.group.as_deref().or(args.domain.as_deref());
            cmd_execute_biome(&context, args.limit, args.domain.as_deref(), effective_group)
        }
        Commands::RunOnceBiome(args) => {
            cmd_run_once_biome(&context, args.lines, args.execute, args.limit, args.domain.as_deref(), args.group.as_deref())
        }
        Commands::QueuePrompt(args) => call_reducer(
            &context,
            "queue_prompt",
            Some(vec![Value::String(args.agent_name), Value::String(args.text)]),
        ),
        Commands::FactSet(args) => call_reducer(
            &context,
            "fact_set",
            Some(vec![json!({
                "fact_key": args.key,
                "value_json": args.value,
                "confidence": some_json_f64(args.confidence),
                "source_type": some_json_string(args.source_type),
                "source_ref": optional_json_string(args.source_ref),
                "metadata_json": some_json_string(args.metadata.unwrap_or_else(|| "{}".to_string()))
            })]),
        ),
        Commands::GoalAdd(args) => call_reducer(
            &context,
            "goal_add",
            Some(vec![json!({
                "goal_key": args.goal_key,
                "title": args.title,
                "detail": some_json_string(args.detail),
                "status": some_json_string(args.status),
                "priority": some_json_u32(args.priority),
                "depends_on_goal_key": optional_json_string(args.depends_on_goal_key),
                "success_fact_key": optional_json_string(args.success_fact_key),
                "metadata_json": some_json_string(args.metadata.unwrap_or_else(|| "{}".to_string())),
                "completion_report": null
            })]),
        ),
        Commands::GoalUpdate(args) => call_reducer(
            &context,
            "goal_update",
            Some(vec![
                Value::String(args.goal_key),
                json!({
                    "title": optional_json_string(args.title),
                    "detail": optional_json_string(args.detail),
                    "status": optional_json_string(args.status),
                    "priority": optional_json_u32(args.priority),
                    "depends_on_goal_key": optional_json_string(args.depends_on_goal_key),
                    "success_fact_key": optional_json_string(args.success_fact_key),
                    "metadata_json": none_json(),
                    "completion_report": none_json(),
                    "clear_depends": args.clear_depends,
                    "clear_success_fact": args.clear_success_fact
                }),
            ]),
        ),
        Commands::GoalRemove(args) => call_reducer(
            &context,
            "goal_remove",
            Some(vec![
                Value::String(args.goal_key),
                Value::Bool(args.delete),
                Value::Bool(args.cascade),
            ]),
        ),
        Commands::GoalSet(args) => call_reducer(
            &context,
            "goal_update",
            Some(vec![
                Value::String(args.goal_key),
                json!({
                    "title": none_json(),
                    "detail": none_json(),
                    "status": some_json_string(args.status),
                    "priority": none_json(),
                    "depends_on_goal_key": none_json(),
                    "success_fact_key": none_json(),
                    "metadata_json": none_json(),
                    "completion_report": none_json(),
                    "clear_depends": false,
                    "clear_success_fact": false
                }),
            ]),
        ),
        Commands::SubGoalAdd(args) => call_reducer(
            &context,
            "sub_goal_add",
            Some(vec![json!({
                "sub_goal_key": args.sub_goal_key,
                "goal_key": args.goal_key,
                "owner_agent": args.owner_agent,
                "title": args.title,
                "detail": some_json_string(args.detail),
                "status": some_json_string(args.status),
                "priority": some_json_u32(args.priority),
                "depends_on_sub_goal_key": optional_json_string(args.depends_on_sub_goal_key),
                "blocked_by": none_json(),
                "success_fact_key": optional_json_string(args.success_fact_key),
                "instruction_text": optional_json_string(args.instruction_text),
                "stuck_guidance_text": optional_json_string(args.stuck_guidance_text),
                "metadata_json": some_json_string(args.metadata.unwrap_or_else(|| "{}".to_string())),
                "completion_report": null
            })]),
        ),
        Commands::SubGoalUpdate(args) => call_reducer(
            &context,
            "sub_goal_update",
            Some(vec![
                Value::String(args.sub_goal_key),
                json!({
                    "goal_key": optional_json_string(args.goal_key),
                    "owner_agent": optional_json_string(args.owner_agent),
                    "title": optional_json_string(args.title),
                    "detail": optional_json_string(args.detail),
                    "status": optional_json_string(args.status),
                    "priority": optional_json_u32(args.priority),
                    "depends_on_sub_goal_key": optional_json_string(args.depends_on_sub_goal_key),
                    "success_fact_key": optional_json_string(args.success_fact_key),
                    "instruction_text": optional_json_string(args.instruction_text),
                    "stuck_guidance_text": optional_json_string(args.stuck_guidance_text),
                    "metadata_json": none_json(),
                    "blocked_by": none_json(),
                    "completion_report": none_json(),
                    "clear_depends": args.clear_depends,
                    "clear_success_fact": args.clear_success_fact,
                    "clear_instruction": args.clear_instruction,
                    "clear_stuck_guidance": args.clear_stuck_guidance,
                    "clear_blocked_by": false
                }),
            ]),
        ),
        Commands::SubGoalRemove(args) => call_reducer(
            &context,
            "sub_goal_remove",
            Some(vec![Value::String(args.sub_goal_key), Value::Bool(args.delete)]),
        ),
        Commands::SubGoalSet(args) => call_reducer(
            &context,
            "sub_goal_update",
            Some(vec![
                Value::String(args.sub_goal_key),
                json!({
                    "goal_key": none_json(),
                    "owner_agent": none_json(),
                    "title": none_json(),
                    "detail": none_json(),
                    "status": some_json_string(args.status),
                    "priority": none_json(),
                    "depends_on_sub_goal_key": none_json(),
                    "success_fact_key": none_json(),
                    "instruction_text": none_json(),
                    "stuck_guidance_text": none_json(),
                    "metadata_json": none_json(),
                    "blocked_by": none_json(),
                    "completion_report": none_json(),
                    "clear_depends": false,
                    "clear_success_fact": false,
                    "clear_instruction": false,
                    "clear_stuck_guidance": false,
                    "clear_blocked_by": false
                }),
            ]),
        ),
        Commands::AgentAdd(args) => call_reducer(
            &context,
            "agent_add",
            Some(vec![
                Value::String(args.name),
                Value::String(args.biome_pane_id),
                optional_json_string(args.workdir),
                optional_json_string(args.default_task),
                optional_json_string(args.tmux_target),
                optional_json_string(args.metadata),
            ]),
        ),
        Commands::AgentCreate(args) => {
            let pane_id = biome_create_pane(&context, &args.name, None)?;
            let _ = writeln!(
                &mut io::stderr().lock(),
                "created pane {pane_id} (name: {})",
                args.name
            );

            if let Some(ref cmd) = args.cmd {
                biome_send_text_delayed(&context, &pane_id, cmd, 150)?;
                let _ = writeln!(
                    &mut io::stderr().lock(),
                    "sent cmd, waiting {}s for init...",
                    args.init_wait
                );
                std::thread::sleep(std::time::Duration::from_secs(args.init_wait));
            }

            if let Some(ref task) = args.task {
                biome_send_text_delayed(&context, &pane_id, task, 300)?;
                let _ = writeln!(&mut io::stderr().lock(), "sent task prompt");
            }

            call_reducer(
                &context,
                "agent_add",
                Some(vec![
                    Value::String(args.name.clone()),
                    Value::String(pane_id.clone()),
                    optional_json_string(args.workdir),
                    optional_json_string(args.default_task),
                    optional_json_string(None),
                    Value::Null,
                ]),
            )?;
            out!("{pane_id}");
            Ok(())
        }
        Commands::AgentRemove(args) => call_reducer(
            &context,
            "agent_remove",
            Some(vec![Value::String(args.name), Value::Bool(args.delete)]),
        ),
        Commands::Send(args) => {
            let pane_id = biome_resolve_pane(&context, &args.pane)?;
            biome_send_text_delayed(&context, &pane_id, &args.text, args.delay)?;
            out!("sent to {pane_id}");
            Ok(())
        }
        Commands::Checkpoint(args) => {
            let patch = json!({
                "goal_key": none_json(),
                "owner_agent": none_json(),
                "title": none_json(),
                "detail": none_json(),
                "status": optional_json_string(args.status),
                "priority": none_json(),
                "depends_on_sub_goal_key": none_json(),
                "success_fact_key": none_json(),
                "instruction_text": none_json(),
                "stuck_guidance_text": none_json(),
                "metadata_json": none_json(),
                "blocked_by": none_json(),
                "completion_report": some_json_string(args.report),
                "clear_depends": false,
                "clear_success_fact": false,
                "clear_instruction": false,
                "clear_stuck_guidance": false,
                "clear_blocked_by": false
            });
            call_reducer(
                &context,
                "sub_goal_update",
                Some(vec![Value::String(args.sub_goal_key), patch]),
            )?;
            for fact_str in &args.facts {
                if let Some((key, value)) = fact_str.split_once('=') {
                    call_reducer_silent(
                        &context,
                        "fact_set",
                        Some(vec![json!({
                            "fact_key": key,
                            "value_json": value,
                            "confidence": some_json_f64(1.0),
                            "source_type": some_json_string("checkpoint".to_string()),
                            "source_ref": none_json(),
                            "metadata_json": some_json_string("{}".to_string())
                        })]),
                    )?;
                }
            }
            Ok(())
        }
        Commands::Screen(args) => {
            let pane_id = biome_resolve_pane(&context, &args.pane)?;
            let lines = if args.lines == 0 { 9999 } else { args.lines };
            let output = biome_screen(&context, &pane_id, lines)?;
            out!("{output}");
            Ok(())
        }
        Commands::Heartbeat(args) => {
            let timestamp = chrono::Utc::now().to_rfc3339();
            call_reducer_silent(
                &context,
                "fact_set",
                Some(vec![json!({
                    "fact_key": format!("{}.supervisor.status", args.domain),
                    "value_json": format!("\"{}\"", args.status),
                    "confidence": some_json_f64(1.0),
                    "source_type": some_json_string("heartbeat".to_string()),
                    "source_ref": some_json_string(format!("domain:{}", args.domain)),
                    "metadata_json": some_json_string(format!("{{\"domain\":\"{}\"}}", args.domain))
                })]),
            )?;
            call_reducer_silent(
                &context,
                "fact_set",
                Some(vec![json!({
                    "fact_key": format!("{}.supervisor.last_heartbeat", args.domain),
                    "value_json": format!("\"{}\"", timestamp),
                    "confidence": some_json_f64(1.0),
                    "source_type": some_json_string("heartbeat".to_string()),
                    "source_ref": some_json_string(format!("domain:{}", args.domain)),
                    "metadata_json": some_json_string(format!("{{\"domain\":\"{}\"}}", args.domain))
                })]),
            )?;
            out!("heartbeat: {}.supervisor.status={}, last_heartbeat={}", args.domain, args.status, timestamp);
            Ok(())
        }
        Commands::EpisodeAdd(args) => call_reducer(
            &context,
            "episode_add",
            Some(vec![json!({
                "summary": args.summary,
                "agent_statuses_json": args.agent_statuses,
                "actions_taken_json": args.actions_taken,
                "goal_progress_json": args.goal_progress
            })]),
        ),
        Commands::Episodes(args) => sql(
            &context,
            &format!(
                "SELECT id, created_at, summary, agent_statuses_json, actions_taken_json, goal_progress_json FROM episodes LIMIT {}",
                args.limit
            ),
        ),
        Commands::AgentDescribe(args) => call_reducer(
            &context,
            "agent_update_description",
            Some(vec![
                Value::String(args.name),
                Value::String(args.description),
            ]),
        ),
        Commands::Services => sql(
            &context,
            "SELECT name, service_type, host, status, last_checked_at, consecutive_failures FROM services",
        ),
        Commands::ServiceAdd(args) => call_reducer(
            &context,
            "service_add",
            Some(vec![json!({
                "name": args.name,
                "service_type": args.service_type,
                "host": some_json_string(args.host),
                "check_target": args.check_target,
                "restart_policy": some_json_string(args.restart_policy),
                "restart_command": optional_json_string(args.restart_command),
                "metadata_json": some_json_string(args.metadata.unwrap_or_else(|| "{}".to_string())),
            })]),
        ),
        Commands::ServiceRemove(args) => call_reducer(
            &context,
            "service_remove",
            Some(vec![Value::String(args.name), Value::Bool(args.delete)]),
        ),
        Commands::PollServices(args) => cmd_poll_services(&context, args.timeout_ms),
        Commands::Panes => cmd_panes(&context),
        Commands::Dump => cmd_dump(&context),
        Commands::Restore => cmd_restore(&context),
    }
}

fn cmd_dump(cli: &CliContext) -> Result<()> {
    let rows = sql_rows(
        cli,
        "SELECT name, biome_pane_id, workdir, default_task, tmux_target, metadata_json FROM agents",
    )?;
    for row in &rows {
        let obj = json!({
            "type": "agent",
            "name": bsatn_unwrap_or(&row[0], ""),
            "biome_pane_id": bsatn_unwrap(&row[1]).unwrap_or_default(),
            "workdir": bsatn_unwrap(&row[2]),
            "default_task": bsatn_unwrap(&row[3]),
            "tmux_target": bsatn_unwrap(&row[4]),
            "metadata_json": bsatn_unwrap_or(&row[5], "{}"),
        });
        out!("{}", serde_json::to_string(&obj)?);
    }

    let rows = sql_rows(cli, "SELECT goal_key, title, detail, status, priority, depends_on_goal_key, success_fact_key, metadata_json FROM goals")?;
    for row in &rows {
        let obj = json!({
            "type": "goal",
            "goal_key": bsatn_unwrap_or(&row[0], ""),
            "title": bsatn_unwrap_or(&row[1], ""),
            "detail": bsatn_unwrap(&row[2]).unwrap_or_default(),
            "status": bsatn_unwrap_or(&row[3], "active"),
            "priority": row[4].as_u64().unwrap_or(50),
            "depends_on_goal_key": bsatn_unwrap(&row[5]),
            "success_fact_key": bsatn_unwrap(&row[6]),
            "metadata_json": bsatn_unwrap_or(&row[7], "{}"),
        });
        out!("{}", serde_json::to_string(&obj)?);
    }

    let rows = sql_rows(cli, "SELECT sub_goal_key, goal_key, owner_agent, title, detail, status, priority, depends_on_sub_goal_key, blocked_by, success_fact_key, instruction_text, stuck_guidance_text, metadata_json FROM sub_goals")?;
    for row in &rows {
        let obj = json!({
            "type": "sub_goal",
            "sub_goal_key": bsatn_unwrap_or(&row[0], ""),
            "goal_key": bsatn_unwrap_or(&row[1], ""),
            "owner_agent": bsatn_unwrap_or(&row[2], ""),
            "title": bsatn_unwrap_or(&row[3], ""),
            "detail": bsatn_unwrap(&row[4]).unwrap_or_default(),
            "status": bsatn_unwrap_or(&row[5], "pending"),
            "priority": row[6].as_u64().unwrap_or(50),
            "depends_on_sub_goal_key": bsatn_unwrap(&row[7]),
            "blocked_by": bsatn_unwrap(&row[8]),
            "success_fact_key": bsatn_unwrap(&row[9]),
            "instruction_text": bsatn_unwrap(&row[10]),
            "stuck_guidance_text": bsatn_unwrap(&row[11]),
            "metadata_json": bsatn_unwrap_or(&row[12], "{}"),
        });
        out!("{}", serde_json::to_string(&obj)?);
    }

    let rows = sql_rows(
        cli,
        "SELECT fact_key, value_json, confidence, source_type, source_ref, metadata_json FROM facts",
    )?;
    for row in &rows {
        let obj = json!({
            "type": "fact",
            "fact_key": bsatn_unwrap_or(&row[0], ""),
            "value_json": bsatn_unwrap_or(&row[1], ""),
            "confidence": row[2].as_f64().unwrap_or(1.0),
            "source_type": bsatn_unwrap_or(&row[3], "manual"),
            "source_ref": bsatn_unwrap(&row[4]),
            "metadata_json": bsatn_unwrap_or(&row[5], "{}"),
        });
        out!("{}", serde_json::to_string(&obj)?);
    }

    Ok(())
}

fn cmd_restore(cli: &CliContext) -> Result<()> {
    let stdin = io::stdin();
    let mut goal_count = 0u32;
    let mut sub_goal_count = 0u32;
    let mut fact_count = 0u32;

    for line in stdin.lock().lines() {
        let line = line.context("reading stdin")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let obj: Value = serde_json::from_str(line)
            .with_context(|| format!("parsing JSONL line: {}", &line[..line.len().min(80)]))?;
        let record_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match record_type {
            "agent" => {
                call_reducer_silent(cli, "agent_add", Some(vec![
                    Value::String(obj["name"].as_str().unwrap_or("").to_string()),
                    Value::String(obj["biome_pane_id"].as_str().unwrap_or("placeholder").to_string()),
                    optional_json_string(obj["workdir"].as_str().map(str::to_string)),
                    optional_json_string(obj["default_task"].as_str().map(str::to_string)),
                    optional_json_string(obj["tmux_target"].as_str().map(str::to_string)),
                    optional_json_string(obj["metadata_json"].as_str().map(str::to_string)),
                ]))?;
            }
            "goal" => {
                call_reducer_silent(cli, "goal_add", Some(vec![json!({
                    "goal_key": obj["goal_key"],
                    "title": obj["title"],
                    "detail": some_json_string(obj["detail"].as_str().unwrap_or("").to_string()),
                    "status": some_json_string(obj["status"].as_str().unwrap_or("active").to_string()),
                    "priority": some_json_u32(obj["priority"].as_u64().unwrap_or(50) as u32),
                    "depends_on_goal_key": optional_json_string(obj["depends_on_goal_key"].as_str().map(str::to_string)),
                    "success_fact_key": optional_json_string(obj["success_fact_key"].as_str().map(str::to_string)),
                    "metadata_json": some_json_string(obj["metadata_json"].as_str().unwrap_or("{}").to_string()),
                    "completion_report": null
                })]))?;
                goal_count += 1;
            }
            "sub_goal" => {
                call_reducer_silent(cli, "sub_goal_add", Some(vec![json!({
                    "sub_goal_key": obj["sub_goal_key"],
                    "goal_key": obj["goal_key"],
                    "owner_agent": obj["owner_agent"],
                    "title": obj["title"],
                    "detail": some_json_string(obj["detail"].as_str().unwrap_or("").to_string()),
                    "status": some_json_string(obj["status"].as_str().unwrap_or("pending").to_string()),
                    "priority": some_json_u32(obj["priority"].as_u64().unwrap_or(50) as u32),
                    "depends_on_sub_goal_key": optional_json_string(obj["depends_on_sub_goal_key"].as_str().map(str::to_string)),
                    "blocked_by": optional_json_string(obj["blocked_by"].as_str().map(str::to_string)),
                    "success_fact_key": optional_json_string(obj["success_fact_key"].as_str().map(str::to_string)),
                    "instruction_text": optional_json_string(obj["instruction_text"].as_str().map(str::to_string)),
                    "stuck_guidance_text": optional_json_string(obj["stuck_guidance_text"].as_str().map(str::to_string)),
                    "metadata_json": some_json_string(obj["metadata_json"].as_str().unwrap_or("{}").to_string()),
                    "completion_report": null
                })]))?;
                sub_goal_count += 1;
            }
            "fact" => {
                call_reducer_silent(cli, "fact_set", Some(vec![json!({
                    "fact_key": obj["fact_key"],
                    "value_json": obj["value_json"],
                    "confidence": some_json_f64(obj["confidence"].as_f64().unwrap_or(1.0)),
                    "source_type": some_json_string(obj["source_type"].as_str().unwrap_or("manual").to_string()),
                    "source_ref": optional_json_string(obj["source_ref"].as_str().map(str::to_string)),
                    "metadata_json": some_json_string(obj["metadata_json"].as_str().unwrap_or("{}").to_string())
                })]))?;
                fact_count += 1;
            }
            _ => {
                let _ = writeln!(&mut io::stderr().lock(), "skipping unknown type: {record_type}");
            }
        }
    }

    let _ = writeln!(
        &mut io::stderr().lock(),
        "restored {goal_count} goals, {sub_goal_count} sub-goals, {fact_count} facts"
    );
    Ok(())
}

fn call_reducer(cli: &CliContext, reducer: &str, args: Option<Vec<Value>>) -> Result<()> {
    run_call(cli, reducer, args.unwrap_or_default(), true)
}

fn call_reducer_silent(cli: &CliContext, reducer: &str, args: Option<Vec<Value>>) -> Result<()> {
    run_call(cli, reducer, args.unwrap_or_default(), false)
}

fn run_call(cli: &CliContext, name: &str, args: Vec<Value>, print: bool) -> Result<()> {
    let url = format!("{}/v1/database/{}/call/{}", cli.server, cli.database, name);
    let body = Value::Array(args).to_string();
    let resp = cli
        .client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .with_context(|| format!("failed to call reducer {name}"))?;

    let status = resp.status();
    let text = resp.text().unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!("call {name} failed (HTTP {status}): {text}"));
    }

    if print && !text.is_empty() {
        if let Ok(json) = serde_json::from_str::<Value>(&text) {
            out!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            out!("{text}");
        }
    }
    Ok(())
}

fn sql(cli: &CliContext, query: &str) -> Result<()> {
    let url = format!("{}/v1/database/{}/sql", cli.server, cli.database);
    let resp = cli
        .client
        .post(&url)
        .body(query.to_string())
        .send()
        .with_context(|| format!("failed to run SQL: {query}"))?;

    let status = resp.status();
    let text = resp.text().unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!("SQL failed (HTTP {status}): {text}"));
    }

    if text.is_empty() {
        return Ok(());
    }

    if let Ok(json) = serde_json::from_str::<Value>(&text) {
        print_sql_result(&json);
    } else {
        out!("{text}");
    }
    Ok(())
}

fn print_sql_result(json: &Value) {
    let empty = vec![];
    let results = json.as_array().unwrap_or(&empty);
    for result in results {
        let schema = result
            .get("schema")
            .and_then(|s| s.get("elements"))
            .and_then(|e| e.as_array());
        let rows = result.get("rows").and_then(|r| r.as_array());

        match (schema, rows) {
            (Some(schema), Some(rows)) => {
                let columns: Vec<String> = schema
                    .iter()
                    .filter_map(|e| {
                        let name = e.get("name")?;
                        // SpacetimeDB wraps column names as {"some": "col_name"}
                        if let Some(s) = name.as_str() {
                            Some(s.to_string())
                        } else {
                            name.get("some").and_then(|s| s.as_str()).map(|s| s.to_string())
                        }
                    })
                    .collect();

                if columns.is_empty() {
                    continue;
                }
                if rows.is_empty() {
                    out!("(0 rows)");
                    continue;
                }

                let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();
                // Cap column widths to keep output readable
                let max_width: usize = 60;
                let string_rows: Vec<Vec<String>> = rows
                    .iter()
                    .filter_map(|row| row.as_array())
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .map(|(i, val)| {
                                let s = format_cell(val);
                                if i < widths.len() {
                                    widths[i] = widths[i].max(s.len());
                                }
                                s
                            })
                            .collect()
                    })
                    .collect();

                for w in &mut widths {
                    *w = (*w).min(max_width);
                }

                // header
                let header: Vec<String> = columns
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let w = widths.get(i).copied().unwrap_or(0);
                        format!("{:w$}", &c[..c.len().min(w)], w = w)
                    })
                    .collect();
                out!(" {}", header.join(" | "));

                // separator
                let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
                out!("-{}-", sep.join("-+-"));

                // rows
                for row in &string_rows {
                    let cells: Vec<String> = row
                        .iter()
                        .enumerate()
                        .map(|(i, val)| {
                            format!("{:w$}", val, w = widths.get(i).copied().unwrap_or(0))
                        })
                        .collect();
                    out!(" {}", cells.join(" | "));
                }
                out!("({} rows)", string_rows.len());
                out!();
            }
            _ => {
                if let Ok(pretty) = serde_json::to_string_pretty(result) {
                    out!("{pretty}");
                }
            }
        }
    }
}

fn format_cell(val: &Value) -> String {
    match val {
        Value::Null => "NULL".to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        // BSATN sum-type: [0, value] = Some(value), [1, []] = None
        Value::Array(arr) if arr.len() == 2 => match arr[0].as_u64() {
            Some(0) => format_cell(&arr[1]),
            Some(1) => "(none)".to_string(),
            _ => val.to_string(),
        },
        _ => val.to_string(),
    }
}

// Kept for Build command only
fn run_status(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to run {:?}", command))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("command exited with status {status}"))
    }
}

fn optional_json_string(value: Option<String>) -> Value {
    match value {
        Some(value) => some_json_string(value),
        None => none_json(),
    }
}

fn optional_json_u32(value: Option<u32>) -> Value {
    match value {
        Some(value) => some_json_u32(value),
        None => none_json(),
    }
}

fn some_json_string(value: String) -> Value {
    json!({ "some": value })
}

fn some_json_u32(value: u32) -> Value {
    json!({ "some": value })
}

fn some_json_f64(value: f64) -> Value {
    json!({ "some": value })
}

fn none_json() -> Value {
    json!({ "none": [] })
}
