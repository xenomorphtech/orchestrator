use std::process::{Command, ExitCode};

use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use serde_json::{json, Map, Value};

#[derive(Parser)]
#[command(name = "harness")]
#[command(about = "Rust CLI for the harness SpacetimeDB module")]
struct Cli {
    #[arg(long, default_value = "harness")]
    database: String,
    #[arg(long)]
    server: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

struct CliContext {
    database: String,
    server: Option<String>,
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
            eprintln!("{err:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let context = CliContext {
        database: cli.database,
        server: cli.server,
    };
    match cli.command {
        Commands::Build => run_status(Command::new("spacetime").args(["build", "-p", "harness"])),
        Commands::SeedAgents => call_reducer(&context, "seed_agents", None),
        Commands::BootstrapKnownGoals => call_reducer(&context, "bootstrap_known_goals", None),
        Commands::Agents => sql(&context, "SELECT * FROM agents ORDER BY name"),
        Commands::Goals => sql(&context, "SELECT * FROM goals ORDER BY priority, goal_key"),
        Commands::SubGoals => sql(
            &context,
            "SELECT * FROM sub_goals ORDER BY goal_key, owner_agent, priority, sub_goal_key",
        ),
        Commands::Facts => sql(
            &context,
            "SELECT fact_key, value_json, confidence, source_type, source_ref, updated_at FROM facts ORDER BY fact_key",
        ),
        Commands::Summary => {
            sql(&context, "SELECT name, status, current_goal_key, current_sub_goal_key, last_seen_at, last_capture_preview FROM agents ORDER BY name")?;
            sql(&context, "SELECT goal_key, status, priority, success_fact_key FROM goals ORDER BY priority, goal_key")?;
            sql(&context, "SELECT sub_goal_key, goal_key, owner_agent, status, priority FROM sub_goals ORDER BY goal_key, priority, sub_goal_key")?;
            sql(&context, "SELECT id, agent_name, action_type, status, reason FROM actions ORDER BY id DESC LIMIT 10")
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
                "confidence": args.confidence,
                "source_type": args.source_type,
                "source_ref": args.source_ref,
                "metadata_json": "{}"
            })]),
        ),
        Commands::GoalAdd(args) => call_reducer(
            &context,
            "goal_add",
            Some(vec![json!({
                "goal_key": args.goal_key,
                "title": args.title,
                "detail": args.detail,
                "status": args.status,
                "priority": args.priority,
                "depends_on_goal_key": args.depends_on_goal_key,
                "success_fact_key": args.success_fact_key,
                "metadata_json": "{}"
            })]),
        ),
        Commands::GoalUpdate(args) => call_reducer(
            &context,
            "goal_update",
            Some(vec![
                Value::String(args.goal_key),
                json!({
                    "title": args.title,
                    "detail": args.detail,
                    "status": args.status,
                    "priority": args.priority,
                    "depends_on_goal_key": args.depends_on_goal_key,
                    "success_fact_key": args.success_fact_key,
                    "metadata_json": null,
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
                    "title": null,
                    "detail": null,
                    "status": args.status,
                    "priority": null,
                    "depends_on_goal_key": null,
                    "success_fact_key": null,
                    "metadata_json": null,
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
                "detail": args.detail,
                "status": args.status,
                "priority": args.priority,
                "depends_on_sub_goal_key": args.depends_on_sub_goal_key,
                "success_fact_key": args.success_fact_key,
                "instruction_text": args.instruction_text,
                "stuck_guidance_text": args.stuck_guidance_text,
                "metadata_json": "{}"
            })]),
        ),
        Commands::SubGoalUpdate(args) => call_reducer(
            &context,
            "sub_goal_update",
            Some(vec![
                Value::String(args.sub_goal_key),
                json!({
                    "goal_key": args.goal_key,
                    "owner_agent": args.owner_agent,
                    "title": args.title,
                    "detail": args.detail,
                    "status": args.status,
                    "priority": args.priority,
                    "depends_on_sub_goal_key": args.depends_on_sub_goal_key,
                    "success_fact_key": args.success_fact_key,
                    "instruction_text": args.instruction_text,
                    "stuck_guidance_text": args.stuck_guidance_text,
                    "metadata_json": null,
                    "clear_depends": args.clear_depends,
                    "clear_success_fact": args.clear_success_fact,
                    "clear_instruction": args.clear_instruction,
                    "clear_stuck_guidance": args.clear_stuck_guidance
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
                    "goal_key": null,
                    "owner_agent": null,
                    "title": null,
                    "detail": null,
                    "status": args.status,
                    "priority": null,
                    "depends_on_sub_goal_key": null,
                    "success_fact_key": null,
                    "instruction_text": null,
                    "stuck_guidance_text": null,
                    "metadata_json": null,
                    "clear_depends": false,
                    "clear_success_fact": false,
                    "clear_instruction": false,
                    "clear_stuck_guidance": false
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
    let mut command = Command::new("spacetime");
    command.arg("call");
    if let Some(server) = cli.server.as_deref() {
        command.args(["--server", server]);
    }
    command.arg(&cli.database);
    command.arg(name);
    for arg in args {
        command.arg(arg.to_string());
    }
    run_status(&mut command)
}

fn sql(cli: &CliContext, query: &str) -> Result<()> {
    let mut command = Command::new("spacetime");
    command.arg("sql");
    if let Some(server) = cli.server.as_deref() {
        command.args(["--server", server]);
    }
    command.arg(&cli.database);
    command.arg(query);
    run_status(&mut command)
}

fn run_status(command: &mut Command) -> Result<()> {
    let status = command.status().with_context(|| format!("failed to run {:?}", command))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("command exited with status {status}"))
    }
}

fn optional_json_string(value: Option<String>) -> Value {
    match value {
        Some(value) => Value::String(value),
        None => Value::Null,
    }
}

#[allow(dead_code)]
fn compact_object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut object = Map::new();
    for (key, value) in entries {
        object.insert(key.to_string(), value);
    }
    Value::Object(object)
}
