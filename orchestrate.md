# Orchestrator

Monitor and drive Claude or Codex agents running in biome_term panes, working toward goals managed by the Rust `harness`.

**Notes:**
- When spawning Codex agents, always use `--dangerously-bypass-approvals-and-sandbox` (not `--full-auto` or `--approval-mode`).
- Codex agents often need to be nudged to continue — if idle, send a follow-up prompt. They tend to stop and wait more than Claude agents.
- Codex always shows a `›` prompt at the bottom of the screen even while working — do NOT use the prompt character alone to detect idle state. Always check for `Working (` in the rows above the prompt first, and check `idle_seconds` from the panes API.
- **Workers always start from a briefing `.md`.** Any time you spawn a new agent or restart one whose context was lost (fresh shell, `/clear`, pane recreated), you MUST first write/refresh `/home/sdancer/orchestrator/briefings/<agent>.md` with enough context for the worker to continue without re-deriving state. The first prompt sent to the worker should instruct it to read that file before doing anything else.

## Config

- Harness: `/home/sdancer/orchestrator/harness`
- Filesystem scan helper: `python /home/sdancer/orchestrator/fs-check.py --db /home/sdancer/orchestrator/fs-check.db`
- biome_term: `http://localhost:3021`

If `$ARGUMENTS` is provided, treat it as the harness server or database override instead of the default.

## Worker briefings

Every worker agent is paired with a briefing file at `/home/sdancer/orchestrator/briefings/<agent>.md`. This file is the worker's source of truth when it starts fresh or after any context reset.

A briefing MUST contain:

1. **Role & workdir** — one sentence on what this agent owns and the absolute path it runs in.
2. **Current goal / sub-goal** — the harness goal_key and sub_goal_key the agent is assigned to, with the title.
3. **Success criteria** — the fact key(s) that, when set, complete the goal, and what "done" looks like concretely.
4. **Progress so far** — bullets summarising prior cycles: what has been tried, what worked, what failed, what artifacts/files were produced (with paths).
5. **Next 2–3 concrete tasks** — ordered, specific, actionable. No vague "keep going".
6. **Constraints & gotchas** — device assignments, non-obvious rules from memory, things the agent must NOT do, known pitfalls from prior episodes.
7. **Relevant files / references** — paths, URLs, fact keys the agent should consult.

Keep briefings under ~150 lines. They are a working document: rewrite (not append) each time you refresh one, pulling the latest state from harness episodes, agent rolling descriptions, and recent screen captures.

**When to write or refresh a briefing:**
- Before spawning a new agent (step in API Reference below).
- Before restarting a dead/stuck agent via `restart_agent` or manually via `harness send`.
- Before sending `/clear` as part of the low-context refresh flow in step 2.
- Any cycle where cross-pollination materially changes the plan and the agent will need that context after its next compaction.

**How workers consume the briefing:** the first message sent after boot must be exactly:

```
Read /home/sdancer/orchestrator/briefings/<agent>.md — that is your full briefing. Then continue with task 1.
```

Do not paste the briefing contents inline — keep it as a file the worker reads, so the briefing can be updated independently and the worker can re-read it after compaction.

## Steps

0. **Recover context** from prior cycles — read recent episodic memory and agent rolling descriptions:

   ```bash
   # Recent cycle episodes (last 5)
   /home/sdancer/orchestrator/harness episodes --limit 5

   # Agent list with rolling descriptions
   /home/sdancer/orchestrator/harness agents
   ```

   Use episodes to understand what happened in previous cycles: what agents accomplished, what strategy was being pursued, and whether any patterns (repeated restarts, persistent stuck states) need attention. Use rolling descriptions to understand each agent's current narrative without relying on session memory.

   **On fresh start (no episodes):** proceed normally — the remaining steps will establish context.
   **On restart (episodes exist):** use episode history to avoid restarting tasks that were already in progress or completed. Check if agents are still working on what the episodes describe before sending new tasks.

1. **Discover agents** from both harness DB and biome_term.

   ```bash
   # Harness-registered agents
   /home/sdancer/orchestrator/harness agents

   # All biome_term panes
   /home/sdancer/orchestrator/harness panes
   ```

   Cross-reference: any biome_term pane whose `name` or `id` does not appear in harness output is "unmanaged". Note these separately.

2. **Capture and classify each agent** by reading its screen via the harness:

   ```bash
   /home/sdancer/orchestrator/harness screen <pane-name-or-id>
   # Or limit to last N lines:
   /home/sdancer/orchestrator/harness screen <pane-name-or-id> --lines 30
   ```

   Classify from the `rows[]` array and pane metadata:
   - **dead**: pane has `terminated: true`, or HTTP 404
   - **working**: `idle_seconds == 0` from the panes API, OR the string "Working (" appears anywhere in the last 20 rows (Codex working indicator), OR spinner characters / keywords (thinking, analyzing, processing, Hatching, running) appear in the last 20 rows
   - **stuck**: last 20 rows contain error patterns (traceback, exception, "error:", permission denied, segmentation fault, command not found) AND agent is NOT also showing working indicators
   - **idle**: `idle_seconds > 0` AND last non-empty row starts with or ends with prompt char (`❯` or `›`) AND no "Working" keyword in last 20 rows
   - **stuck (stale)**: no output change for 10+ minutes (compare to previous cycle)

   **Auto-continue idle agents**: When an idle agent's last output describes a clear next step or task (e.g. "The next useful move is to...", "Next step is to...", "I'm going to..."), send a short continuation nudge like `Continue.` or `Continue. <one-line summary of their stated next step>` — do NOT re-explain what they already said. Only add cross-pollination context if another agent produced a finding that changes their plan. If the agent's stated next step was already superseded by another agent's work (e.g. they say "search donor logs" but another agent already did that and found nothing), redirect them instead.

   **Context and compaction**: Both Claude Code and Codex (GPT-5.4) agents have automatic context compaction. Low context % does NOT mean the agent is exhausted — compaction will reclaim space and the agent will keep working. Always nudge idle agents regardless of context %, as long as they have remaining work. Do not skip agents just because they show low context.

   **Context refresh for low-context agents** (use sparingly — compaction usually handles this): When an agent is idle and context is low (roughly ≤20%), but they still have productive work to do:
   1. Send: `Summarize your current goal, what you've accomplished, and the exact next 2-3 tasks to continue. Be concise.`
   2. Wait for the agent to reply with its summary.
   3. Read the summary from the screen.
   4. Rewrite `/home/sdancer/orchestrator/briefings/<agent>.md` using the worker's summary plus the latest episode/fact context (see "Worker briefings" above).
   5. Send `/clear` to reset the agent's context window.
   6. After the clear, send the single briefing-pointer prompt:
      ```
      Read /home/sdancer/orchestrator/briefings/<agent>.md — that is your full briefing. Then continue with task 1.
      ```
   Never paste the summary or task list inline — always drive the worker off the briefing file so a later restart can reuse the same document. Only refresh agents that have clear remaining work; don't refresh agents that are effectively done.

3. **Poll service health** to check systemd units, HTTP endpoints, and TCP ports:

   ```bash
   /home/sdancer/orchestrator/harness poll-services
   ```

   This checks all registered services and records health status. Services with `restart_policy=auto` and 3+ consecutive failures will have a `restart_service` action queued automatically.

4. **Run the harness cycle** to poll, resolve goals, decide and execute actions:

   ```bash
   /home/sdancer/orchestrator/harness run-once-biome --execute
   ```

   This handles registered agents automatically: sends follow-up prompts to idle agents, corrective prompts to stuck agents, restarts dead agents, cross-pollinates facts, and queues artifact indexing.

   **Before** running this with `--execute`, check whether any dead agents are likely to be restarted this cycle. For each such agent, refresh `/home/sdancer/orchestrator/briefings/<agent>.md` first, and make sure the agent's `default_task` in the harness is the briefing-pointer prompt (see "Worker briefings"). The harness reuses `default_task` as the post-boot prompt on restart — if that still points at the briefing file, the restarted worker will pick up full context automatically.

5. **Handle unmanaged panes** discovered in step 1:
   - Report their name, id, and classified status
   - If stuck or dead, suggest registering them:
     ```bash
     /home/sdancer/orchestrator/harness agent-add <name> \
       --biome-pane-id <uuid> --workdir <path> --default-task "<task description>"
     ```
   - Optionally send a generic nudge to stuck unmanaged panes:
     ```bash
     /home/sdancer/orchestrator/harness send <name-or-id> "Continue from where you left off. If stuck on an error, try a different approach."
     ```

6. **Cross-pollinate and index**:
   - Read the screen captures from step 2. If any agent has produced a significant finding (completed a task, generated output, found a result), share it with related agents by setting facts:
     ```bash
     /home/sdancer/orchestrator/harness fact-set <key> <value>
     ```
   - Index significant new files with `mcp__openviking__add_resource`.

7. **Report** a brief status summary covering all agents (managed and unmanaged), service health status, actions taken this cycle, goal progress, and any alerts requiring manual attention. Include any services with `unhealthy` or `degraded` status.

8. **Record episode** — push a structured summary of this cycle to SpacetimeDB for continuity across restarts:

   ```bash
   /home/sdancer/orchestrator/harness episode-add \
     "<1-2 sentence cycle summary>" \
     --agent-statuses '{"agent1":"working","agent2":"idle"}' \
     --actions-taken '["action1","action2"]' \
     --goal-progress '{"goal1":"active - subgoal in progress","goal2":"done"}'
   ```

   Update rolling descriptions for agents whose context changed this cycle:

   ```bash
   /home/sdancer/orchestrator/harness agent-describe <name> \
     "Working on: <current task>. Done: <key results>. Next: <planned steps>."
   ```

   Keep descriptions concise (2-3 sentences) but specific enough that a fresh orchestrator session can continue without re-reading screens.

## API Reference

### Spawn a Claude agent in biome_term

```bash
# 1. Write the briefing FIRST (see "Worker briefings" for required content).
#    Without this, the worker boots blind.
$EDITOR /home/sdancer/orchestrator/briefings/my-agent.md

# 2. Create pane (raw biome_term API — harness send will auto-resolve the name)
curl -s -X POST http://localhost:3021/panes \
  -H 'Content-Type: application/json' -H "X-API-Key: $HARNESS_BIOME_API_KEY" \
  -d '{"name":"my-agent","cols":220,"rows":50}'
# Response: {"id":"<uuid>","name":"my-agent","cols":220,"rows":50}

# 3. Start Claude
/home/sdancer/orchestrator/harness send my-agent "cd /path/to/project && claude --dangerously-skip-permissions"

# 4. Wait ~5s for Claude to initialize, then point it at the briefing.
#    Use this exact phrasing so restarts stay consistent.
/home/sdancer/orchestrator/harness send my-agent \
  "Read /home/sdancer/orchestrator/briefings/my-agent.md — that is your full briefing. Then continue with task 1."

# 5. Register with harness. The default_task MUST be the briefing-pointer
#    prompt so that automatic restarts re-seed context from the same file.
/home/sdancer/orchestrator/harness agent-add my-agent \
  --biome-pane-id <uuid> --workdir /path/to/project \
  --default-task "Read /home/sdancer/orchestrator/briefings/my-agent.md — that is your full briefing. Then continue with task 1."
```

### Spawn a Codex agent in biome_term

```bash
# 1. Write briefing first: /home/sdancer/orchestrator/briefings/codex-agent.md

curl -s -X POST http://localhost:3021/panes \
  -H 'Content-Type: application/json' -H "X-API-Key: $HARNESS_BIOME_API_KEY" \
  -d '{"name":"codex-agent","cols":220,"rows":50}'

# 2. Launch codex with the briefing-pointer as its initial task.
/home/sdancer/orchestrator/harness send codex-agent \
  'cd /path/to/project && codex --dangerously-bypass-approvals-and-sandbox "Read /home/sdancer/orchestrator/briefings/codex-agent.md — that is your full briefing. Then continue with task 1."'

# 3. Register with matching default_task so restarts re-use the briefing.
/home/sdancer/orchestrator/harness agent-add codex-agent \
  --biome-pane-id <uuid> --workdir /path/to/project \
  --default-task "Read /home/sdancer/orchestrator/briefings/codex-agent.md — that is your full briefing. Then continue with task 1."
```

### Monitor a pane

```bash
# List all panes
/home/sdancer/orchestrator/harness panes

# Get current screen (full)
/home/sdancer/orchestrator/harness screen <pane-name-or-id>

# Get last N lines of screen
/home/sdancer/orchestrator/harness screen <pane-name-or-id> --lines 30

# Get event log since sequence N (raw biome_term API)
curl -s -H "X-API-Key: $HARNESS_BIOME_API_KEY" "http://localhost:3021/panes/<uuid>/events?after=0"

# Kill a pane (raw biome_term API)
curl -s -H "X-API-Key: $HARNESS_BIOME_API_KEY" -X DELETE http://localhost:3021/panes/<uuid>
```

### Send input to a pane

Use `harness send` — it resolves pane names, sends the text, waits briefly, then sends the carriage return:

```bash
/home/sdancer/orchestrator/harness send <pane-name-or-id> "your prompt here"

# Examples:
/home/sdancer/orchestrator/harness send vampir-api "Continue."
/home/sdancer/orchestrator/harness send g4-scaffold "Continue. Fix the build error."
/home/sdancer/orchestrator/harness send 3d94116f "Continue."

# Custom delay before carriage return (default 150ms):
/home/sdancer/orchestrator/harness send --delay 300 vampir-api "long prompt here"
```

### Anthropic SDK (programmatic, non-pane)

```python
import anthropic

client = anthropic.Anthropic()
message = client.messages.create(
    model="claude-sonnet-4-20250514",
    max_tokens=8192,
    messages=[{"role": "user", "content": "your task"}],
)
print(message.content[0].text)
```

### Harness goal management

```bash
# Add a top-level goal
/home/sdancer/orchestrator/harness goal-add \
  <goal_key> "<title>" --priority 10 --success-fact-key <fact_key>

# Add a sub-goal assigned to an agent
/home/sdancer/orchestrator/harness sub-goal-add \
  <sub_goal_key> <goal_key> <agent_name> "<title>" \
  --instruction-text "<prompt when idle>" \
  --stuck-guidance-text "<prompt when stuck>" \
  --success-fact-key <fact_key> --priority 10

# Set a fact
/home/sdancer/orchestrator/harness fact-set <key> <value>

# View full state
/home/sdancer/orchestrator/harness summary

# Register an agent
/home/sdancer/orchestrator/harness agent-add <name> \
  --biome-pane-id <uuid> --workdir <path> --default-task "<text>"

# Deregister an agent
/home/sdancer/orchestrator/harness agent-remove <name>

# Cancel a goal
/home/sdancer/orchestrator/harness goal-remove <goal_key>
```

### Service health monitoring

```bash
# List all registered services
/home/sdancer/orchestrator/harness services

# Register a systemd service
/home/sdancer/orchestrator/harness service-add spacetimedb \
  --service-type systemd --check-target spacetimedb.service \
  --restart-policy auto

# Register an HTTP endpoint
/home/sdancer/orchestrator/harness service-add biome-term \
  --service-type http --check-target http://localhost:3021/panes \
  --restart-policy manual

# Register a TCP port check
/home/sdancer/orchestrator/harness service-add stdb-port \
  --service-type tcp --check-target 127.0.0.1:3000

# Register a remote systemd service via SSH
/home/sdancer/orchestrator/harness service-add remote-nginx \
  --service-type ssh_systemd --check-target nginx.service \
  --host remote-server --restart-policy auto

# Poll all services (check health)
/home/sdancer/orchestrator/harness poll-services

# Poll with custom timeout
/home/sdancer/orchestrator/harness poll-services --timeout-ms 10000

# Remove a service (soft — sets status to unknown)
/home/sdancer/orchestrator/harness service-remove spacetimedb

# Remove a service (hard — deletes service and health records)
/home/sdancer/orchestrator/harness service-remove spacetimedb --delete
```

### Episodic memory and descriptions

```bash
# Query recent cycle episodes
/home/sdancer/orchestrator/harness episodes --limit 5

# Record a cycle episode
/home/sdancer/orchestrator/harness episode-add "<summary>" \
  --agent-statuses '<json>' --actions-taken '<json>' --goal-progress '<json>'

# Update agent rolling description
/home/sdancer/orchestrator/harness agent-describe <name> "<description>"
```
