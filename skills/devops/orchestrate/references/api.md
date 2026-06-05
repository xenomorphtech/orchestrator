# API reference — `harness` CLI + spawn patterns

Default binary: `/home/sdancer/orchestrator/harness`. Default biome_term endpoint: `http://localhost:3021`. `$ARGUMENTS` if provided overrides the harness server/database default.

## Monitor a pane

```bash
/home/sdancer/orchestrator/harness panes
/home/sdancer/orchestrator/harness screen <name-or-id>
/home/sdancer/orchestrator/harness screen <name-or-id> --lines 30
curl -s -H "X-API-Key: $HARNESS_BIOME_API_KEY" "http://localhost:3021/panes/<uuid>/events?after=0"
curl -s -H "X-API-Key: $HARNESS_BIOME_API_KEY" -X DELETE http://localhost:3021/panes/<uuid>
```

## Send input to a pane

```bash
/home/sdancer/orchestrator/harness send <name-or-id> "your prompt"
/home/sdancer/orchestrator/harness send --delay 300 <name-or-id> "long prompt"   # custom CR delay
```

## Goals, paths, facts, agents

```bash
# Goals
/home/sdancer/orchestrator/harness goal-add <key> "<title>" --priority 10 --success-fact-key <fact>
/home/sdancer/orchestrator/harness sub-goal-add <sg_key> <goal_key> <agent> "<title>" \
  --instruction-text "<prompt when idle>" \
  --stuck-guidance-text "<prompt when stuck>" \
  --success-fact-key <fact> --priority 10
/home/sdancer/orchestrator/harness fact-set <key> <value>
/home/sdancer/orchestrator/harness goal-remove <key>
/home/sdancer/orchestrator/harness summary

# Portfolio
/home/sdancer/orchestrator/harness path-list --json
/home/sdancer/orchestrator/harness path-add <name> --goal <key> --worktree <path> --hypothesis "..." --falsification "..." --status <active|backlog>
/home/sdancer/orchestrator/harness path-set <name> --stall-counter N --last-metric-move-at <iso> --status <active|stalled|falsified|done|path-dropped|stalled-meta>
/home/sdancer/orchestrator/harness path-remove <name>

# Agents
/home/sdancer/orchestrator/harness agent-list --json
/home/sdancer/orchestrator/harness agent-get <name> --json
/home/sdancer/orchestrator/harness agent-add <name> --kind {biome_term|codex_app_server} --workdir <path> [--default-task "..."] [--biome-pane-id <uuid>]
/home/sdancer/orchestrator/harness agent-remove <name>
/home/sdancer/orchestrator/harness agent-describe <name> "<2-3 sentence rolling description>"

/home/sdancer/orchestrator/harness facts
```

## Briefings

```bash
/home/sdancer/orchestrator/harness briefing-set <name> --body-file briefings/<name>.md --goal <key> --category <cat> --tags <csv>
/home/sdancer/orchestrator/harness briefing-set-meta <name> --goal <k> --category <c> --tags <csv>
/home/sdancer/orchestrator/harness briefing-get <name> [--materialize]
/home/sdancer/orchestrator/harness briefing-list [--archived|--only-archived] [--goal k] [--category c]
/home/sdancer/orchestrator/harness briefing-archive <name> [--restore]
/home/sdancer/orchestrator/harness briefing-archive-unused   # housekeeping: every K=6 cycles
```

## Service health

```bash
/home/sdancer/orchestrator/harness services
/home/sdancer/orchestrator/harness service-add <name> \
  --service-type {systemd|http|tcp|ssh_systemd} --check-target <target> \
  --restart-policy {auto|manual}
/home/sdancer/orchestrator/harness poll-services [--timeout-ms 10000]
/home/sdancer/orchestrator/harness service-remove <name> [--delete]
```

## Episodic memory

```bash
/home/sdancer/orchestrator/harness episodes --limit 5
/home/sdancer/orchestrator/harness episode-add "<summary>" \
  --agent-statuses '<json>' --actions-taken '<json>' --goal-progress '<json>'
/home/sdancer/orchestrator/harness agent-describe <name> "<description>"
```

## Spawning workers

### Codex agent — PREFERRED (`codex_app_server` kind, no pane)

JSON-RPC over stdio. Durable thread state lives server-side and persists across `harness send` invocations via the `codex_thread_id` recorded in `agents.metadata_json`. No tmux pane, no paste-queue races, no init-prompt loss. **Default for all new Codex agents.**

```bash
# 1. Write briefing first.
# 2. Register with codex_app_server kind. Workdir must exist.
/home/sdancer/orchestrator/harness agent-add <name> \
  --kind codex_app_server \
  --workdir /path/to/worktree \
  --default-task "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."

# 3. Drive a turn — first send runs initialize -> thread/start -> turn/start.
/home/sdancer/orchestrator/harness send <name> \
  "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
```

Subsequent sends reuse the thread (continuity). Useful flags on `send`:
- `--wait` (default true): block until `turn/completed`.
- `--no-wait`: fire-and-forget — return after `turn/start`.
- `--timeout SECS` (default 300).
- `--follow`: stream events to stdout while waiting.

Per-send Codex events land in `analysis/codex-sessions/<agent>/<turn-id>/`. Full operator doc: `/home/sdancer/orchestrator/codex-app-server-mode.md`.

### Claude agent (when pane visibility is required)

```bash
# 1. Write briefing first.
# 2. Create pane.
curl -s -X POST http://localhost:3021/panes \
  -H 'Content-Type: application/json' -H "X-API-Key: $HARNESS_BIOME_API_KEY" \
  -d '{"name":"<name>","cols":220,"rows":50}'

# 3. Start Claude.
/home/sdancer/orchestrator/harness send <name> \
  "cd /path/to/worktree && claude --dangerously-skip-permissions"

# 4. ~5s later, point at the briefing.
/home/sdancer/orchestrator/harness send <name> \
  "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."

# 5. Register.
/home/sdancer/orchestrator/harness agent-add <name> \
  --biome-pane-id <uuid> --workdir /path/to/worktree \
  --default-task "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
```

### Codex agent in pane — LEGACY (only when pane visibility is required)

Same shape as Claude pane, but launch Codex with `--dangerously-bypass-approvals-and-sandbox`. NOT the default — use the `codex_app_server` kind unless you have a specific reason to want the pane.

### Worktree creation

```bash
git -C <repo> worktree add <path> -b <branch>             # named branch
git -C <repo> worktree add <path> <commit-sha>            # detached HEAD at a specific commit
git -C <repo> worktree remove <path>                      # when retiring a path
```

## Anthropic SDK (programmatic, non-pane)

```python
import anthropic
client = anthropic.Anthropic()
message = client.messages.create(
    model="claude-sonnet-4-6",
    max_tokens=8192,
    messages=[{"role": "user", "content": "your task"}],
)
print(message.content[0].text)
```
