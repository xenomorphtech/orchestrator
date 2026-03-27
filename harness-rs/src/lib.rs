use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use spacetimedb::{
    http::{Body, Request, Timeout},
    ProcedureContext, ReducerContext, SpacetimeType, Table, TimeDuration,
};

const DEFAULT_PROJECT_DIR: &str = "/home/sdancer/games/nmss";
const DEFAULT_TMUX_SESSION: &str = "nmss";
const DEFAULT_BIOME_TERM_URL: &str = "http://localhost:3000";
const POLL_SCROLLBACK_LINES: usize = 20;
const REPEAT_STUCK_SECONDS: i64 = 600;
const BIOME_HTTP_TIMEOUT_MS: u64 = 500;
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

#[derive(Clone, Debug, SpacetimeType)]
pub struct AgentInput {
    pub name: String,
    pub tmux_target: String,
    pub workdir: Option<String>,
    pub default_task: String,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct GoalInput {
    pub goal_key: String,
    pub title: String,
    pub detail: Option<String>,
    pub status: Option<String>,
    pub priority: Option<u32>,
    pub depends_on_goal_key: Option<String>,
    pub success_fact_key: Option<String>,
    pub metadata_json: Option<String>,
    pub completion_report: Option<String>,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct GoalPatch {
    pub title: Option<String>,
    pub detail: Option<String>,
    pub status: Option<String>,
    pub priority: Option<u32>,
    pub depends_on_goal_key: Option<String>,
    pub success_fact_key: Option<String>,
    pub metadata_json: Option<String>,
    pub completion_report: Option<String>,
    pub clear_depends: bool,
    pub clear_success_fact: bool,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct SubGoalInput {
    pub sub_goal_key: String,
    pub goal_key: String,
    pub owner_agent: String,
    pub title: String,
    pub detail: Option<String>,
    pub status: Option<String>,
    pub priority: Option<u32>,
    pub depends_on_sub_goal_key: Option<String>,
    pub blocked_by: Option<String>,
    pub success_fact_key: Option<String>,
    pub instruction_text: Option<String>,
    pub stuck_guidance_text: Option<String>,
    pub metadata_json: Option<String>,
    pub completion_report: Option<String>,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct SubGoalPatch {
    pub goal_key: Option<String>,
    pub owner_agent: Option<String>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub status: Option<String>,
    pub priority: Option<u32>,
    pub depends_on_sub_goal_key: Option<String>,
    pub blocked_by: Option<String>,
    pub success_fact_key: Option<String>,
    pub instruction_text: Option<String>,
    pub stuck_guidance_text: Option<String>,
    pub metadata_json: Option<String>,
    pub completion_report: Option<String>,
    pub clear_depends: bool,
    pub clear_success_fact: bool,
    pub clear_instruction: bool,
    pub clear_stuck_guidance: bool,
    pub clear_blocked_by: bool,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct FactInput {
    pub fact_key: String,
    pub value_json: String,
    pub confidence: Option<f64>,
    pub source_type: Option<String>,
    pub source_ref: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct ArtifactInput {
    pub path: String,
    pub sha256: Option<String>,
    pub size_bytes: u64,
    pub mtime_ns: i128,
    pub significance: Option<String>,
    pub indexed_state: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct ActionInput {
    pub agent_name: Option<String>,
    pub action_type: String,
    pub payload_json: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct AgentPollInput {
    pub agent_name: String,
    pub status: String,
    pub last_capture_hash: Option<String>,
    pub last_capture_preview: Option<String>,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct PollSummary {
    pub agent_name: String,
    pub status: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, SpacetimeType)]
pub struct PendingActionSummary {
    pub action_id: u64,
    pub status: String,
    pub result_text: String,
}

#[derive(Clone)]
struct AgentRuntime {
    name: String,
    biome_pane_id: String,
    last_seen_at: Option<String>,
    last_capture_hash: Option<String>,
}

#[derive(Deserialize)]
struct BiomeScreen {
    rows: Vec<String>,
}

#[derive(Deserialize)]
struct BiomePaneCreated {
    id: String,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = agents, public)]
pub struct Agent {
    #[primary_key]
    pub name: String,
    pub tmux_target: String,
    pub workdir: Option<String>,
    pub default_task: String,
    pub status: String,
    pub current_goal_key: Option<String>,
    pub current_sub_goal_key: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_capture_hash: Option<String>,
    pub last_capture_preview: Option<String>,
    pub metadata_json: String,
    pub biome_pane_id: Option<String>,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = goals, public)]
pub struct Goal {
    #[primary_key]
    pub goal_key: String,
    pub title: String,
    pub detail: Option<String>,
    #[index(btree)]
    pub status: String,
    #[index(btree)]
    pub priority: u32,
    pub depends_on_goal_key: Option<String>,
    pub success_fact_key: Option<String>,
    pub metadata_json: String,
    pub completion_report: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = sub_goals, public)]
pub struct SubGoal {
    #[primary_key]
    pub sub_goal_key: String,
    #[index(btree)]
    pub goal_key: String,
    #[index(btree)]
    pub owner_agent: String,
    pub title: String,
    pub detail: Option<String>,
    #[index(btree)]
    pub status: String,
    #[index(btree)]
    pub priority: u32,
    pub depends_on_sub_goal_key: Option<String>,
    /// Comma-separated list of goal_keys or sub_goal_keys that must be "done" before this unblocks.
    pub blocked_by: Option<String>,
    pub success_fact_key: Option<String>,
    pub instruction_text: Option<String>,
    pub stuck_guidance_text: Option<String>,
    pub metadata_json: String,
    pub completion_report: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = facts, public)]
pub struct Fact {
    #[primary_key]
    pub fact_key: String,
    pub value_json: String,
    pub confidence: f64,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub updated_at: String,
    pub metadata_json: String,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = observations, public)]
pub struct Observation {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub agent_name: String,
    pub kind: String,
    pub content: String,
    pub content_hash: String,
    pub created_at: String,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = artifacts, public)]
pub struct Artifact {
    #[primary_key]
    pub path: String,
    pub sha256: Option<String>,
    pub size_bytes: u64,
    pub mtime_ns: i128,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub significance: String,
    pub indexed_state: String,
    pub metadata_json: String,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = actions, public)]
pub struct Action {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub agent_name: Option<String>,
    #[index(btree)]
    pub action_type: String,
    pub payload_json: String,
    pub reason: Option<String>,
    #[index(btree)]
    pub status: String,
    pub created_at: String,
    pub executed_at: Option<String>,
    pub result_text: Option<String>,
}

#[derive(Clone)]
#[spacetimedb::table(accessor = events, public)]
pub struct Event {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub agent_name: Option<String>,
    #[index(btree)]
    pub event_type: String,
    pub message: String,
    pub payload_json: String,
    pub created_at: String,
}

fn now(ctx: &ReducerContext) -> String {
    ctx.timestamp.to_string()
}

fn json_or_empty(value: Option<String>) -> String {
    value.unwrap_or_else(|| "{}".to_string())
}

fn opt_text(value: Option<String>) -> Option<String> {
    value.and_then(|v| if v.is_empty() { None } else { Some(v) })
}

fn biome_base_url(base_url: Option<&str>) -> String {
    base_url.unwrap_or(DEFAULT_BIOME_TERM_URL).trim_end_matches('/').to_string()
}

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

fn classify_capture(text: Option<&str>, previous_hash: Option<&str>, previous_seen_at: Option<&str>) -> String {
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
                .map(|then| Utc::now().signed_duration_since(then.with_timezone(&Utc)).num_seconds() >= REPEAT_STUCK_SECONDS)
                .unwrap_or(false)
        {
            return "stuck".to_string();
        }
    }
    "working".to_string()
}

fn fact_is_true(ctx: &ReducerContext, fact_key: &Option<String>) -> bool {
    let Some(fact_key) = fact_key.as_ref() else {
        return false;
    };
    ctx.db
        .facts()
        .fact_key()
        .find(fact_key)
        .map(|fact| {
            let raw = fact.value_json.trim().to_ascii_lowercase();
            raw == "true" || raw == "\"true\""
        })
        .unwrap_or(false)
}

fn require_agent(ctx: &ReducerContext, agent_name: &str) -> Result<Agent, String> {
    ctx.db
        .agents()
        .name()
        .find(&agent_name.to_string())
        .ok_or_else(|| format!("unknown agent: {agent_name}"))
}

fn require_goal(ctx: &ReducerContext, goal_key: &str) -> Result<Goal, String> {
    ctx.db
        .goals()
        .goal_key()
        .find(&goal_key.to_string())
        .ok_or_else(|| format!("unknown goal: {goal_key}"))
}

fn require_sub_goal(ctx: &ReducerContext, sub_goal_key: &str) -> Result<SubGoal, String> {
    ctx.db
        .sub_goals()
        .sub_goal_key()
        .find(&sub_goal_key.to_string())
        .ok_or_else(|| format!("unknown sub-goal: {sub_goal_key}"))
}

fn log_event(ctx: &ReducerContext, agent_name: Option<String>, event_type: &str, message: String, payload_json: Option<String>) {
    ctx.db.events().insert(Event {
        id: 0,
        agent_name,
        event_type: event_type.to_string(),
        message,
        payload_json: payload_json.unwrap_or_else(|| "{}".to_string()),
        created_at: now(ctx),
    });
}

fn queue_action_internal(
    ctx: &ReducerContext,
    agent_name: Option<String>,
    action_type: String,
    payload_json: String,
    reason: Option<String>,
) {
    let duplicate = ctx.db.actions().iter().any(|row| {
        row.status == "pending"
            && row.action_type == action_type
            && row.agent_name == agent_name
            && row.payload_json == payload_json
    });
    if duplicate {
        return;
    }
    ctx.db.actions().insert(Action {
        id: 0,
        agent_name: agent_name.clone(),
        action_type: action_type.clone(),
        payload_json: payload_json.clone(),
        reason: opt_text(reason.clone()),
        status: "pending".to_string(),
        created_at: now(ctx),
        executed_at: None,
        result_text: None,
    });
    log_event(
        ctx,
        agent_name,
        "action.queued",
        format!("{}: {}", action_type, reason.unwrap_or_default()),
        Some(payload_json),
    );
}

fn current_sub_goal_row(ctx: &ReducerContext, agent_name: &str) -> Option<SubGoal> {
    let agent = ctx.db.agents().name().find(&agent_name.to_string())?;
    let sub_goal_key = agent.current_sub_goal_key?;
    ctx.db.sub_goals().sub_goal_key().find(&sub_goal_key)
}

fn fallback_instruction_for_sub_goal(ctx: &ReducerContext, sub_goal: &SubGoal) -> String {
    if sub_goal.sub_goal_key == "hybrid.capture_test"
        && fact_is_true(ctx, &Some("crypto.algorithm_identified".to_string()))
    {
        return "The capture script appears ready. Test it end-to-end now and record the captured session data."
            .to_string();
    }
    format!(
        "Continue sub-goal `{}` under goal `{}`. Title: {}. Detail: {}",
        sub_goal.sub_goal_key,
        sub_goal.goal_key,
        sub_goal.title,
        sub_goal.detail.clone().unwrap_or_else(|| "n/a".to_string())
    )
}

fn current_task_prompt(ctx: &ReducerContext, agent_name: &str) -> String {
    if let Some(sub_goal) = current_sub_goal_row(ctx, agent_name) {
        return sub_goal
            .instruction_text
            .clone()
            .unwrap_or_else(|| fallback_instruction_for_sub_goal(ctx, &sub_goal));
    }
    ctx.db
        .agents()
        .name()
        .find(&agent_name.to_string())
        .map(|agent| agent.default_task)
        .unwrap_or_else(|| "Continue the current task.".to_string())
}

fn corrective_prompt(ctx: &ReducerContext, agent_name: &str, preview: &str) -> String {
    let base = format!(
        "You look stuck. Recover from the last issue and keep moving. Current tail:\n\n{}\n",
        preview
    );
    if let Some(sub_goal) = current_sub_goal_row(ctx, agent_name) {
        if let Some(guidance) = sub_goal.stuck_guidance_text {
            return format!("{base}{guidance}");
        }
    }
    match agent_name {
        "crypto" => format!("{base}Try hooking sub_20bb48 and sub_2070a8 deeper with Capstone. If static analysis is already sufficient, move to Unicorn emulation using nmss_emu.py against output/decrypted/nmsscr.dec."),
        "oracle" => format!("{base}If the build is complete, run the oracle now and record the output."),
        "hybrid" => format!("{base}If crypto is not solved yet, stub the computation and validate the capture path anyway."),
        _ => base,
    }
}

fn follow_up_prompt(ctx: &ReducerContext, agent_name: &str) -> Option<String> {
    current_sub_goal_row(ctx, agent_name).map(|_| current_task_prompt(ctx, agent_name))
}

fn cross_pollinate_internal(ctx: &ReducerContext) {
    let algo_details = ctx
        .db
        .facts()
        .fact_key()
        .find(&"crypto.algorithm_details".to_string())
        .map(|row| row.value_json);
    if let Some(algo_details) = algo_details {
        if !fact_is_true(ctx, &Some("hybrid.algorithm_shared".to_string())) {
            queue_action_internal(
                ctx,
                Some("hybrid".to_string()),
                "send_prompt".to_string(),
                format!(
                    "{{\"text\":{}}}",
                    serde_json_escape(&format!(
                        "Crypto recovered the algorithm details below. Use them to replace the stub and validate capture end-to-end:\n\n{}",
                        algo_details
                    ))
                ),
                Some("Share crypto findings with hybrid".to_string()),
            );
            fact_set(
                ctx,
                FactInput {
                    fact_key: "hybrid.algorithm_shared".to_string(),
                    value_json: "true".to_string(),
                    confidence: Some(1.0),
                    source_type: Some("policy".to_string()),
                    source_ref: Some("cross_pollinate".to_string()),
                    metadata_json: Some("{}".to_string()),
                },
            );
        }
    }

    let session_data = ctx
        .db
        .facts()
        .fact_key()
        .find(&"hybrid.session_data".to_string())
        .map(|row| row.value_json);
    if let Some(session_data) = session_data {
        if !fact_is_true(ctx, &Some("crypto.session_shared".to_string())) {
            queue_action_internal(
                ctx,
                Some("crypto".to_string()),
                "send_prompt".to_string(),
                format!(
                    "{{\"text\":{}}}",
                    serde_json_escape(&format!(
                        "Hybrid captured new session data. Use it to drive crypto analysis and emulation:\n\n{}",
                        session_data
                    ))
                ),
                Some("Share hybrid capture with crypto".to_string()),
            );
            fact_set(
                ctx,
                FactInput {
                    fact_key: "crypto.session_shared".to_string(),
                    value_json: "true".to_string(),
                    confidence: Some(1.0),
                    source_type: Some("policy".to_string()),
                    source_ref: Some("cross_pollinate".to_string()),
                    metadata_json: Some("{}".to_string()),
                },
            );
        }
    }
}

fn queue_index_actions_internal(ctx: &ReducerContext) {
    let mut rows: Vec<_> = ctx
        .db
        .artifacts()
        .iter()
        .filter(|row| row.significance == "high" && row.indexed_state == "pending")
        .collect();
    rows.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));
    for row in rows.into_iter().take(25) {
        queue_action_internal(
            ctx,
            None,
            "index_artifact".to_string(),
            format!("{{\"path\":{}}}", serde_json_escape(&row.path)),
            Some("Index significant artifact".to_string()),
        );
    }
}

fn decide_actions_internal(ctx: &ReducerContext) {
    resolve_active_sub_goals(ctx);
    let agents: Vec<_> = ctx.db.agents().iter().collect();
    for agent in agents {
        if agent.status == "paused" {
            continue;
        }
        let preview = agent.last_capture_preview.clone().unwrap_or_default();
        match agent.status.as_str() {
            "dead" => queue_action_internal(
                ctx,
                Some(agent.name.clone()),
                "restart_agent".to_string(),
                format!(
                    "{{\"task\":{}}}",
                    serde_json_escape(&current_task_prompt(ctx, &agent.name))
                ),
                Some("Agent appears dead".to_string()),
            ),
            "stuck" => queue_action_internal(
                ctx,
                Some(agent.name.clone()),
                "send_prompt".to_string(),
                format!(
                    "{{\"text\":{}}}",
                    serde_json_escape(&corrective_prompt(ctx, &agent.name, &preview))
                ),
                Some("Agent appears stuck".to_string()),
            ),
            "idle" => {
                if let Some(prompt) = follow_up_prompt(ctx, &agent.name) {
                    queue_action_internal(
                        ctx,
                        Some(agent.name.clone()),
                        "send_prompt".to_string(),
                        format!("{{\"text\":{}}}", serde_json_escape(&prompt)),
                        Some("Idle follow-up".to_string()),
                    );
                }
            }
            _ => {}
        }
    }
    cross_pollinate_internal(ctx);
    queue_index_actions_internal(ctx);
}

fn biome_request(
    ctx: &mut ProcedureContext,
    method: &str,
    url: String,
    body: Option<String>,
) -> Result<spacetimedb::http::Response<Body>, String> {
    let mut builder = Request::builder()
        .method(method)
        .uri(url)
        .extension(Timeout::from(TimeDuration::from_micros(
            (BIOME_HTTP_TIMEOUT_MS * 1_000) as i64,
        )));
    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }
    let request = builder
        .body(body.map(Body::from).unwrap_or_else(Body::empty))
        .map_err(|err| err.to_string())?;
    ctx.http.send(request).map_err(|err| err.to_string())
}

fn biome_screen(
    ctx: &mut ProcedureContext,
    base_url: &str,
    pane_id: &str,
    lines: usize,
) -> Result<String, String> {
    let response = biome_request(ctx, "GET", format!("{base_url}/panes/{pane_id}/screen"), None)?;
    if !response.status().is_success() {
        return Err(format!("biome screen failed with {}", response.status()));
    }
    let body = response.into_body().into_string().map_err(|err| err.to_string())?;
    let screen: BiomeScreen = serde_json::from_str(&body).map_err(|err| err.to_string())?;
    let len = screen.rows.len();
    let start = len.saturating_sub(lines);
    Ok(screen.rows[start..].join("\n"))
}

fn biome_send_text(ctx: &mut ProcedureContext, base_url: &str, pane_id: &str, text: &str) -> Result<(), String> {
    let payload = serde_json::json!({
        "data": BASE64_STANDARD.encode(format!("{text}\r").as_bytes())
    });
    let response = biome_request(
        ctx,
        "POST",
        format!("{base_url}/panes/{pane_id}/input"),
        Some(payload.to_string()),
    )?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("biome input failed with {}", response.status()))
    }
}

fn biome_create_pane(ctx: &mut ProcedureContext, base_url: &str, agent_name: &str) -> Result<String, String> {
    let response = biome_request(
        ctx,
        "POST",
        format!("{base_url}/panes"),
        Some(
            serde_json::json!({
                "name": agent_name,
                "cols": 220,
                "rows": 50
            })
            .to_string(),
        ),
    )?;
    if !response.status().is_success() {
        return Err(format!("biome create pane failed with {}", response.status()));
    }
    let body = response.into_body().into_string().map_err(|err| err.to_string())?;
    let created: BiomePaneCreated = serde_json::from_str(&body).map_err(|err| err.to_string())?;
    Ok(created.id)
}

fn biome_delete_pane(ctx: &mut ProcedureContext, base_url: &str, pane_id: &str) -> Result<(), String> {
    let response = biome_request(ctx, "DELETE", format!("{base_url}/panes/{pane_id}"), None)?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("biome delete pane failed with {}", response.status()))
    }
}

fn apply_agent_poll(ctx: &ReducerContext, agent_name: &str, capture: Option<String>, previous_hash: Option<String>, previous_seen_at: Option<String>) -> PollSummary {
    let status = classify_capture(capture.as_deref(), previous_hash.as_deref(), previous_seen_at.as_deref());
    let content = capture.unwrap_or_default();
    let content_hash = if content.is_empty() {
        String::new()
    } else {
        hash_text(&content)
    };
    let preview = nonempty_lines(&content)
        .into_iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let mut agent = require_agent(ctx, agent_name).expect("agent should still exist during poll apply");
    agent.status = status.clone();
    agent.last_seen_at = Some(now(ctx));
    agent.last_capture_hash = Some(content_hash.clone());
    agent.last_capture_preview = if preview.is_empty() { None } else { Some(preview.clone()) };
    ctx.db.agents().name().update(agent);
    ctx.db.observations().insert(Observation {
        id: 0,
        agent_name: agent_name.to_string(),
        kind: "biome_capture".to_string(),
        content,
        content_hash: content_hash.clone(),
        created_at: now(ctx),
    });
    log_event(
        ctx,
        Some(agent_name.to_string()),
        "agent.polled",
        format!("status={status}"),
        if preview.is_empty() {
            None
        } else {
            Some(format!("{{\"preview\":{}}}", serde_json_escape(&preview)))
        },
    );
    PollSummary {
        agent_name: agent_name.to_string(),
        status,
        content_hash,
    }
}

fn default_agents() -> Vec<AgentInput> {
    vec![
        AgentInput {
            name: "oracle".to_string(),
            tmux_target: format!("{DEFAULT_TMUX_SESSION}:oracle"),
            workdir: Some(DEFAULT_PROJECT_DIR.to_string()),
            default_task: "Continue the oracle task in /home/sdancer/games/nmss/. If done, test the oracle by running it. If already tested, save the results to VikingDB via `mcp openviking add_resource`.".to_string(),
        },
        AgentInput {
            name: "crypto".to_string(),
            tmux_target: format!("{DEFAULT_TMUX_SESSION}:crypto"),
            workdir: Some(DEFAULT_PROJECT_DIR.to_string()),
            default_task: "Continue the crypto task in /home/sdancer/games/nmss/. If static analysis is done, try Unicorn emulation of the crypto function using nmss_emu.py. If stuck, hook sub_20bb48 and sub_2070a8 deeper with Capstone. Binary: output/decrypted/nmsscr.dec.".to_string(),
        },
        AgentInput {
            name: "hybrid".to_string(),
            tmux_target: format!("{DEFAULT_TMUX_SESSION}:hybrid"),
            workdir: Some(DEFAULT_PROJECT_DIR.to_string()),
            default_task: "Continue the hybrid capture task in /home/sdancer/games/nmss/. If the capture script is done, test it. If crypto is not solved yet, stub the computation and validate that the capture path works.".to_string(),
        },
    ]
}

fn bootstrap_goals() -> Vec<GoalInput> {
    vec![
        GoalInput {
            goal_key: "orchestrator.deliver_tested_oracle".to_string(),
            title: "Deliver tested oracle and indexed evidence".to_string(),
            detail: Some("Drive the oracle from implementation through execution and VikingDB indexing.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(10),
            depends_on_goal_key: None,
            success_fact_key: Some("oracle.indexed".to_string()),
            metadata_json: None,
            completion_report: None,
        },
        GoalInput {
            goal_key: "orchestrator.recover_crypto".to_string(),
            title: "Recover the crypto algorithm".to_string(),
            detail: Some("Drive crypto from static analysis through emulation to an identified algorithm.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(20),
            depends_on_goal_key: None,
            success_fact_key: Some("crypto.algorithm_identified".to_string()),
            metadata_json: None,
            completion_report: None,
        },
        GoalInput {
            goal_key: "orchestrator.validate_capture_path".to_string(),
            title: "Validate the capture path".to_string(),
            detail: Some("Build and test the capture flow; use a stub until the crypto is fully solved.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(30),
            depends_on_goal_key: None,
            success_fact_key: Some("hybrid.capture_tested".to_string()),
            metadata_json: None,
            completion_report: None,
        },
    ]
}

fn bootstrap_sub_goals() -> Vec<SubGoalInput> {
    vec![
        SubGoalInput {
            sub_goal_key: "oracle.test_oracle".to_string(),
            goal_key: "orchestrator.deliver_tested_oracle".to_string(),
            owner_agent: "oracle".to_string(),
            title: "Run the oracle".to_string(),
            detail: Some("Execute the oracle and capture concrete evidence.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(10),
            depends_on_sub_goal_key: None,
            success_fact_key: Some("oracle.tested".to_string()),
            instruction_text: Some("If you have finished the oracle implementation, run it now and capture the results.".to_string()),
            stuck_guidance_text: Some("If the build is complete, run the oracle now and record the output.".to_string()),
            metadata_json: None,
            blocked_by: None,
            completion_report: None,
        },
        SubGoalInput {
            sub_goal_key: "oracle.index_results".to_string(),
            goal_key: "orchestrator.deliver_tested_oracle".to_string(),
            owner_agent: "oracle".to_string(),
            title: "Index oracle results".to_string(),
            detail: Some("Save significant oracle artifacts to VikingDB.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(20),
            depends_on_sub_goal_key: Some("oracle.test_oracle".to_string()),
            success_fact_key: Some("oracle.indexed".to_string()),
            instruction_text: Some("The oracle appears tested. Save significant results to VikingDB via `mcp openviking add_resource` and note what you indexed.".to_string()),
            stuck_guidance_text: None,
            metadata_json: None,
            blocked_by: None,
            completion_report: None,
        },
        SubGoalInput {
            sub_goal_key: "crypto.static_analysis".to_string(),
            goal_key: "orchestrator.recover_crypto".to_string(),
            owner_agent: "crypto".to_string(),
            title: "Finish static analysis".to_string(),
            detail: Some("Recover enough structure to drive emulation.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(10),
            depends_on_sub_goal_key: None,
            success_fact_key: Some("crypto.static_analysis_done".to_string()),
            instruction_text: Some("Keep advancing the crypto analysis. Hook sub_20bb48 and sub_2070a8 deeper with Capstone if needed. Binary: output/decrypted/nmsscr.dec.".to_string()),
            stuck_guidance_text: Some("Try hooking sub_20bb48 and sub_2070a8 deeper with Capstone. Binary: output/decrypted/nmsscr.dec.".to_string()),
            metadata_json: None,
            blocked_by: None,
            completion_report: None,
        },
        SubGoalInput {
            sub_goal_key: "crypto.unicorn_emulation".to_string(),
            goal_key: "orchestrator.recover_crypto".to_string(),
            owner_agent: "crypto".to_string(),
            title: "Run Unicorn emulation".to_string(),
            detail: Some("Use nmss_emu.py against output/decrypted/nmsscr.dec.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(20),
            depends_on_sub_goal_key: Some("crypto.static_analysis".to_string()),
            success_fact_key: Some("crypto.unicorn_done".to_string()),
            instruction_text: Some("Static analysis appears complete. Try Unicorn emulation of the crypto function using nmss_emu.py. Binary: output/decrypted/nmsscr.dec. Capture any concrete inputs, outputs, or recovered constants.".to_string()),
            stuck_guidance_text: None,
            metadata_json: None,
            blocked_by: None,
            completion_report: None,
        },
        SubGoalInput {
            sub_goal_key: "crypto.identify_algorithm".to_string(),
            goal_key: "orchestrator.recover_crypto".to_string(),
            owner_agent: "crypto".to_string(),
            title: "Identify the algorithm".to_string(),
            detail: Some("Turn the recovered behavior into a concrete algorithm description and evidence.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(30),
            depends_on_sub_goal_key: Some("crypto.unicorn_emulation".to_string()),
            success_fact_key: Some("crypto.algorithm_identified".to_string()),
            instruction_text: Some("Turn the recovered behavior into a concrete algorithm description with evidence, constants, and any validation runs.".to_string()),
            stuck_guidance_text: None,
            metadata_json: None,
            blocked_by: None,
            completion_report: None,
        },
        SubGoalInput {
            sub_goal_key: "hybrid.capture_script".to_string(),
            goal_key: "orchestrator.validate_capture_path".to_string(),
            owner_agent: "hybrid".to_string(),
            title: "Build the capture script".to_string(),
            detail: Some("Produce a script that captures session data.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(10),
            depends_on_sub_goal_key: None,
            success_fact_key: Some("hybrid.capture_script_done".to_string()),
            instruction_text: Some("Keep iterating on the capture script. If crypto remains unresolved, use a stub to validate the session capture path.".to_string()),
            stuck_guidance_text: Some("If crypto is not solved yet, stub the computation and validate the capture path anyway.".to_string()),
            metadata_json: None,
            blocked_by: None,
            completion_report: None,
        },
        SubGoalInput {
            sub_goal_key: "hybrid.capture_test".to_string(),
            goal_key: "orchestrator.validate_capture_path".to_string(),
            owner_agent: "hybrid".to_string(),
            title: "Validate the capture path".to_string(),
            detail: Some("Test the capture flow; stub crypto if needed.".to_string()),
            status: Some("pending".to_string()),
            priority: Some(20),
            depends_on_sub_goal_key: Some("hybrid.capture_script".to_string()),
            success_fact_key: Some("hybrid.capture_tested".to_string()),
            instruction_text: Some("The capture script appears ready. Test it now; if crypto is not solved yet, stub the computation and validate that capture works.".to_string()),
            stuck_guidance_text: None,
            metadata_json: None,
            blocked_by: None,
            completion_report: None,
        },
    ]
}

fn upsert_agent_internal(ctx: &ReducerContext, input: AgentInput, biome_pane_id: Option<String>) {
    let existing = ctx.db.agents().name().find(&input.name);
    let row = Agent {
        name: input.name.clone(),
        tmux_target: input.tmux_target,
        workdir: opt_text(input.workdir),
        default_task: input.default_task,
        status: existing.as_ref().map(|row| row.status.clone()).unwrap_or_else(|| "unknown".to_string()),
        current_goal_key: existing.as_ref().and_then(|row| row.current_goal_key.clone()),
        current_sub_goal_key: existing.as_ref().and_then(|row| row.current_sub_goal_key.clone()),
        last_seen_at: existing.as_ref().and_then(|row| row.last_seen_at.clone()),
        last_capture_hash: existing.as_ref().and_then(|row| row.last_capture_hash.clone()),
        last_capture_preview: existing.as_ref().and_then(|row| row.last_capture_preview.clone()),
        metadata_json: existing
            .as_ref()
            .map(|row| row.metadata_json.clone())
            .unwrap_or_else(|| "{}".to_string()),
        biome_pane_id: biome_pane_id.or_else(|| existing.and_then(|row| row.biome_pane_id)),
    };
    let _ = ctx.db.agents().name().delete(&row.name);
    ctx.db.agents().insert(row);
}

fn upsert_goal_internal(ctx: &ReducerContext, input: GoalInput) -> Result<(), String> {
    if let Some(dep) = input.depends_on_goal_key.as_ref() {
        require_goal(ctx, dep)?;
    }
    let timestamp = now(ctx);
    let existing = ctx.db.goals().goal_key().find(&input.goal_key);
    let row = Goal {
        goal_key: input.goal_key.clone(),
        title: input.title,
        detail: opt_text(input.detail),
        status: input.status.unwrap_or_else(|| "pending".to_string()),
        priority: input.priority.unwrap_or(50),
        depends_on_goal_key: opt_text(input.depends_on_goal_key),
        success_fact_key: opt_text(input.success_fact_key),
        metadata_json: json_or_empty(input.metadata_json),
        completion_report: input.completion_report.or_else(|| existing.as_ref().and_then(|r| r.completion_report.clone())),
        created_at: existing
            .as_ref()
            .map(|row| row.created_at.clone())
            .unwrap_or_else(|| timestamp.clone()),
        updated_at: timestamp,
    };
    let _ = ctx.db.goals().goal_key().delete(&row.goal_key);
    ctx.db.goals().insert(row);
    Ok(())
}

fn upsert_sub_goal_internal(ctx: &ReducerContext, input: SubGoalInput) -> Result<(), String> {
    require_goal(ctx, &input.goal_key)?;
    require_agent(ctx, &input.owner_agent)?;
    if let Some(dep) = input.depends_on_sub_goal_key.as_ref() {
        require_sub_goal(ctx, dep)?;
    }
    let timestamp = now(ctx);
    let existing = ctx.db.sub_goals().sub_goal_key().find(&input.sub_goal_key);
    // Validate blocked_by references (comma-separated goal_keys or sub_goal_keys)
    if let Some(ref blocked_by) = input.blocked_by {
        for dep_key in blocked_by.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let goal_exists = ctx.db.goals().goal_key().find(&dep_key.to_string()).is_some();
            let sub_goal_exists = ctx.db.sub_goals().sub_goal_key().find(&dep_key.to_string()).is_some();
            if !goal_exists && !sub_goal_exists {
                return Err(format!("blocked_by references unknown key: {dep_key}"));
            }
        }
    }
    let row = SubGoal {
        sub_goal_key: input.sub_goal_key.clone(),
        goal_key: input.goal_key,
        owner_agent: input.owner_agent,
        title: input.title,
        detail: opt_text(input.detail),
        status: input.status.unwrap_or_else(|| "pending".to_string()),
        priority: input.priority.unwrap_or(50),
        depends_on_sub_goal_key: opt_text(input.depends_on_sub_goal_key),
        blocked_by: opt_text(input.blocked_by),
        success_fact_key: opt_text(input.success_fact_key),
        instruction_text: opt_text(input.instruction_text),
        stuck_guidance_text: opt_text(input.stuck_guidance_text),
        metadata_json: json_or_empty(input.metadata_json),
        completion_report: input.completion_report.or_else(|| existing.as_ref().and_then(|r| r.completion_report.clone())),
        created_at: existing
            .as_ref()
            .map(|row| row.created_at.clone())
            .unwrap_or_else(|| timestamp.clone()),
        updated_at: timestamp,
    };
    let _ = ctx.db.sub_goals().sub_goal_key().delete(&row.sub_goal_key);
    ctx.db.sub_goals().insert(row);
    Ok(())
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {}

#[spacetimedb::reducer]
pub fn seed_agents(ctx: &ReducerContext) {
    for agent in default_agents() {
        let name = agent.name.clone();
        upsert_agent_internal(ctx, agent, None);
        log_event(ctx, Some(name), "agent.seeded", "seeded default agent".to_string(), None);
    }
}

#[spacetimedb::reducer]
pub fn agent_add(
    ctx: &ReducerContext,
    name: String,
    biome_pane_id: String,
    workdir: Option<String>,
    default_task: Option<String>,
    tmux_target: Option<String>,
) {
    upsert_agent_internal(
        ctx,
        AgentInput {
            name: name.clone(),
            tmux_target: tmux_target.unwrap_or_default(),
            workdir,
            default_task: default_task.unwrap_or_else(|| "Continue the current task.".to_string()),
        },
        Some(biome_pane_id.clone()),
    );
    log_event(
        ctx,
        Some(name),
        "agent.registered",
        format!("biome_pane_id={biome_pane_id}"),
        None,
    );
}

#[spacetimedb::reducer]
pub fn agent_remove(ctx: &ReducerContext, name: String, delete: bool) -> Result<(), String> {
    let agent = require_agent(ctx, &name)?;
    if delete {
        for sub_goal in ctx.db.sub_goals().iter().filter(|row| row.owner_agent == name) {
            let _ = ctx.db.sub_goals().sub_goal_key().delete(&sub_goal.sub_goal_key);
        }
        for observation in ctx.db.observations().iter().filter(|row| row.agent_name == name) {
            let _ = ctx.db.observations().id().delete(observation.id);
        }
        for action in ctx.db.actions().iter().filter(|row| row.agent_name.as_deref() == Some(name.as_str())) {
            let _ = ctx.db.actions().id().delete(action.id);
        }
        for event in ctx.db.events().iter().filter(|row| row.agent_name.as_deref() == Some(name.as_str())) {
            let _ = ctx.db.events().id().delete(event.id);
        }
        let _ = ctx.db.agents().name().delete(&name);
        log_event(ctx, None, "agent.deleted", name, None);
    } else {
        let mut updated = agent;
        updated.status = "unknown".to_string();
        updated.biome_pane_id = None;
        ctx.db.agents().name().update(updated);
        log_event(ctx, Some(name.clone()), "agent.deregistered", name, None);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn bootstrap_known_goals(ctx: &ReducerContext) -> Result<(), String> {
    seed_agents(ctx);
    for goal in bootstrap_goals() {
        let key = goal.goal_key.clone();
        upsert_goal_internal(ctx, goal)?;
        log_event(ctx, None, "goal.upserted", key, Some("{\"bootstrap\":true}".to_string()));
    }
    for sub_goal in bootstrap_sub_goals() {
        let key = sub_goal.sub_goal_key.clone();
        let owner = sub_goal.owner_agent.clone();
        upsert_sub_goal_internal(ctx, sub_goal)?;
        log_event(ctx, Some(owner), "sub_goal.upserted", key, Some("{\"bootstrap\":true}".to_string()));
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn goal_add(ctx: &ReducerContext, input: GoalInput) -> Result<(), String> {
    let key = input.goal_key.clone();
    let status = input.status.clone().unwrap_or_else(|| "pending".to_string());
    upsert_goal_internal(ctx, input)?;
    log_event(
        ctx,
        None,
        "goal.upserted",
        key,
        Some(format!("{{\"status\":\"{status}\"}}")),
    );
    Ok(())
}

#[spacetimedb::reducer]
pub fn goal_update(ctx: &ReducerContext, goal_key: String, patch: GoalPatch) -> Result<(), String> {
    let current = require_goal(ctx, &goal_key)?;
    let next = GoalInput {
        goal_key: goal_key.clone(),
        title: patch.title.unwrap_or(current.title),
        detail: patch.detail.or(current.detail),
        status: patch.status.clone().or(Some(current.status)),
        priority: patch.priority.or(Some(current.priority)),
        depends_on_goal_key: if patch.clear_depends {
            None
        } else {
            patch.depends_on_goal_key.or(current.depends_on_goal_key)
        },
        success_fact_key: if patch.clear_success_fact {
            None
        } else {
            patch.success_fact_key.or(current.success_fact_key)
        },
        metadata_json: patch.metadata_json.or(Some(current.metadata_json)),
        completion_report: patch.completion_report.or(current.completion_report),
    };
    upsert_goal_internal(ctx, next)?;
    log_event(ctx, None, "goal.updated", goal_key, None);
    Ok(())
}

#[spacetimedb::reducer]
pub fn goal_remove(ctx: &ReducerContext, goal_key: String, delete: bool, cascade: bool) -> Result<(), String> {
    require_goal(ctx, &goal_key)?;
    if delete {
        let children: Vec<_> = ctx.db.sub_goals().iter().filter(|row| row.goal_key == goal_key).collect();
        if !children.is_empty() && !cascade {
            return Err(format!(
                "goal {goal_key} still has {} sub-goal(s); use --cascade or cancel instead",
                children.len()
            ));
        }
        for child in children {
            let _ = ctx.db.sub_goals().sub_goal_key().delete(&child.sub_goal_key);
        }
        for agent in ctx
            .db
            .agents()
            .iter()
            .filter(|row| row.current_goal_key.as_deref() == Some(goal_key.as_str()))
        {
            let mut updated = agent;
            updated.current_goal_key = None;
            updated.current_sub_goal_key = None;
            ctx.db.agents().name().update(updated);
        }
        let _ = ctx.db.goals().goal_key().delete(&goal_key);
        log_event(
            ctx,
            None,
            "goal.deleted",
            goal_key,
            Some(format!("{{\"cascade\":{cascade}}}")),
        );
    } else {
        goal_update(
            ctx,
            goal_key.clone(),
            GoalPatch {
                title: None,
                detail: None,
                status: Some("cancelled".to_string()),
                priority: None,
                depends_on_goal_key: None,
                success_fact_key: None,
                metadata_json: None,
                completion_report: None,
                clear_depends: false,
                clear_success_fact: false,
            },
        )?;
        for sub_goal in ctx.db.sub_goals().iter().filter(|row| row.goal_key == goal_key && row.status != "done") {
            let mut updated = sub_goal;
            updated.status = "cancelled".to_string();
            updated.updated_at = now(ctx);
            ctx.db.sub_goals().sub_goal_key().update(updated);
        }
        for agent in ctx
            .db
            .agents()
            .iter()
            .filter(|row| row.current_goal_key.as_deref() == Some(goal_key.as_str()))
        {
            let mut updated = agent;
            updated.current_goal_key = None;
            updated.current_sub_goal_key = None;
            ctx.db.agents().name().update(updated);
        }
        log_event(ctx, None, "goal.cancelled", goal_key, None);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn sub_goal_add(ctx: &ReducerContext, input: SubGoalInput) -> Result<(), String> {
    let key = input.sub_goal_key.clone();
    let owner = input.owner_agent.clone();
    upsert_sub_goal_internal(ctx, input)?;
    log_event(ctx, Some(owner), "sub_goal.upserted", key, None);
    Ok(())
}

#[spacetimedb::reducer]
pub fn sub_goal_update(ctx: &ReducerContext, sub_goal_key: String, patch: SubGoalPatch) -> Result<(), String> {
    let current = require_sub_goal(ctx, &sub_goal_key)?;
    let owner = patch.owner_agent.clone().unwrap_or_else(|| current.owner_agent.clone());
    let next = SubGoalInput {
        sub_goal_key: sub_goal_key.clone(),
        goal_key: patch.goal_key.unwrap_or(current.goal_key),
        owner_agent: owner.clone(),
        title: patch.title.unwrap_or(current.title),
        detail: patch.detail.or(current.detail),
        status: patch.status.or(Some(current.status)),
        priority: patch.priority.or(Some(current.priority)),
        depends_on_sub_goal_key: if patch.clear_depends {
            None
        } else {
            patch.depends_on_sub_goal_key.or(current.depends_on_sub_goal_key)
        },
        success_fact_key: if patch.clear_success_fact {
            None
        } else {
            patch.success_fact_key.or(current.success_fact_key)
        },
        instruction_text: if patch.clear_instruction {
            None
        } else {
            patch.instruction_text.or(current.instruction_text)
        },
        stuck_guidance_text: if patch.clear_stuck_guidance {
            None
        } else {
            patch.stuck_guidance_text.or(current.stuck_guidance_text)
        },
        metadata_json: patch.metadata_json.or(Some(current.metadata_json)),
        blocked_by: if patch.clear_blocked_by {
            None
        } else {
            patch.blocked_by.or(current.blocked_by)
        },
        completion_report: patch.completion_report.or(current.completion_report),
    };
    upsert_sub_goal_internal(ctx, next)?;
    log_event(ctx, Some(owner), "sub_goal.updated", sub_goal_key, None);
    Ok(())
}

#[spacetimedb::reducer]
pub fn sub_goal_remove(ctx: &ReducerContext, sub_goal_key: String, delete: bool) -> Result<(), String> {
    let current = require_sub_goal(ctx, &sub_goal_key)?;
    if delete {
        for agent in ctx
            .db
            .agents()
            .iter()
            .filter(|row| row.current_sub_goal_key.as_deref() == Some(sub_goal_key.as_str()))
        {
            let mut updated = agent;
            updated.current_sub_goal_key = None;
            ctx.db.agents().name().update(updated);
        }
        let _ = ctx.db.sub_goals().sub_goal_key().delete(&sub_goal_key);
        log_event(ctx, Some(current.owner_agent), "sub_goal.deleted", sub_goal_key, None);
    } else {
        let mut updated = current.clone();
        updated.status = "cancelled".to_string();
        updated.updated_at = now(ctx);
        ctx.db.sub_goals().sub_goal_key().update(updated);
        for agent in ctx
            .db
            .agents()
            .iter()
            .filter(|row| row.current_sub_goal_key.as_deref() == Some(sub_goal_key.as_str()))
        {
            let mut updated = agent;
            updated.current_sub_goal_key = None;
            ctx.db.agents().name().update(updated);
        }
        log_event(ctx, Some(current.owner_agent), "sub_goal.cancelled", sub_goal_key, None);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn fact_set(ctx: &ReducerContext, input: FactInput) {
    let fact_key = input.fact_key.clone();
    let row = Fact {
        fact_key: input.fact_key,
        value_json: input.value_json.clone(),
        confidence: input.confidence.unwrap_or(1.0),
        source_type: input.source_type.unwrap_or_else(|| "manual".to_string()),
        source_ref: opt_text(input.source_ref),
        updated_at: now(ctx),
        metadata_json: json_or_empty(input.metadata_json),
    };
    let _ = ctx.db.facts().fact_key().delete(&row.fact_key);
    ctx.db.facts().insert(row);
    log_event(
        ctx,
        None,
        "fact.updated",
        format!("{fact_key}={}", input.value_json),
        None,
    );
}

#[spacetimedb::reducer]
pub fn observation_add(
    ctx: &ReducerContext,
    agent_name: String,
    kind: String,
    content: String,
    content_hash: String,
) -> Result<(), String> {
    require_agent(ctx, &agent_name)?;
    ctx.db.observations().insert(Observation {
        id: 0,
        agent_name: agent_name.clone(),
        kind,
        content,
        content_hash,
        created_at: now(ctx),
    });
    log_event(ctx, Some(agent_name), "observation.recorded", "stored observation".to_string(), None);
    Ok(())
}

#[spacetimedb::reducer]
pub fn artifact_upsert(ctx: &ReducerContext, input: ArtifactInput) {
    let timestamp = now(ctx);
    let existing = ctx.db.artifacts().path().find(&input.path);
    let is_new = existing.is_none();
    let row = Artifact {
        path: input.path.clone(),
        sha256: opt_text(input.sha256),
        size_bytes: input.size_bytes,
        mtime_ns: input.mtime_ns,
        first_seen_at: existing
            .as_ref()
            .map(|row| row.first_seen_at.clone())
            .unwrap_or_else(|| timestamp.clone()),
        last_seen_at: timestamp,
        significance: input.significance.unwrap_or_else(|| "normal".to_string()),
        indexed_state: input.indexed_state.unwrap_or_else(|| "pending".to_string()),
        metadata_json: json_or_empty(input.metadata_json),
    };
    let _ = ctx.db.artifacts().path().delete(&row.path);
    ctx.db.artifacts().insert(row.clone());
    log_event(
        ctx,
        None,
        if is_new { "artifact.new" } else { "artifact.modified" },
        row.path,
        None,
    );
}

#[spacetimedb::reducer]
pub fn action_queue(ctx: &ReducerContext, input: ActionInput) -> Result<(), String> {
    if let Some(agent_name) = input.agent_name.as_ref() {
        require_agent(ctx, agent_name)?;
    }
    queue_action_internal(ctx, input.agent_name, input.action_type, input.payload_json, input.reason);
    Ok(())
}

#[spacetimedb::reducer]
pub fn action_complete(ctx: &ReducerContext, action_id: u64, status: String, result_text: Option<String>) -> Result<(), String> {
    let mut action = ctx
        .db
        .actions()
        .id()
        .find(action_id)
        .ok_or_else(|| format!("unknown action: {action_id}"))?;
    action.status = status.clone();
    action.executed_at = Some(now(ctx));
    action.result_text = opt_text(result_text.clone());
    ctx.db.actions().id().update(action.clone());
    log_event(
        ctx,
        action.agent_name,
        "action.executed",
        format!("{} -> {}", action.action_type, status),
        result_text,
    );
    Ok(())
}

#[spacetimedb::reducer]
pub fn queue_prompt(ctx: &ReducerContext, agent_name: String, text: String) -> Result<(), String> {
    action_queue(
        ctx,
        ActionInput {
            agent_name: Some(agent_name),
            action_type: "send_prompt".to_string(),
            payload_json: format!("{{\"text\":{}}}", serde_json_escape(&text)),
            reason: Some("manual queue".to_string()),
        },
    )
}

#[spacetimedb::reducer]
pub fn agent_poll_record(ctx: &ReducerContext, input: AgentPollInput) -> Result<(), String> {
    let mut agent = require_agent(ctx, &input.agent_name)?;
    agent.status = input.status.clone();
    agent.last_seen_at = Some(now(ctx));
    agent.last_capture_hash = opt_text(input.last_capture_hash);
    agent.last_capture_preview = opt_text(input.last_capture_preview.clone());
    ctx.db.agents().name().update(agent);
    log_event(
        ctx,
        Some(input.agent_name),
        "agent.polled",
        format!("status={}", input.status),
        input.last_capture_preview.map(|preview| format!("{{\"preview\":{}}}", serde_json_escape(&preview))),
    );
    Ok(())
}

#[spacetimedb::reducer]
pub fn resolve_goal_states(ctx: &ReducerContext) {
    let goals: Vec<_> = ctx.db.goals().iter().collect();
    for goal in goals {
        if goal.status == "cancelled" || goal.status == "paused" {
            continue;
        }
        let new_status = if fact_is_true(ctx, &goal.success_fact_key) {
            "done".to_string()
        } else if let Some(dep_key) = goal.depends_on_goal_key.as_ref() {
            let dep = ctx.db.goals().goal_key().find(dep_key);
            if dep.as_ref().map(|row| row.status.as_str()) != Some("done") {
                "blocked".to_string()
            } else {
                "active".to_string()
            }
        } else {
            "active".to_string()
        };
        if goal.status != new_status {
            let mut updated = goal;
            updated.status = new_status.clone();
            updated.updated_at = now(ctx);
            ctx.db.goals().goal_key().update(updated.clone());
            log_event(ctx, None, "goal.resolved", updated.goal_key, Some(format!("{{\"status\":\"{new_status}\"}}")));
        }
    }
}

#[spacetimedb::reducer]
pub fn resolve_sub_goal_states(ctx: &ReducerContext) {
    let sub_goals: Vec<_> = ctx.db.sub_goals().iter().collect();
    for sub_goal in sub_goals {
        if sub_goal.status == "cancelled" || sub_goal.status == "paused" {
            continue;
        }
        let parent_goal = ctx.db.goals().goal_key().find(&sub_goal.goal_key);
        let new_status = if fact_is_true(ctx, &sub_goal.success_fact_key) {
            "done".to_string()
        } else if !matches!(parent_goal.as_ref().map(|row| row.status.as_str()), Some("active" | "pending")) {
            "blocked".to_string()
        } else if let Some(dep_key) = sub_goal.depends_on_sub_goal_key.as_ref() {
            let dep = ctx.db.sub_goals().sub_goal_key().find(dep_key);
            if dep.as_ref().map(|row| row.status.as_str()) != Some("done") {
                "blocked".to_string()
            } else {
                "pending".to_string()
            }
        } else if let Some(ref blocked_by) = sub_goal.blocked_by {
            // Check all blocked_by references — all must be "done" to unblock
            let all_done = blocked_by
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .all(|dep_key| {
                    let goal_done = ctx.db.goals().goal_key().find(&dep_key.to_string())
                        .map(|g| g.status == "done").unwrap_or(false);
                    let sub_goal_done = ctx.db.sub_goals().sub_goal_key().find(&dep_key.to_string())
                        .map(|sg| sg.status == "done").unwrap_or(false);
                    goal_done || sub_goal_done
                });
            if all_done { "pending".to_string() } else { "blocked".to_string() }
        } else {
            "pending".to_string()
        };
        if sub_goal.status != new_status {
            let mut updated = sub_goal;
            updated.status = new_status.clone();
            updated.updated_at = now(ctx);
            let owner = updated.owner_agent.clone();
            let key = updated.sub_goal_key.clone();
            ctx.db.sub_goals().sub_goal_key().update(updated);
            log_event(ctx, Some(owner), "sub_goal.resolved", key, Some(format!("{{\"status\":\"{new_status}\"}}")));
        }
    }
}

#[spacetimedb::reducer]
pub fn resolve_active_sub_goals(ctx: &ReducerContext) {
    resolve_goal_states(ctx);
    resolve_sub_goal_states(ctx);

    let agents: Vec<_> = ctx.db.agents().iter().collect();
    for agent in agents {
        let mut candidates: Vec<_> = ctx
            .db
            .sub_goals()
            .iter()
            .filter(|sg| sg.owner_agent == agent.name && sg.status == "pending")
            .filter_map(|sg| {
                ctx.db
                    .goals()
                    .goal_key()
                    .find(&sg.goal_key)
                    .filter(|goal| goal.status == "active")
                    .map(|goal| (goal.priority, sg.priority, sg))
            })
            .collect();
        candidates.sort_by(|a, b| (a.0, a.1, a.2.sub_goal_key.as_str()).cmp(&(b.0, b.1, b.2.sub_goal_key.as_str())));

        if let Some((_, _, selected)) = candidates.into_iter().next() {
            for active in ctx
                .db
                .sub_goals()
                .iter()
                .filter(|row| row.owner_agent == agent.name && row.status == "active" && row.sub_goal_key != selected.sub_goal_key)
            {
                let mut updated = active;
                updated.status = "pending".to_string();
                updated.updated_at = now(ctx);
                ctx.db.sub_goals().sub_goal_key().update(updated);
            }
            let mut selected_row = selected.clone();
            selected_row.status = "active".to_string();
            selected_row.updated_at = now(ctx);
            ctx.db.sub_goals().sub_goal_key().update(selected_row.clone());

            let mut updated_agent = agent.clone();
            updated_agent.current_goal_key = Some(selected.goal_key.clone());
            updated_agent.current_sub_goal_key = Some(selected.sub_goal_key.clone());
            ctx.db.agents().name().update(updated_agent);
            log_event(
                ctx,
                Some(agent.name),
                "agent.sub_goal_assigned",
                selected.sub_goal_key,
                Some(format!("{{\"goal_key\":\"{}\"}}", selected.goal_key)),
            );
        } else {
            let mut updated_agent = agent;
            updated_agent.current_goal_key = None;
            updated_agent.current_sub_goal_key = None;
            ctx.db.agents().name().update(updated_agent);
        }
    }
}

#[spacetimedb::reducer]
pub fn decide_actions(ctx: &ReducerContext) {
    decide_actions_internal(ctx);
}

#[spacetimedb::procedure]
pub fn poll_agents_biome(
    ctx: &mut ProcedureContext,
    base_url: Option<String>,
    lines: Option<u32>,
) -> Result<Vec<PollSummary>, String> {
    let base_url = biome_base_url(base_url.as_deref());
    let lines = lines.map(|value| value as usize).unwrap_or(POLL_SCROLLBACK_LINES);
    let agents = ctx.try_with_tx(|tx| {
        Ok::<_, String>(
            tx.db
                .agents()
                .iter()
                .filter(|agent| agent.status != "paused")
                .filter_map(|agent| {
                    agent.biome_pane_id.clone().map(|biome_pane_id| AgentRuntime {
                        name: agent.name,
                        biome_pane_id,
                        last_seen_at: agent.last_seen_at,
                        last_capture_hash: agent.last_capture_hash,
                    })
                })
                .collect::<Vec<_>>(),
        )
    })?;

    let mut summaries = Vec::with_capacity(agents.len());
    for agent in agents {
        let capture = biome_screen(ctx, &base_url, &agent.biome_pane_id, lines).ok();
        let summary = ctx.try_with_tx(|tx| {
            Ok::<_, String>(apply_agent_poll(
                tx,
                &agent.name,
                capture.clone(),
                agent.last_capture_hash.clone(),
                agent.last_seen_at.clone(),
            ))
        })?;
        summaries.push(summary);
    }
    Ok(summaries)
}

#[spacetimedb::procedure]
pub fn send_prompt_biome(
    ctx: &mut ProcedureContext,
    agent_name: String,
    text: String,
    base_url: Option<String>,
) -> Result<String, String> {
    let base_url = biome_base_url(base_url.as_deref());
    let pane_id = ctx.try_with_tx(|tx| {
        Ok::<_, String>(
            require_agent(tx, &agent_name)?
                .biome_pane_id
                .ok_or_else(|| format!("agent {agent_name} has no biome_pane_id"))?,
        )
    })?;
    biome_send_text(ctx, &base_url, &pane_id, &text)?;
    Ok("sent".to_string())
}

#[spacetimedb::procedure]
pub fn execute_pending_actions_biome(
    ctx: &mut ProcedureContext,
    base_url: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<PendingActionSummary>, String> {
    let base_url = biome_base_url(base_url.as_deref());
    let limit = limit.unwrap_or(10) as usize;
    let actions = ctx.try_with_tx(|tx| {
        let mut rows: Vec<_> = tx.db.actions().iter().filter(|row| row.status == "pending").collect();
        rows.sort_by_key(|row| row.id);
        Ok::<_, String>(rows.into_iter().take(limit).collect::<Vec<_>>())
    })?;

    let mut results = Vec::new();
    for action in actions {
        let (status, result_text) = match action.action_type.as_str() {
            "send_prompt" => {
                let agent_name = action
                    .agent_name
                    .clone()
                    .ok_or_else(|| format!("action {} missing agent_name", action.id))?;
                let pane_id = ctx.try_with_tx(|tx| {
                    Ok::<_, String>(
                        require_agent(tx, &agent_name)?
                            .biome_pane_id
                            .ok_or_else(|| format!("agent {agent_name} has no biome_pane_id"))?,
                    )
                })?;
                let payload: serde_json::Value =
                    serde_json::from_str(&action.payload_json).map_err(|err| err.to_string())?;
                let text = payload
                    .get("text")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| format!("action {} missing payload.text", action.id))?;
                match biome_send_text(ctx, &base_url, &pane_id, text) {
                    Ok(()) => ("done".to_string(), "sent".to_string()),
                    Err(err) => ("failed".to_string(), err),
                }
            }
            "restart_agent" => {
                let agent_name = action
                    .agent_name
                    .clone()
                    .ok_or_else(|| format!("action {} missing agent_name", action.id))?;
                let (agent, default_task_text) = ctx.try_with_tx(|tx| {
                    Ok::<_, String>((
                        require_agent(tx, &agent_name)?,
                        current_task_prompt(tx, &agent_name),
                    ))
                })?;
                let task = serde_json::from_str::<serde_json::Value>(&action.payload_json)
                    .ok()
                    .and_then(|value| value.get("task").and_then(|value| value.as_str()).map(str::to_string))
                    .unwrap_or(default_task_text);
                let old_pane = agent.biome_pane_id.clone();
                if let Some(old_pane) = old_pane.as_deref() {
                    let _ = biome_delete_pane(ctx, &base_url, old_pane);
                }
                match biome_create_pane(ctx, &base_url, &agent_name) {
                    Ok(new_pane_id) => {
                        let workdir = agent.workdir.unwrap_or_else(|| "~".to_string());
                        let boot = format!("cd {workdir} && claude --dangerously-skip-permissions");
                        let send_result = biome_send_text(ctx, &base_url, &new_pane_id, &boot)
                            .and_then(|_| biome_send_text(ctx, &base_url, &new_pane_id, &task));
                        let _ = ctx.try_with_tx(|tx| {
                            let mut row = require_agent(tx, &agent_name)?;
                            row.biome_pane_id = Some(new_pane_id.clone());
                            tx.db.agents().name().update(row);
                            Ok::<_, String>(())
                        });
                        match send_result {
                            Ok(()) => (
                                "done".to_string(),
                                format!("restarted with new biome pane {new_pane_id}"),
                            ),
                            Err(err) => ("failed".to_string(), err),
                        }
                    }
                    Err(err) => ("failed".to_string(), err),
                }
            }
            _ => (
                "failed".to_string(),
                format!("unsupported biome action type: {}", action.action_type),
            ),
        };

        ctx.try_with_tx(|tx| action_complete(tx, action.id, status.clone(), Some(result_text.clone())))?;
        results.push(PendingActionSummary {
            action_id: action.id,
            status,
            result_text,
        });
    }

    Ok(results)
}

#[spacetimedb::procedure]
pub fn run_once_biome(
    ctx: &mut ProcedureContext,
    base_url: Option<String>,
    lines: Option<u32>,
    execute: Option<bool>,
    limit: Option<u32>,
) -> Result<Vec<PendingActionSummary>, String> {
    let _ = poll_agents_biome(ctx, base_url.clone(), lines)?;
    ctx.try_with_tx(|tx| {
        decide_actions_internal(tx);
        Ok::<_, String>(())
    })?;
    if execute.unwrap_or(false) {
        execute_pending_actions_biome(ctx, base_url, limit)
    } else {
        Ok(Vec::new())
    }
}

fn serde_json_escape(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}
