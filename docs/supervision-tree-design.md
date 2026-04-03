# Supervision Tree Architecture for Agent Orchestration

An OTP-inspired hierarchical supervision model that evolves the current flat
orchestrator into a tree of domain supervisors and tiered worker agents.

---

## Status Quo

Today, one Claude session (the orchestrator) directly manages all agents:

```
orchestrator (this session)
  |-- oracle    (Claude in biome_term)
  |-- crypto    (Claude in biome_term)
  |-- hybrid    (Claude in biome_term)
```

The harness DB stores Agents, Goals, SubGoals, Facts, Observations, Actions, and
Events. Every agent is a peer — flat. The orchestrator polls them all, decides
actions for each, cross-pollinates facts, and restarts the dead. This works for
3-5 agents on a single project. It breaks down when:

- Multiple projects run concurrently (aeon, nmss, vampir, biome_term itself).
- Agent count exceeds what one orchestrator can poll in a cycle (~10+).
- A stuck supervisor-level problem cascades into all workers.
- Different projects need different restart policies, escalation paths, and
  complexity tiers.

---

## OTP Concepts and Their Mappings

### The Erlang Model

In OTP, a supervision tree is a hierarchy of supervisor processes. Each
supervisor manages child processes (workers or other supervisors) according to a
**restart strategy** and **child spec**. When a child crashes, the supervisor
decides what to do based on the strategy.

| OTP Concept          | Our Mapping                                                      |
|----------------------|------------------------------------------------------------------|
| **Process**          | An agent running in a biome_term pane (Claude or Codex)          |
| **Supervisor**       | A Claude Code agent whose sole job is managing its children      |
| **Worker**           | A Claude/Codex agent executing a sub-goal                        |
| **Child spec**       | SubGoal + agent config (tier, workdir, default_task, etc.)       |
| **Restart strategy** | `strategy` field on Goal: `one_for_one`, `one_for_all`, `rest_for_one` |
| **Max restarts**     | `max_restarts` field on Goal (default: 3)                        |
| **Max seconds**      | `max_restart_seconds` field on Goal (default: 3600)              |
| **init/1**           | Domain supervisor spawns workers on startup                      |
| **terminate/2**      | Supervisor drains workers before shutting down                    |
| **Escalation**       | Worker fails beyond max_restarts -> supervisor marks goal failed -> root handles |

### Restart Strategies

**one_for_one** — When one child dies, only that child is restarted. Other
children are unaffected. Use this for independent parallel tasks.

```
aeon-supervisor (one_for_one)
  |-- aeon-lift-arm64    (S, Codex) — lift instructions
  |-- aeon-datalog       (S, Codex) — run datalog passes
  |-- aeon-crypto-search (M, Claude) — behavioral crypto detection
```

If `aeon-lift-arm64` dies, the others keep working. Only it gets restarted.

**one_for_all** — When any child dies, all children are terminated and
restarted. Use this for tightly coupled parallel tracks where one failure
invalidates the others' work.

```
nmss-supervisor (one_for_all for crypto+hybrid track)
  |-- crypto (M, Claude) — recover algorithm
  |-- hybrid (M, Claude) — build capture path using crypto output
```

If `crypto` dies and loses its algorithm recovery progress, `hybrid` is working
against stale assumptions. Kill both, restart both with fresh context.

**rest_for_one** — When a child dies, it and all children started *after* it are
terminated and restarted. Use this for sequential pipelines where downstream
stages depend on upstream output.

```
aeon-eval-supervisor (rest_for_one)
  |-- aeon-build         (S, Codex) — compile the workspace [started 1st]
  |-- aeon-eval-harness  (M, Claude) — run eval suite   [started 2nd]
  |-- aeon-eval-report   (S, Codex) — generate report   [started 3rd]
```

If `aeon-eval-harness` dies, `aeon-eval-report` (downstream) is killed and both
are restarted. `aeon-build` (upstream) keeps running.

---

## The Supervision Tree

### Three-Level Hierarchy

```
Root Supervisor (this session — the orchestrator)
  |
  |-- aeon-supervisor       (Claude Code agent in biome_term)
  |     |-- aeon-lift        (S, Codex)
  |     |-- aeon-il-refine   (M, Claude)
  |     |-- aeon-eval        (L, Claude+worktree)
  |
  |-- nmss-supervisor       (Claude Code agent in biome_term)
  |     |-- oracle           (M, Claude)
  |     |-- crypto           (M, Claude)
  |     |-- hybrid           (M, Claude)
  |
  |-- biome-supervisor      (Claude Code agent in biome_term)
  |     |-- biome-bugfix     (S, Codex)
  |     |-- biome-feature    (L, Claude+worktree)
  |
  |-- infra-supervisor      (Claude Code agent in biome_term)
        |-- harness-dev      (M, Claude)
        |-- openviking-dev   (M, Claude)
```

### Level Responsibilities

**Root supervisor (this session)**
- Spawns, monitors, and restarts domain supervisors only.
- Never talks to workers directly.
- Holds the global goal graph and cross-project dependencies.
- Escalation target when a domain supervisor exhausts its restart budget.
- Runs the top-level orchestration loop: poll supervisors, resolve cross-domain
  dependencies, redistribute work if a domain is overloaded.

**Domain supervisor (Claude Code in biome_term)**
- One per project/domain (aeon, nmss, biome_term, infrastructure).
- Is itself an agent in a biome_term pane running Claude Code.
- Owns a subset of the goal graph (goals scoped to its domain).
- Spawns, monitors, and restarts its workers.
- Runs `harness run-once-biome --execute` scoped to its agents.
- Cross-pollinates facts within its domain.
- Reports status upward via facts: `aeon.supervisor.status = "healthy"`.
- Has its own restart policy from root's perspective.

**Worker agent (Claude, Codex, or Claude+worktree)**
- Executes a single sub-goal.
- Classified by complexity tier (S/M/L).
- Reports progress via facts and sub-goal completion.
- Gets restarted by its domain supervisor when it dies or gets stuck.

---

## Complexity Tiers

| Tier | Runtime    | Agent Type        | Use Case                                            | Context Strategy      |
|------|------------|-------------------|-----------------------------------------------------|-----------------------|
| **S** | < 5 min   | Codex             | Targeted fixes, linting, single-file refactors, builds | Fire-and-forget       |
| **M** | 5-30 min  | Codex or Claude   | Multi-file changes, analysis tasks, test suites     | Monitor, nudge if idle |
| **L** | 30+ min   | Claude + worktree | Architecture changes, deep research, multi-step investigation | Active supervision, context refresh, git worktree isolation |

### Tier Selection Heuristics

The domain supervisor selects a tier based on the sub-goal:

```
if sub_goal.metadata_json contains "tier":
    use explicit tier
else if sub_goal has no dependencies AND title matches (fix|lint|format|build):
    tier = S
else if sub_goal has dependencies OR title matches (analyze|test|implement):
    tier = M
else if sub_goal has detail length > 500 OR multiple blocked_by entries:
    tier = L
```

### Agent Type by Tier

- **S**: Prefer Codex. Fast startup, good for bounded tasks. Always use
  `--dangerously-bypass-approvals-and-sandbox`.
- **M**: Codex for well-defined tasks, Claude for exploratory ones. Domain
  supervisor decides based on whether `instruction_text` is a precise command
  (Codex) or an open-ended investigation (Claude).
- **L**: Always Claude Code with `--dangerously-skip-permissions`. Spawn in a
  git worktree for isolation. Domain supervisor actively manages context via the
  refresh protocol (summarize -> /clear -> reinject).

---

## Restart Policies and Escalation

### Restart Policy (per Goal)

Each goal carries a restart policy in `metadata_json`:

```json
{
  "supervision": {
    "strategy": "one_for_one",
    "max_restarts": 3,
    "max_restart_seconds": 3600,
    "tier": "M",
    "agent_type": "claude"
  }
}
```

**Restart counter**: The domain supervisor tracks restart events per agent. When
`max_restarts` is hit within `max_restart_seconds`, the supervisor stops
restarting and escalates.

### Escalation Path

```
Worker dies
  -> Domain supervisor restarts it (up to max_restarts)
  -> If budget exhausted:
       Domain supervisor sets fact: "<domain>.escalation.<sub_goal_key> = true"
       Domain supervisor sets sub-goal status to "escalated"
       Domain supervisor reports to root via fact:
         "<domain>.supervisor.escalation = <sub_goal_key>"
  -> Root supervisor receives escalation
       Options:
         1. Reassign to a higher tier (S->M, M->L)
         2. Reassign to a different domain supervisor
         3. Mark goal as failed and unblock dependents with fallback
         4. Alert the human operator
```

### Domain Supervisor Restart Policy

Root treats domain supervisors as children too:

```json
{
  "supervision": {
    "strategy": "one_for_one",
    "max_restarts": 2,
    "max_restart_seconds": 7200,
    "tier": "L",
    "agent_type": "claude"
  }
}
```

When a domain supervisor itself dies:
1. Root restarts it in the same biome_term pane.
2. The supervisor re-discovers its workers from the harness DB (agents
   with `metadata_json.domain == "<domain>"`).
3. Living workers continue uninterrupted — the supervisor catches up by
   polling.
4. Dead workers get restarted by the supervisor.

This is the OTP principle: **let it crash, then recover state from durable
storage** (the harness DB).

---

## Context Preservation

Agent state is ephemeral (context windows compact and sessions die), but the
harness DB is durable. Context preservation strategy:

### Worker Context

1. **Sub-goal instruction_text**: The sub-goal's `instruction_text` is the
   canonical task description. When a worker restarts, it receives this prompt.
2. **Completion reports**: When a worker finishes a sub-goal, it writes a
   `completion_report` to the sub-goal row. Downstream workers can read this.
3. **Facts**: Key findings are persisted as facts. Workers query relevant facts
   on startup.
4. **Observations**: Screen captures are stored as observations. A restarted
   worker's supervisor can inject a summary of the last observation into the
   new prompt.

### Supervisor Context

1. **Goal graph in DB**: The supervisor reconstructs its world from the harness.
   Goals, sub-goals, agents, facts — all durable.
2. **Supervisor fact**: `<domain>.supervisor.state_summary` — a JSON blob the
   supervisor writes each cycle with its current assessment. Used to fast-track
   recovery after restart.
3. **Event log**: The events table provides an audit trail the supervisor can
   read to understand what happened before it died.

### Restart Prompt Template (Worker)

```
You are resuming work on sub-goal: {sub_goal.title}
Goal: {goal.title}
Previous status: {sub_goal.status} (restarted due to: {restart_reason})

Context from prior work:
{sub_goal.completion_report OR last observation summary}

Relevant facts:
{facts matching sub_goal.goal_key prefix}

Your task:
{sub_goal.instruction_text}

If the prior approach failed, try a different strategy.
Working directory: {agent.workdir}
```

### Restart Prompt Template (Domain Supervisor)

```
You are the {domain} domain supervisor. You manage worker agents for the
{domain} project.

Your workers (from harness DB):
{list of agents where metadata_json.domain == domain}

Your goals:
{list of goals where goal_key starts with domain prefix}

Last known state:
{fact: <domain>.supervisor.state_summary}

Recent events:
{last 20 events for agents in your domain}

Resume supervision. Poll your workers, assess their status, and take
corrective action as needed. Use `harness run-once-biome --execute` for
your scoped agents.
```

---

## Harness DB Schema Additions

The current schema needs these extensions to support supervision trees. All
changes are backward-compatible additions to `metadata_json` or new fields.

### Agent Table — New Fields

```rust
#[derive(Clone)]
#[spacetimedb::table(accessor = agents, public)]
pub struct Agent {
    // ... existing fields ...

    /// "worker" | "supervisor"
    #[default("worker".to_string())]
    pub role: String,

    /// Domain this agent belongs to (e.g., "aeon", "nmss")
    pub domain: Option<String>,

    /// Name of the supervisor agent managing this worker
    pub supervisor_agent: Option<String>,

    /// Complexity tier: "S", "M", "L"
    pub tier: Option<String>,

    /// Agent backend: "claude", "codex"
    pub agent_type: Option<String>,

    /// Number of restarts in current window
    #[default(0)]
    pub restart_count: u32,

    /// Timestamp of first restart in current window
    pub restart_window_start: Option<String>,
}
```

### Goal Table — New Fields

```rust
#[derive(Clone)]
#[spacetimedb::table(accessor = goals, public)]
pub struct Goal {
    // ... existing fields ...

    /// Domain scope: "aeon", "nmss", "biome", "infra"
    pub domain: Option<String>,

    /// Restart strategy: "one_for_one", "one_for_all", "rest_for_one"
    #[default("one_for_one".to_string())]
    pub restart_strategy: String,

    /// Max restarts before escalation
    #[default(3)]
    pub max_restarts: u32,

    /// Window in seconds for max_restarts counter
    #[default(3600)]
    pub max_restart_seconds: u32,
}
```

### SubGoal Table — New Fields

```rust
#[derive(Clone)]
#[spacetimedb::table(accessor = sub_goals, public)]
pub struct SubGoal {
    // ... existing fields ...

    /// Preferred complexity tier: "S", "M", "L"
    pub tier: Option<String>,

    /// Preferred agent type: "claude", "codex"
    pub preferred_agent_type: Option<String>,

    /// Start order within the goal (for rest_for_one strategy)
    pub start_order: Option<u32>,

    /// Status: "escalated" added to existing enum
    /// existing: pending, active, blocked, done, cancelled
    /// new: escalated
}
```

### New Reducer: `restart_agent_supervised`

```rust
#[spacetimedb::reducer]
pub fn restart_agent_supervised(
    ctx: &ReducerContext,
    agent_name: String,
    restart_reason: String,
) -> Result<(), String> {
    let mut agent = require_agent(ctx, &agent_name)?;
    let now_str = now(ctx);

    // Check restart budget
    let window_start = agent.restart_window_start.clone()
        .unwrap_or_else(|| now_str.clone());
    // ... parse timestamps, check if within window ...

    if agent.restart_count >= goal_max_restarts {
        // Escalate
        agent.status = "escalated".to_string();
        // Set escalation fact
        // ...
        return Err(format!("restart budget exhausted for {agent_name}"));
    }

    agent.restart_count += 1;
    agent.status = "restarting".to_string();
    // ... update agent, queue restart action ...
    Ok(())
}
```

### New Reducer: `apply_restart_strategy`

```rust
#[spacetimedb::reducer]
pub fn apply_restart_strategy(
    ctx: &ReducerContext,
    failed_agent_name: String,
    goal_key: String,
) -> Result<(), String> {
    let goal = require_goal(ctx, &goal_key)?;

    match goal.restart_strategy.as_str() {
        "one_for_one" => {
            // Restart only the failed agent
            restart_agent_supervised(ctx, failed_agent_name, "one_for_one".into())?;
        }
        "one_for_all" => {
            // Restart all agents under this goal
            let sub_goals: Vec<_> = ctx.db.sub_goals().goal_key()
                .filter(&goal_key).collect();
            for sg in sub_goals {
                restart_agent_supervised(ctx, sg.owner_agent, "one_for_all".into())?;
            }
        }
        "rest_for_one" => {
            // Find the failed agent's start_order, restart it and all after
            let failed_sg = ctx.db.sub_goals().iter()
                .find(|sg| sg.owner_agent == failed_agent_name
                    && sg.goal_key == goal_key);
            let failed_order = failed_sg
                .and_then(|sg| sg.start_order)
                .unwrap_or(0);
            let mut to_restart: Vec<_> = ctx.db.sub_goals().goal_key()
                .filter(&goal_key)
                .filter(|sg| sg.start_order.unwrap_or(0) >= failed_order)
                .collect();
            to_restart.sort_by_key(|sg| sg.start_order.unwrap_or(0));
            for sg in to_restart {
                restart_agent_supervised(ctx, sg.owner_agent, "rest_for_one".into())?;
            }
        }
        _ => return Err(format!("unknown strategy: {}", goal.restart_strategy)),
    }
    Ok(())
}
```

---

## Concrete Example: Aeon Project

The aeon project (ARM64 binary analysis toolkit) has multiple crates and
several natural work tracks.

### Goal Decomposition

```
Goal: aeon.deliver_v2
  Title: "Ship aeon v2 with IL improvements and eval harness"
  Domain: aeon
  Strategy: one_for_one (tracks are independent)
  max_restarts: 3, max_restart_seconds: 3600

  SubGoals:
    aeon.il_lifting (M, Claude)
      Title: "Improve AeonIL lifting coverage for ARM64"
      Instruction: "Extend the IL lifter in crates/aeonil/ to cover
        SIMD and FP instructions. Run `cargo test` after each change."
      start_order: 1

    aeon.reduce_engine (M, Codex)
      Title: "Implement reduce/simplification passes"
      Instruction: "Build expression simplification in crates/aeon-reduce/.
        Constant folding, dead store elimination. Test against eval corpus."
      start_order: 1

    aeon.eval_suite (L, Claude+worktree)
      Title: "Build comprehensive eval harness"
      Instruction: "In crates/aeon-eval/, create an evaluation framework
        that runs aeon against a corpus of binaries, measures coverage,
        and generates a report. Use a git worktree for isolation."
      start_order: 1
      depends_on: (none — but will consume outputs of the other two)

Goal: aeon.crypto_detection
  Title: "Harden behavioral crypto detection"
  Domain: aeon
  Strategy: rest_for_one (pipeline: analyze -> detect -> validate)
  max_restarts: 2, max_restart_seconds: 1800

  SubGoals:
    aeon.analyze_patterns (S, Codex)
      Title: "Catalog crypto patterns in sample binaries"
      start_order: 1

    aeon.detect_impl (M, Claude)
      Title: "Implement pattern matchers"
      Depends on: aeon.analyze_patterns
      start_order: 2

    aeon.validate_detection (S, Codex)
      Title: "Validate detection against known binaries"
      Depends on: aeon.detect_impl
      start_order: 3
```

### Supervision Tree

```
Root Supervisor (this session)
  |
  |-- aeon-supervisor (Claude Code, domain=aeon)
  |     |
  |     |-- [one_for_one: aeon.deliver_v2]
  |     |     |-- aeon-il-agent    (M, Claude)  — IL lifting
  |     |     |-- aeon-reduce-agent (M, Codex)  — reduce engine
  |     |     |-- aeon-eval-agent  (L, Claude+worktree) — eval suite
  |     |
  |     |-- [rest_for_one: aeon.crypto_detection]
  |           |-- aeon-patterns    (S, Codex)   — catalog patterns
  |           |-- aeon-detect      (M, Claude)  — implement matchers
  |           |-- aeon-validate    (S, Codex)   — validate detection
  |
  |-- nmss-supervisor (Claude Code, domain=nmss)
  |     |-- ... (existing oracle/crypto/hybrid agents)
```

### Lifecycle Walkthrough

1. **Root spawns aeon-supervisor:**
   ```bash
   curl -s -X POST http://localhost:3021/panes \
     -H 'Content-Type: application/json' \
     -d '{"name":"aeon-supervisor","cols":220,"rows":50}'

   harness send aeon-supervisor \
     "cd /home/sdancer/aeon && claude --dangerously-skip-permissions"

   # Wait for init, then inject supervisor role:
   harness send aeon-supervisor \
     "You are the aeon domain supervisor. <supervisor prompt template>"

   harness agent-add aeon-supervisor \
     --biome-pane-id <uuid> --workdir /home/sdancer/aeon \
     --default-task "Supervise aeon workers"
   ```

2. **aeon-supervisor spawns workers:**
   The supervisor reads its goals from the harness, then for each sub-goal:
   ```bash
   # S-tier: Codex
   curl -s -X POST http://localhost:3021/panes \
     -H 'Content-Type: application/json' \
     -d '{"name":"aeon-patterns","cols":220,"rows":50}'
   harness send aeon-patterns \
     'cd /home/sdancer/aeon && codex --dangerously-bypass-approvals-and-sandbox \
       "Catalog crypto patterns in sample binaries under samples/"'

   # M-tier: Claude
   curl -s -X POST http://localhost:3021/panes \
     -H 'Content-Type: application/json' \
     -d '{"name":"aeon-il-agent","cols":220,"rows":50}'
   harness send aeon-il-agent \
     "cd /home/sdancer/aeon && claude --dangerously-skip-permissions"
   # ... then send task prompt

   # L-tier: Claude in worktree
   cd /home/sdancer/aeon && git worktree add /tmp/aeon-eval-wt eval-branch
   curl -s -X POST http://localhost:3021/panes \
     -H 'Content-Type: application/json' \
     -d '{"name":"aeon-eval-agent","cols":220,"rows":50}'
   harness send aeon-eval-agent \
     "cd /tmp/aeon-eval-wt && claude --dangerously-skip-permissions"
   ```

3. **aeon-patterns (S, Codex) completes:**
   - Sets fact: `aeon.analyze_patterns.done = true`
   - aeon-supervisor sees this in its next poll cycle.
   - Since `aeon.crypto_detection` uses `rest_for_one` strategy,
     `aeon-detect` (start_order=2) can now start.

4. **aeon-detect (M, Claude) crashes:**
   - aeon-supervisor detects status=dead in poll.
   - Checks restart budget: restart_count=0, max_restarts=2. OK.
   - Applies `rest_for_one`: restart aeon-detect (order=2) and
     aeon-validate (order=3, not yet started, so no-op).
   - Increments restart_count, sends restart prompt with context.

5. **aeon-detect crashes again, then a third time:**
   - restart_count hits max_restarts (2).
   - aeon-supervisor escalates:
     ```bash
     harness fact-set aeon.escalation.aeon-detect "true"
     harness sub-goal-update aeon.detect_impl --status escalated
     ```
   - Root supervisor sees the escalation fact in its next cycle.
   - Root decides: upgrade tier from M to L (Claude+worktree), or
     reassign to a different agent, or mark as failed.

6. **aeon-supervisor itself crashes:**
   - Root detects aeon-supervisor status=dead.
   - Root restarts it in the same biome_term pane.
   - aeon-supervisor re-reads harness DB, finds its workers still
     running, catches up on their status, resumes supervision.

---

## Communication Protocol

### Root <-> Domain Supervisor

Root only communicates with domain supervisors. The protocol is:

| Direction | Mechanism | Content |
|-----------|-----------|---------|
| Root -> Supervisor | `harness send` prompt | High-level directives, new goals, re-prioritization |
| Supervisor -> Root | Facts | `<domain>.supervisor.status`, `<domain>.supervisor.escalation`, `<domain>.supervisor.state_summary` |
| Root -> Supervisor | Goal mutations | `harness goal-add`, `harness goal-update` scoped to domain |
| Supervisor -> Root | Sub-goal status | Supervisor updates sub-goal statuses in harness DB |

### Domain Supervisor <-> Workers

| Direction | Mechanism | Content |
|-----------|-----------|---------|
| Supervisor -> Worker | `harness send` prompt | Task instructions, nudges, corrective prompts |
| Worker -> Supervisor | Screen captures + facts | Supervisor polls screens, workers set facts on completion |
| Supervisor -> Worker | Restart | Kill pane + create new pane + send restart prompt |

### Cross-Domain Communication

Workers in different domains never talk directly. Cross-domain information
flows through the fact store:

```
aeon worker sets fact: aeon.crypto_search.rc4_found = true
  -> aeon-supervisor reads it, sets: aeon.finding.rc4_detected = true
  -> Root reads it, decides nmss workers could use this info
  -> Root tells nmss-supervisor via prompt or fact:
       nmss.cross_pollinate.aeon_rc4 = "<details>"
  -> nmss-supervisor shares with relevant workers
```

---

## Domain Supervisor Skill

Each domain supervisor is a Claude Code agent that runs the orchestration
skill scoped to its domain. The supervisor's injected system context:

```markdown
# Domain Supervisor: {domain}

You are a domain supervisor. You manage worker agents for the {domain}
project at {workdir}.

## Your Responsibilities
1. Poll your workers via biome_term screen captures.
2. Classify each worker: dead, stuck, idle, working.
3. Apply restart strategies when workers fail.
4. Track restart budgets. Escalate when exhausted.
5. Cross-pollinate findings between your workers via facts.
6. Report your status upward via facts.

## Your Workers
{dynamically populated from harness: agents where domain == this domain}

## Your Goals
{dynamically populated from harness: goals where domain == this domain}

## Restart Strategies
- one_for_one: restart only the failed worker.
- one_for_all: restart ALL workers under the same goal.
- rest_for_one: restart the failed worker AND all workers with higher
  start_order under the same goal.

## Escalation
When restart_count >= max_restarts within max_restart_seconds:
1. Set fact: {domain}.escalation.{sub_goal_key} = true
2. Set sub-goal status to "escalated"
3. Stop restarting. Root will handle it.

## Status Reporting
Every cycle, set:
  harness fact-set {domain}.supervisor.status "healthy|degraded|critical"
  harness fact-set {domain}.supervisor.state_summary "<JSON blob>"

## Commands
- Poll: harness run-once-biome --execute (scoped to your agents)
- Spawn: curl + harness agent-add
- Send: harness send <agent> "<prompt>"
- Facts: harness fact-set <key> <value>
```

---

## Migration Path

### Phase 1: Schema Extensions (no behavior change)

Add the new fields to Agent, Goal, SubGoal tables with sensible defaults.
All existing agents get `role=worker`, `domain=nmss`, `supervisor_agent=null`.
All existing goals get `restart_strategy=one_for_one`, `max_restarts=3`.

### Phase 2: Single Domain Supervisor (nmss)

Spawn one domain supervisor for the existing nmss project. Move the current
oracle/crypto/hybrid agents under it. Root stops managing workers directly
and only talks to nmss-supervisor. Validate that the restart/escalation
loop works.

### Phase 3: Multi-Domain

Spawn aeon-supervisor for the aeon project. Add goals and workers. Root
now manages two domain supervisors. Cross-domain fact sharing is tested.

### Phase 4: Full Autonomy

Domain supervisors autonomously spawn/kill workers based on goal graph
changes. Root only sets high-level goals and handles escalations. The
skill prompt for domain supervisors is refined based on operational
experience.

---

## Failure Modes and Mitigations

| Failure | Detection | Response |
|---------|-----------|----------|
| Worker dies | Supervisor polls screen, sees dead/terminated | Restart per strategy |
| Worker stuck >10min | Supervisor sees stale capture hash | Send corrective prompt, then restart if no change |
| Worker in crash loop | restart_count >= max_restarts | Escalate to root, try tier upgrade |
| Supervisor dies | Root polls supervisor pane | Restart supervisor; it re-discovers workers from DB |
| Supervisor stuck | Root sees no fact updates from supervisor for N cycles | Send nudge, then restart |
| Root dies (session ends) | Human notices | Human restarts orchestrator; it re-discovers supervisors from DB |
| biome_term down | All HTTP calls fail | Root alerts human; no automatic recovery |
| Harness DB down | SpacetimeDB unreachable | Root alerts human; agents continue working but lose coordination |
| All agents in a domain fail simultaneously | Supervisor sees all workers dead | one_for_all: restart all; one_for_one: restart each; escalate if budget blown |

---

## Summary of Key Principles

1. **Let it crash.** Workers and supervisors can die. The harness DB is the
   source of truth. Restart from durable state, not ephemeral context.

2. **Hierarchical isolation.** Root doesn't know about workers. Supervisors
   don't know about other domains. Each level manages one level down.

3. **Strategies match coupling.** Independent tasks get `one_for_one`. Coupled
   parallel tracks get `one_for_all`. Pipelines get `rest_for_one`.

4. **Escalation, not infinite retry.** Bounded restart budgets prevent
   crash loops. Escalation goes up the tree, never down.

5. **Tiers match complexity.** S-tier (Codex, <5min) for quick tasks.
   M-tier (Claude/Codex, 5-30min) for substantial work. L-tier
   (Claude+worktree, 30min+) for deep investigation.

6. **Facts are the nervous system.** All inter-agent communication goes
   through the fact store. No direct agent-to-agent messaging.

7. **Supervisors are agents too.** Domain supervisors are Claude Code
   instances in biome_term panes, managed by root exactly like root
   currently manages workers. The recursion is the architecture.
