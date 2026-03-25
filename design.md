## Core model

### Agents
One row per long-running worker.

Suggested fields:

- `name`: `agent_a`, `agent_b`, `agent_c`
- `tmux_target`: e.g. `myproject:agent_a`
- `status`: `working`, `idle`, `stuck`, `dead`, `unknown`
- `current_goal_key`: active parent orchestrator goal
- `current_sub_goal_key`: active agent sub-goal
- `last_seen_at`
- `last_capture_hash`
- `last_capture_preview`

### Goals
Top-level orchestrator-owned deliverables. These do **not** belong to an agent.

Examples:

- `orchestrator.deliver_tested_oracle`
- `orchestrator.recover_crypto`
- `orchestrator.validate_capture_path`

Useful fields:

- `goal_key`
- `title`
- `detail`
- `status`: `pending`, `active`, `blocked`, `done`, `cancelled`
- `priority`
- `depends_on_goal_key`
- `success_fact_key`
- `metadata_json`

### Sub-goals
Agent-owned steps that are derived from an orchestrator goal.

Examples:

- under `orchestrator.deliver_tested_oracle`
  - `oracle.test_oracle`
  - `oracle.index_results`
- under `orchestrator.recover_crypto`
  - `crypto.static_analysis`
  - `crypto.unicorn_emulation`
  - `crypto.identify_algorithm`
- under `orchestrator.validate_capture_path`
  - `hybrid.capture_script`
  - `hybrid.capture_test`

Useful fields:

- `sub_goal_key`
- `goal_key`
- `owner_agent`
- `title`
- `detail`
- `status`: `pending`, `active`, `blocked`, `done`, `cancelled`
- `priority`
- `depends_on_sub_goal_key`
- `success_fact_key`
- `instruction_text`: prompt to send when the agent is idle on this sub-goal
- `stuck_guidance_text`: extra recovery guidance when the agent is stuck
- `metadata_json`

### Facts
Atomic, queryable pieces of knowledge.

Examples:

- `oracle.tested = true`
- `oracle.indexed = false`
- `crypto.static_analysis_done = true`
- `crypto.unicorn_done = false`
- `crypto.algorithm_identified = true`
- `crypto.algorithm_details = "..."`
- `hybrid.capture_script_done = true`
- `hybrid.capture_tested = false`
- `hybrid.session_data = "..."`

### Observations
Raw inputs used to derive state.

Examples:

- tmux capture output
- file scan deltas
- command results

### Actions
Queued control operations.

Examples:

- `send_prompt`
- `restart_agent`
- `index_artifact`

### Artifacts
Files produced or modified under the project directory.

### Events
Append-only audit log.

---

## Dynamic goal model

Top-level goals are **not** hardcoded as required runtime state.

The harness only auto-seeds:

- agents,
- schema,
- optional example bootstraps when explicitly requested.

That means the orchestrator can interact with the harness and mutate the goal graph directly.

Supported operations:

- add a top-level goal,
- update a top-level goal,
- cancel a top-level goal,
- hard-delete a top-level goal,
- add/update/remove sub-goals under it,
- attach idle and stuck prompt text to each sub-goal.

This is the critical distinction:

- the **orchestrator** defines and mutates goals,
- the harness resolves them into agent **sub-goals**,
- the agents execute those sub-goals.

## Minimal control loop

### 1. Poll agents
For each tmux target:

- capture the last scrollback window,
- store it as an observation,
- classify the agent.

Heuristics can stay lightweight:

- **dead**: tmux capture failed
- **stuck**: visible error/exception or unchanged non-idle output for too long
- **idle**: prompt visible (`❯`)
- **working**: spinner visible or “thinking/analyzing/processing” text

### 2. Scan project artifacts
Walk the configured project directory, record new or modified files, and mark important artifacts for indexing.

### 3. Update facts
Facts can come from:

- manual input,
- agent output parsing,
- file presence,
- policy code.

### 4. Resolve orchestrator goals
Update goal state from facts and dependencies.

Examples:

- `orchestrator.deliver_tested_oracle` is done when `oracle.indexed == true`
- `orchestrator.recover_crypto` is done when `crypto.algorithm_identified == true`
- `orchestrator.validate_capture_path` is done when `hybrid.capture_tested == true`

### 5. Resolve the active sub-goal for each agent
Pick the highest-priority sub-goal for each agent whose parent goal is still active, whose dependencies are satisfied, and whose success fact is not already true.

### 6. Decide actions
Drive behavior from **agent status + active sub-goal + facts**.

Examples:

- `oracle` idle with active sub-goal `oracle.test_oracle` → send its `instruction_text`
- `crypto` idle with active sub-goal `crypto.unicorn_emulation` → prompt Unicorn use
- `crypto` stuck → append `stuck_guidance_text`
- `hybrid` idle with active sub-goal `hybrid.capture_test` → prompt capture validation
- `dead` → restart Claude and resend the current sub-goal instruction

### 7. Cross-pollinate
Use facts to bridge agents.

Examples:

- when `crypto.algorithm_details` appears, push it to `hybrid`
- when `hybrid.session_data` appears, push it to `crypto`

### 8. Index results
Queue indexing for significant new artifacts.

---

## Example orchestrator interactions

Bootstrap agents and an example workflow:

```bash
./harness seed-agents
./harness bootstrap-known-goals
```

Add a new top-level goal dynamically:

```bash
./harness goal-add \
  orchestrator.collect_more_sessions \
  "Collect more session captures" \
  --detail "Gather additional live captures for validation" \
  --priority 15 \
  --success-fact-key agent_c.more_sessions_collected
```

Attach a sub-goal to an agent:

```bash
./harness sub-goal-add \
  agent_c.collect_more_sessions \
  orchestrator.collect_more_sessions \
  agent_c \
  "Capture more sessions" \
  --instruction-text "Collect three fresh session captures and save the artifacts." \
  --stuck-guidance-text "If a dependency is unavailable, stub it and validate the path." \
  --success-fact-key agent_c.more_sessions_collected \
  --priority 10
```

Cancel a goal without deleting history:

```bash
./harness goal-remove orchestrator.collect_more_sessions
```

Hard-delete a goal and its sub-goals:

```bash
./harness goal-remove orchestrator.collect_more_sessions --delete --cascade
```

Run the loop once:

```bash
./harness run-once-biome --execute
```

---

## Practical consequence

So, to your question directly: **yes, top-level goals should be dynamic**.

In the revised harness:

- they are addable and removable at runtime,
- they stem from orchestrator interaction rather than being agent-owned defaults,
- sub-goals inherit from those orchestrator goals,
- the scheduler resolves agent work from the current goal graph in the database.
