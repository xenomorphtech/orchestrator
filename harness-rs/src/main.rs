use std::io::Write;
use std::process::{Command, ExitCode};

use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use reqwest::blocking::Client;
use serde_json::{json, Value};

#[derive(Parser)]
#[command(name = "harness")]
#[command(about = "Rust CLI for the harness SpacetimeDB module")]
struct Cli {
    #[arg(long, default_value = "orchestrator-harness")]
    database: String,
    #[arg(long, default_value = "http://127.0.0.1:3001")]
    server: String,
    #[command(subcommand)]
    command: Commands,
}

struct CliContext {
    database: String,
    server: String,
    client: Client,
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
    AgentRemove(AgentRemoveArgs),
}

#[derive(Args)]
struct PollBiomeArgs {
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long, default_value_t = 20)]
    lines: u32,
}

#[derive(Args)]
struct ExecuteBiomeArgs {
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long, default_value_t = 10)]
    limit: u32,
}

#[derive(Args)]
struct RunOnceBiomeArgs {
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long, default_value_t = 20)]
    lines: u32,
    #[arg(long)]
    execute: bool,
    #[arg(long, default_value_t = 10)]
    limit: u32,
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
}

#[derive(Args)]
struct AgentRemoveArgs {
    name: String,
    #[arg(long)]
    delete: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let _ = writeln!(&mut std::io::stderr().lock(), "{err:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let context = CliContext {
        database: cli.database,
        server: cli.server,
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
            sql(&context, "SELECT id, agent_name, action_type, status, reason FROM actions LIMIT 10")
        }
        Commands::DecideActions => call_reducer(&context, "decide_actions", None),
        Commands::ResolveActiveSubGoals => call_reducer(&context, "resolve_active_sub_goals", None),
        Commands::PollBiome(args) => call_procedure(
            &context,
            "poll_agents_biome",
            vec![
                optional_json_string(args.base_url),
                Value::Number(args.lines.into()),
            ],
        ),
        Commands::ExecuteBiome(args) => call_procedure(
            &context,
            "execute_pending_actions_biome",
            vec![
                optional_json_string(args.base_url),
                Value::Number(args.limit.into()),
            ],
        ),
        Commands::RunOnceBiome(args) => call_procedure(
            &context,
            "run_once_biome",
            vec![
                optional_json_string(args.base_url),
                Value::Number(args.lines.into()),
                Value::Bool(args.execute),
                Value::Number(args.limit.into()),
            ],
        ),
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
                "confidence": json!({"some": args.confidence}),
                "source_type": some_json_string(args.source_type),
                "source_ref": optional_json_string(args.source_ref),
                "metadata_json": some_json_string("{}".to_string())
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
                "metadata_json": some_json_string("{}".to_string()),
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
                "metadata_json": some_json_string("{}".to_string()),
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
            ]),
        ),
        Commands::AgentRemove(args) => call_reducer(
            &context,
            "agent_remove",
            Some(vec![Value::String(args.name), Value::Bool(args.delete)]),
        ),
    }
}

fn call_reducer(cli: &CliContext, reducer: &str, args: Option<Vec<Value>>) -> Result<()> {
    run_call(cli, reducer, args.unwrap_or_default())
}

fn call_procedure(cli: &CliContext, procedure: &str, args: Vec<Value>) -> Result<()> {
    run_call(cli, procedure, args)
}

fn run_call(cli: &CliContext, name: &str, args: Vec<Value>) -> Result<()> {
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

    if !text.is_empty() {
        if let Ok(json) = serde_json::from_str::<Value>(&text) {
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("{text}");
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
        println!("{text}");
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
                    println!("(0 rows)");
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
                println!(" {}", header.join(" | "));

                // separator
                let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
                println!("-{}-", sep.join("-+-"));

                // rows
                for row in &string_rows {
                    let cells: Vec<String> = row
                        .iter()
                        .enumerate()
                        .map(|(i, val)| {
                            format!("{:w$}", val, w = widths.get(i).copied().unwrap_or(0))
                        })
                        .collect();
                    println!(" {}", cells.join(" | "));
                }
                println!("({} rows)", string_rows.len());
                println!();
            }
            _ => {
                if let Ok(pretty) = serde_json::to_string_pretty(result) {
                    println!("{pretty}");
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

fn none_json() -> Value {
    json!({ "none": {} })
}
