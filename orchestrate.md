# Orchestrator

Monitor and drive Claude or Codex agents running in biome_term panes, working toward goals managed by the harness.

**Notes:**
- When spawning Codex agents, always use `--dangerously-bypass-approvals-and-sandbox` (not `--full-auto` or `--approval-mode`).
- Codex agents often need to be nudged to continue — if idle, send a follow-up prompt. They tend to stop and wait more than Claude agents.

## Config

- Harness: `python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db`
- biome_term: `http://localhost:3000`

If `$ARGUMENTS` is provided, treat it as the harness DB path instead of the default.

## Steps

1. **Discover agents** from both harness DB and biome_term.

   ```bash
   # Harness-registered agents
   python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db agents

   # All biome_term panes
   curl -s http://localhost:3000/panes
   ```

   Cross-reference: any biome_term pane whose `name` or `id` does not appear in harness output is "unmanaged". Note these separately.

2. **Capture and classify each agent** by reading its screen via biome_term:

   ```bash
   curl -s http://localhost:3000/panes/<id>/screen
   ```

   Classify from the `rows[]` array:
   - **dead**: pane has `terminated: true`, or HTTP 404
   - **stuck**: last 20 rows contain error patterns (traceback, exception, "error:", permission denied, segmentation fault, command not found)
   - **idle**: last non-empty row starts with or ends with prompt char (`>`)
   - **working**: spinner characters visible or keywords (thinking, analyzing, processing, working, running)
   - **stuck (stale)**: no output change for 10+ minutes (compare to previous cycle)

   **Auto-continue idle agents**: When an idle agent's last output describes a clear next step or task (e.g. "The next useful move is to...", "Next step is to...", "I'm going to..."), send a short continuation nudge like `Continue.` or `Continue. <one-line summary of their stated next step>` — do NOT re-explain what they already said. Only add cross-pollination context if another agent produced a finding that changes their plan. If the agent's stated next step was already superseded by another agent's work (e.g. they say "search donor logs" but another agent already did that and found nothing), redirect them instead.

   **Context and compaction**: Both Claude Code and Codex (GPT-5.4) agents have automatic context compaction. Low context % does NOT mean the agent is exhausted — compaction will reclaim space and the agent will keep working. Always nudge idle agents regardless of context %, as long as they have remaining work. Do not skip agents just because they show low context.

   **Context refresh for low-context agents** (use sparingly — compaction usually handles this): When an agent is idle and context is low (roughly ≤20%), but they still have productive work to do:
   1. Send: `Summarize your current goal, what you've accomplished, and the exact next 2-3 tasks to continue. Be concise.`
   2. Wait for the agent to reply with its summary.
   3. Read the summary from the screen.
   4. Send `/clear` to reset the agent's context window.
   5. After the clear, inject the goal and tasks back:
      ```
      You are continuing work on: <goal from summary>
      Accomplished so far: <key accomplishments from summary>
      Your next tasks:
      1. <task 1 from summary>
      2. <task 2 from summary>
      Continue with task 1.
      ```
   This gives the agent a fresh context window while preserving continuity. Only do this for agents that have clear remaining work — don't refresh agents that are effectively done.

3. **Run the harness cycle** to poll, resolve goals, decide and execute actions:

   ```bash
   python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db run-once --execute
   ```

   This handles registered agents automatically: sends follow-up prompts to idle agents, corrective prompts to stuck agents, restarts dead agents, cross-pollinates facts, and queues artifact indexing.

4. **Handle unmanaged panes** discovered in step 1:
   - Report their name, id, and classified status
   - If stuck or dead, suggest registering them:
     ```bash
     python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db agent-add <name> \
       --biome-pane-id <uuid> --workdir <path> --default-task "<task description>"
     ```
   - Optionally send a generic nudge to stuck unmanaged panes:
     ```bash
     B64=$(printf 'Continue from where you left off. If stuck on an error, try a different approach.\r' | base64 -w0)
     curl -s -X POST http://localhost:3000/panes/<id>/input \
       -H 'Content-Type: application/json' -d "{\"data\":\"$B64\"}"
     ```

5. **Cross-pollinate and index**:
   - Read the screen captures from step 2. If any agent has produced a significant finding (completed a task, generated output, found a result), share it with related agents by setting facts:
     ```bash
     python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db fact-set <key> <value>
     ```
   - Index significant new files with `mcp__openviking__add_resource`.

6. **Report** a brief status summary covering all agents (managed and unmanaged), actions taken this cycle, goal progress, and any alerts requiring manual attention.

## API Reference

### Spawn a Claude agent in biome_term

```bash
# Create pane
curl -s -X POST http://localhost:3000/panes \
  -H 'Content-Type: application/json' \
  -d '{"name":"my-agent","cols":220,"rows":50}'
# Response: {"id":"<uuid>","name":"my-agent","cols":220,"rows":50}

# Start Claude (replace <uuid> with the id from above)
printf 'cd /path/to/project && claude --dangerously-skip-permissions\n' | base64
# Then send:
curl -s -X POST http://localhost:3000/panes/<uuid>/input \
  -H 'Content-Type: application/json' -d '{"data":"<base64>"}'

# Wait ~5s for Claude to initialize, then send task
printf 'Your task prompt here\n' | base64
curl -s -X POST http://localhost:3000/panes/<uuid>/input \
  -H 'Content-Type: application/json' -d '{"data":"<base64>"}'

# Register with harness
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db agent-add my-agent \
  --biome-pane-id <uuid> --workdir /path/to/project \
  --default-task "Continue the task"
```

### Spawn a Codex agent in biome_term

```bash
curl -s -X POST http://localhost:3000/panes \
  -H 'Content-Type: application/json' \
  -d '{"name":"codex-agent","cols":220,"rows":50}'

printf 'cd /path/to/project && codex --dangerously-bypass-approvals-and-sandbox "your task"\n' | base64
curl -s -X POST http://localhost:3000/panes/<uuid>/input \
  -H 'Content-Type: application/json' -d '{"data":"<base64>"}'

python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db agent-add codex-agent \
  --biome-pane-id <uuid> --workdir /path/to/project \
  --default-task "Continue the codex task"
```

### Monitor a pane

```bash
# List all panes
curl -s http://localhost:3000/panes

# Get current screen (VT100-rendered rows)
curl -s http://localhost:3000/panes/<uuid>/screen

# Get event log since sequence N
curl -s "http://localhost:3000/panes/<uuid>/events?after=0"

# Kill a pane
curl -s -X DELETE http://localhost:3000/panes/<uuid>
```

### Send input to a pane

Use the helper script — it resolves names, appends `\r`, and base64-encodes automatically:

```bash
/home/sdancer/orchestrate/send.sh <pane-name-or-id> "your prompt here"

# Examples:
/home/sdancer/orchestrate/send.sh native_harness "Continue."
/home/sdancer/orchestrate/send.sh aion2-protocol "Continue. Fix the decode regression."
/home/sdancer/orchestrate/send.sh ecf38525 "Continue."
```

Manual equivalent (if send.sh is unavailable):
```bash
B64=$(printf 'your prompt here\r' | base64 -w0)
curl -s -X POST http://localhost:3000/panes/<uuid>/input \
  -H 'Content-Type: application/json' -d "{\"data\":\"$B64\"}"
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
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db goal-add \
  <goal_key> "<title>" --priority 10 --success-fact-key <fact_key>

# Add a sub-goal assigned to an agent
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db sub-goal-add \
  <sub_goal_key> <goal_key> <agent_name> "<title>" \
  --instruction-text "<prompt when idle>" \
  --stuck-guidance-text "<prompt when stuck>" \
  --success-fact-key <fact_key> --priority 10

# Set a fact
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db fact-set <key> <value>

# View full state
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db summary --verbose

# Register an agent
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db agent-add <name> \
  --biome-pane-id <uuid> --workdir <path> --default-task "<text>"

# Deregister an agent
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db agent-remove <name>

# Cancel a goal
python /home/sdancer/orchestrate/harness.py --db /home/sdancer/orchestrate/orchestrate.db goal-remove <goal_key>
```
