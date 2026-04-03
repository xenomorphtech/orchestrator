# Supervision Tree Design Review

Reviewed against:
- `docs/supervision-tree-design.md`
- `orchestrate.md`
- `harness-rs/src/main.rs`
- `harness-rs/src/lib.rs`
- `../biome_term/docs/api.md`

## Bottom Line

The design is directionally useful, but it is not implementable as written on top of the current harness. A small single-domain pilot is feasible. A full OTP-style supervision tree with restart strategies, tiered worker spawning, and root/domain isolation is not, because the current harness is still a flat global scheduler with prompt injection and screen polling.

If humans are observers only, the gap is larger than the design currently admits. Escalation, retry exhaustion, repo conflicts, and infrastructure failure all need deterministic machine policies, not "alert the human" placeholders.

The biggest mistake in the design is treating "supervisors are agents too" as mostly a prompt problem. It is mainly an infrastructure problem.

## 1. Feasibility

### What is feasible now

- Register supervisor panes as normal agents and prompt them to manage a subset of workers.
- Use biome_term as the execution substrate. The API already supports pane create/list/input/delete plus screen and event capture (`../biome_term/docs/api.md:71-203`, `../biome_term/docs/api.md:241-283`).
- Use the existing `facts`, `observations`, `events`, `metadata_json`, and `completion_report` fields as a persistence layer (`harness-rs/src/lib.rs:115-257`).
- Run one preconfigured domain pilot where the worker set is static and restart policy is effectively `one_for_one`.

### What is not feasible with the current harness

- Scoped supervision is missing. The design says a domain supervisor runs `harness run-once-biome --execute` "scoped to its agents" (`docs/supervision-tree-design.md:138`, `docs/supervision-tree-design.md:322-323`, `docs/supervision-tree-design.md:733-734`), but `run-once-biome` has no filter arguments and always polls all agents, then runs global `decide_actions` (`harness-rs/src/main.rs:686-690`, `harness-rs/src/lib.rs:523-568`).
- Root isolation is missing. The design says root never talks to workers directly (`docs/supervision-tree-design.md:125-131`), but the current scheduler iterates every agent in the DB and queues prompts/restarts for all of them (`harness-rs/src/lib.rs:523-568`).
- Restart behavior is not backend-aware. `execute_restart_agent` always boots Claude with `claude --dangerously-skip-permissions` (`harness-rs/src/main.rs:695-753`). That breaks the proposed Codex tiers immediately.
- Worker spawning is not a first-class harness action. The design assumes supervisors can spawn/kill/manage workers routinely, but the executor only supports `send_prompt` and `restart_agent` (`harness-rs/src/main.rs:616-662`). There is no `spawn_agent`, `create_worktree`, `merge_worktree`, or `set_status` action path.
- Metadata needed by the design is not exposed via CLI. The tables already have `metadata_json`, and goals/sub-goals already have `completion_report`, but the CLI hardcodes metadata to `{}` for `goal-add`, `goal-update`, `sub-goal-add`, `sub-goal-update`, and `fact-set` (`harness-rs/src/main.rs:822-847`, `harness-rs/src/main.rs:849-920`).
- Current status detection is weaker than the orchestrator doc claims. `orchestrate.md` says to use `idle_seconds` and `terminated` from `/panes` (`orchestrate.md:38-43`), and biome_term exposes both (`../biome_term/docs/api.md:71-90`), but `cmd_poll_biome` only reads `/screen` and hashes text (`harness-rs/src/main.rs:531-613`).

### Missing infrastructure, concretely

- Agent scoping by `domain`, `role`, `supervisor_agent`, or explicit agent set.
- Launch spec per agent: backend, bootstrap command, worktree path, env, branch.
- Structured checkpoint/report API so workers can persist progress without hand-editing facts.
- Non-global action executor with more action types.
- File ownership or isolation policy for concurrent writers.
- A real escalation path beyond "set a fact and hope root notices".
- An autonomous failure policy for observer-only operation.
  - Example: pause goal, freeze writers, downgrade to read-only analysis, or fail closed.

## 2. Complexity vs Value

### Highest value for lowest effort

- Add scoped harness operations.
  - `run-once-biome`, `poll-biome`, `decide-actions`, and `execute-biome` need `--domain`, `--agent`, or `--metadata-filter`.
  - Without this, the tree cannot exist operationally.
- Make restart backend-aware.
  - Store launch metadata per agent and relaunch Claude vs Codex correctly.
  - This is required even before supervision trees if Codex agents are first-class.
- Add a simple worker checkpoint/report command.
  - Example: `harness sub-goal-report <sub_goal> --status done --report "..."`
  - Current `completion_report` exists in schema but is awkward to use from the CLI.
- Use existing `metadata_json` first instead of adding many columns immediately.
  - Domain, role, tier, backend, restart policy, start order, and ownership globs can all live there initially.

### Moderate value, moderate effort

- One domain supervisor pilot for `nmss`, with static workers and only `one_for_one`.
- Supervisor liveness facts like `<domain>.supervisor.status`.
- Use biome_term `/events` in addition to `/screen` for better incremental observation.
- Explicit automatic failure handling for "observer-only" mode.
  - Retry budget exhausted should map to a machine action, not a notification.

### Over-engineered for current scale

- Full three-level recursive tree across multiple projects on day one.
- `one_for_all` and `rest_for_one` as automatic restart policies.
  - These are attractive on paper, but not safe without disk-state isolation or rollback.
- Tier auto-selection heuristics (`docs/supervision-tree-design.md:159-183`).
  - Start with explicit tier/backend in metadata. Heuristics will be noisy and need debugging.
- Automatic cross-domain routing through root.
  - Current cross-pollination is still hard-coded for specific facts (`harness-rs/src/lib.rs:432-502`), not a generic message bus.
- Worktree automation only for L-tier.
  - If multiple agents can write concurrently, isolation is a repo-level concern, not just an L-tier concern.

## 3. OTP Mapping Accuracy

### Where the analogy helps

- Durable state matters more than live process memory.
- Restart budgets and escalation are useful concepts.
- Hierarchical ownership can reduce operator cognitive load.

### Where the analogy breaks

- OTP workers are deterministic processes with isolated memory. AI agents are not.
  - A restart does not recreate the same state machine. It recreates a new model session with partial textual context.
- OTP restarts recover process state. Here, disk mutations survive crashes.
  - If an agent half-edits files and then dies, `one_for_all` does not restore sibling workers to a clean pre-failure state.
- OTP mailboxes are structured and ordered. Here, communication is a mix of prompt injection, facts, screen polling, and event scraping.
- OTP supports strong child specs. Current agents do not have stable child specs; they have `workdir`, `default_task`, and an implicit launcher.
- OTP hot code reload does not map. The design should explicitly say this analogy stops at supervision and escalation patterns, not process semantics.
- Context compaction and context expiry make "let it crash" weaker than in Erlang.
  - The harness DB does not currently store enough structured checkpoints to recover real working state.

### Specific design claims to correct

- "Restart it in the same biome_term pane" is inaccurate (`docs/supervision-tree-design.md:243-249`).
  - Current restart deletes the old pane and creates a new one with a new pane ID (`harness-rs/src/main.rs:719-749`).
- "Facts are the nervous system" is overstated (`docs/supervision-tree-design.md:804-805`).
  - In practice, the current system also depends heavily on screen scraping and ad hoc prompts.

## 4. Practical Concerns

### Git conflict resolution

This is the biggest operational gap in the design.

- Multiple agents writing to the same checkout will race.
- `one_for_all` and `rest_for_one` are not meaningful unless you also define how to invalidate or roll back filesystem state.
- Restarting an agent does not revert its branch, uncommitted changes, or partially generated files.
- If no human will resolve conflicts, "multiple writers in one checkout" is not a recoverable operating mode.

### What needs to happen in practice

- Pick one of these policies and make it explicit:
  - One writer per repo, all others read-only.
  - One worktree per writer.
  - Explicit path ownership claims; no overlapping writers.
- My recommendation:
  - For an MVP, allow only one write-capable worker per repo/domain checkout.
  - If a second writer is necessary, it must get its own worktree with an automated merge/reconcile step.
- Add ownership metadata to sub-goals.
  - Example: `{"write_paths":["harness-rs/src/**","docs/**"]}`
- Supervisors should refuse to spawn overlapping writers.
- Do not treat human conflict resolution as part of the runtime design.
  - If a merge cannot be resolved automatically, the safe machine behavior is to mark the goal blocked/failed and stop further writes.

### How message passing actually works today

Current reality:

- Supervisor/root -> agent:
  - `harness send` -> biome_term `POST /panes/{id}/input`
- Agent -> supervisor/root:
  - Indirectly via terminal output on the pane screen
  - Optionally via `harness fact-set`
- Durable observation:
  - `cmd_poll_biome` snapshots `/screen` and stores preview + full observation (`harness-rs/src/main.rs:531-613`)
- Durable control state:
  - SpacetimeDB `facts`, `events`, `actions`, `sub_goals`

This means message passing is currently:

- Part structured control plane (`facts`, `actions`)
- Part unstructured screen scraping (`/screen`)
- Not a mailbox system
- Not request/response
- Not strongly ordered
- Not acknowledged

If the design keeps the OTP language, it should explicitly call this a "best-effort supervision/control plane", not a process mailbox model.

## 5. Missing Pieces

The design does not address several things that are required in practice:

- Scoped queries and reducers.
  - Root and supervisors need non-overlapping control domains.
- Agent launch spec.
  - Backend, model CLI, worktree path, branch, env vars, bootstrap sequence.
- Structured checkpointing.
  - What exact command does a worker use to persist summary, files touched, and next steps?
- Repo-write policy.
  - Who may write where, and how are conflicts prevented?
- Merge/cleanup policy for worktrees.
  - When are worktrees created, rebased, merged, deleted?
- Cancellation/drain semantics.
  - OTP `terminate/2` is not "kill the pane"; it implies orderly shutdown. The design does not define what "drain workers" means here.
- Escalation handling.
  - Root "decides" how to re-tier or reassign, but there is no proposed reducer or CLI for reassigning ownership, promoting tier, or applying a deterministic fallback policy.
- Supervisor bootstrap/query ergonomics.
  - There is no easy CLI to ask "show me only my workers/goals/facts".
- Observation retention and summarization.
  - Storing raw screen captures forever will become noisy fast.
- Acknowledged action results.
  - Today an action is either executed or failed, but there is no higher-level notion of "worker accepted instruction" or "worker completed supervisor command".
- Infrastructure outage policy.
  - If `biome_term` or SpacetimeDB is down and humans are observers only, what does the system do besides emit an event? Keep running blind, stop all writers, or enter degraded read-only mode?

## 6. Suggested Simplification

### Minimal viable version that fits in one session

Do not implement the full tree. Implement a single-domain supervision pilot with machine support where it matters:

1. Keep one root orchestrator and one domain supervisor (`nmss-supervisor`) only.
2. Add metadata-based scoping to the harness.
   - `agent.metadata_json.domain`
   - `agent.metadata_json.role`
   - `sub_goal.metadata_json.restart_policy`
3. Add `--domain <name>` to:
   - `poll-biome`
   - `decide-actions`
   - `execute-biome`
   - `run-once-biome`
4. Add agent launch metadata and backend-aware restart.
   - `backend = claude|codex`
   - `boot_cmd`
   - optional `worktree_path`
5. Support only `one_for_one` in automation.
   - Escalation after `N` restarts becomes a fact + sub-goal status update.
   - Do not automate `one_for_all` or `rest_for_one` yet.
6. Make worker set static.
   - No autonomous spawn/kill in phase one.
   - Workers are registered during deployment/startup, not dynamically by policy.
7. Enforce one write-capable worker per checkout.
   - If you need a second writer, give it a separate worktree plus an automated reconcile step.
8. Define fail-closed escalation.
   - On retry exhaustion: mark sub-goal failed/escalated, stop overlapping writers, emit observer event, and let root either reassign automatically or leave the goal blocked.

### Suggested MVP implementation plan

#### Phase A: Harness support

- Add CLI flags to pass and update `metadata_json` for agents/goals/sub-goals/facts.
- Add `--domain` filtering to biome poll/decide/execute commands.
- Add a `restart_policy` reader from metadata, but only honor `one_for_one`.
- Make `restart_agent` relaunch from per-agent metadata instead of hardcoded Claude.
- Add a root-level fallback reducer for observer-only mode.
  - Example: `escalate_sub_goal` -> `blocked | failed | retry_with_backup_agent`.

#### Phase B: Reporting/checkpoints

- Add a small reducer/CLI for worker checkpoints:
  - sub-goal status
  - completion report
  - touched paths
  - optional fact updates
- Add supervisor status fact updates:
  - `<domain>.supervisor.status`
  - `<domain>.supervisor.last_heartbeat`

#### Phase C: Pilot

- Register `nmss-supervisor` plus the existing `oracle`, `crypto`, and `hybrid` workers.
- Tag workers with `domain=nmss` and `supervisor=nmss-supervisor`.
- Root only runs `run-once-biome --domain supervisors`.
- `nmss-supervisor` runs `run-once-biome --domain nmss-workers`.
- Validate:
  - idle follow-up
  - stuck corrective prompt
  - backend-aware restart
  - escalation after repeated failure
  - fail-closed behavior when escalation is unresolved

### Recommended edits to the design doc

- Replace "full OTP-inspired supervision tree" with "hierarchical supervision pattern inspired by OTP".
- Remove claims that scoped harness execution already exists.
- Replace the schema-first migration with "metadata-first, schema later".
- Remove automatic `one_for_all` and `rest_for_one` from the MVP.
- Add an explicit repo isolation section.
- Add a structured checkpoint/report protocol section.
- Replace "alert the human" branches with explicit automated degradation/failure behavior.

## Final Recommendation

Build the control-plane primitives first, not the recursive supervisor story first.

The current harness is close enough to support:
- one domain supervisor,
- one-for-one restart,
- backend-aware relaunch,
- scoped polling/execution,
- explicit checkpoints.
- fail-closed escalation for observer-only operation.

It is not close enough yet to support:
- robust multi-domain recursive supervision,
- automatic restart trees,
- safe multi-agent concurrent writes in one checkout,
- OTP-like semantics beyond the naming analogy.
