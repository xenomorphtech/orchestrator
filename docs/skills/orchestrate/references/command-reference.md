# Command Reference

Use these commands when executing the orchestration workflow.

## Recover Context

```bash
/home/sdancer/orchestrator/harness episodes --limit 5
/home/sdancer/orchestrator/harness agents
```

## Discover Panes

```bash
curl -s http://localhost:3021/panes
curl -s http://localhost:3021/panes/<id>/screen
```

Cross-reference harness agents with biome_term panes. Any pane whose name or id is not registered in harness is unmanaged.

## Poll Services

```bash
/home/sdancer/orchestrator/harness poll-services
```

## Run One Cycle

```bash
/home/sdancer/orchestrator/harness run-once-biome --execute
```

## Nudge or Redirect an Agent

```bash
/home/sdancer/orchestrator/harness send <name-or-id> "Continue."
/home/sdancer/orchestrator/harness send <name-or-id> "Continue. Fix the build error."
```

Use short follow-ups. Prefer continuing the agent's own stated next step.

## Spawn a Claude Agent

```bash
curl -s -X POST http://localhost:3021/panes \
  -H 'Content-Type: application/json' \
  -d '{"name":"my-agent","cols":220,"rows":50}'

/home/sdancer/orchestrator/harness send my-agent \
  "cd /path/to/project && claude --dangerously-skip-permissions"

/home/sdancer/orchestrator/harness agent-add my-agent \
  --biome-pane-id <uuid> --workdir /path/to/project \
  --default-task "Continue the task"
```

## Spawn a Codex Agent

```bash
curl -s -X POST http://localhost:3021/panes \
  -H 'Content-Type: application/json' \
  -d '{"name":"codex-agent","cols":220,"rows":50}'

/home/sdancer/orchestrator/harness send codex-agent \
  'cd /path/to/project && codex --dangerously-bypass-approvals-and-sandbox "your task"'

/home/sdancer/orchestrator/harness agent-add codex-agent \
  --biome-pane-id <uuid> --workdir /path/to/project \
  --default-task "Continue the codex task"
```

## Handle Unmanaged Panes

Report unmanaged panes by name, id, and classified status.

If useful, register one:

```bash
/home/sdancer/orchestrator/harness agent-add <name> \
  --biome-pane-id <uuid> --workdir <path> \
  --default-task "<task description>"
```

Generic fallback nudge:

```bash
/home/sdancer/orchestrator/harness send <name-or-id> \
  "Continue from where you left off. If stuck on an error, try a different approach."
```

## Cross-Pollinate Findings

```bash
/home/sdancer/orchestrator/harness fact-set <key> <value>
```

Index significant new files with `mcp__openviking__add_resource` when that tool is available in the current environment.

## Record the Cycle

```bash
/home/sdancer/orchestrator/harness episode-add \
  "<1-2 sentence cycle summary>" \
  --agent-statuses '{"agent1":"working","agent2":"idle"}' \
  --actions-taken '["action1","action2"]' \
  --goal-progress '{"goal1":"active - subgoal in progress","goal2":"done"}'
```

Update rolling descriptions when an agent's context changed:

```bash
/home/sdancer/orchestrator/harness agent-describe <name> \
  "Working on: <current task>. Done: <key results>. Next: <planned steps>."
```
