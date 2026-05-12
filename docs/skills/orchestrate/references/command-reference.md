# Command Reference

Commands grouped by control-cycle phase. Order within each phase matches typical issue order.

## SENSE — observe without writing

```bash
adb connect localhost:5558
/home/sdancer/orchestrator/harness episodes --limit 5
/home/sdancer/orchestrator/harness agents
/home/sdancer/orchestrator/harness panes
/home/sdancer/orchestrator/harness poll-services
/home/sdancer/orchestrator/harness screen <name-or-id> --lines 30
curl -s -H "X-API-Key: $HARNESS_BIOME_API_KEY" "http://localhost:3021/panes/<uuid>/events?after=0"
```

Read ledgers:

```bash
cat /home/sdancer/orchestrator/analysis/paths.json
cat /home/sdancer/orchestrator/analysis/hypotheses.md
cat /home/sdancer/orchestrator/analysis/falsified.md
```

Cross-reference harness agents with biome_term panes. Any pane name/id not registered in harness is **unmanaged**.

## ACTUATE — briefing first, then worktree, then agent, then cross-pollination

### 1. Rewrite briefing (always before spawn/restart/clear-refresh)

```bash
$EDITOR /home/sdancer/orchestrator/briefings/<agent>.md
```

### 2. Worktree ops

```bash
git -C <repo> worktree add <path> -b <branch>
git -C <repo> worktree add <path> <commit-sha>      # detached HEAD
git -C <repo> worktree remove <path>                # when retiring a path
```

### 3. Spawn a Codex agent (preferred — `codex_app_server` kind, no pane)

```bash
/home/sdancer/orchestrator/harness agent-add <name> \
  --kind codex_app_server \
  --workdir <worktree-path> \
  --default-task "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."

/home/sdancer/orchestrator/harness send <name> \
  "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
```

Useful `send` flags for `codex_app_server`: `--wait` (default), `--no-wait`, `--timeout SECS`, `--follow`.

### 4. Spawn a Claude agent (pane visibility required)

```bash
curl -s -X POST http://localhost:3021/panes \
  -H 'Content-Type: application/json' -H "X-API-Key: $HARNESS_BIOME_API_KEY" \
  -d '{"name":"<name>","cols":220,"rows":50}'

/home/sdancer/orchestrator/harness send <name> \
  "cd <worktree-path> && claude --dangerously-skip-permissions"

# ~5s wait, then point at briefing
/home/sdancer/orchestrator/harness send <name> \
  "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."

/home/sdancer/orchestrator/harness agent-add <name> \
  --biome-pane-id <uuid> --workdir <worktree-path> \
  --default-task "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
```

### 5. Spawn a Codex agent in pane (LEGACY — only when pane is required)

```bash
curl -s -X POST http://localhost:3021/panes \
  -H 'Content-Type: application/json' -H "X-API-Key: $HARNESS_BIOME_API_KEY" \
  -d '{"name":"<name>","cols":220,"rows":50}'

/home/sdancer/orchestrator/harness send <name> \
  'cd <worktree-path> && codex --dangerously-bypass-approvals-and-sandbox "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."'

/home/sdancer/orchestrator/harness agent-add <name> \
  --biome-pane-id <uuid> --workdir <worktree-path> \
  --default-task "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
```

### 6. Nudge / redirect a worker

```bash
/home/sdancer/orchestrator/harness send <name> "Continue."
/home/sdancer/orchestrator/harness send <name> "Continue. <one-line summary of their own stated next step>"
/home/sdancer/orchestrator/harness send --delay 300 <name> "long prompt that needs CR delay tuning"
```

### 7. Context refresh (sparingly, low-context idle workers only)

```bash
/home/sdancer/orchestrator/harness send <name> \
  "Summarize your current goal, what you've accomplished, and the exact next 2-3 tasks. Be concise."

# Read summary from screen, then rewrite briefing:
$EDITOR /home/sdancer/orchestrator/briefings/<name>.md

# Clear and re-seed:
/home/sdancer/orchestrator/harness send <name> "/clear"
/home/sdancer/orchestrator/harness send <name> \
  "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
```

### 8. Cross-pollination

```bash
/home/sdancer/orchestrator/harness fact-set <key> <value>
```

Index significant new files with `mcp__openviking__add_resource` when the tool is available.

### 9. Retire a path (after divergence rule fires or falsification observed)

```bash
/home/sdancer/orchestrator/harness agent-remove <name>
git -C <repo> worktree remove <worktree-path>
# Append to /home/sdancer/orchestrator/analysis/falsified.md
```

## RECORD — episode + descriptions + ledgers

```bash
/home/sdancer/orchestrator/harness episode-add \
  "<1-2 sentence cycle summary>" \
  --agent-statuses '{"agent1":"working","agent2":"idle"}' \
  --actions-taken '["nudge agent1","spawn path-foo","retire path-bar"]' \
  --goal-progress '{"nmss_cert_pure_rust":"0/5 - trace-diff progressing"}'

/home/sdancer/orchestrator/harness agent-describe <name> \
  "Working on: <current task>. Done: <key results>. Next: <planned steps>."

$EDITOR /home/sdancer/orchestrator/analysis/paths.json
```

## Planner spawn (every K=6 cycles, or on empty-backlog divergence)

```
Agent({
  description: "Path portfolio audit",
  subagent_type: "Plan",
  prompt: "Read /home/sdancer/orchestrator/analysis/paths.json, hypotheses.md, falsified.md, /home/sdancer/nmss-emu/WIKI.md, and `harness facts | tail -40`. For each goal: (1) name the metric value vs target, (2) classify each active path as progressing/stalled/at-risk with one-line justification grounded in observable facts, (3) propose 1-3 fresh hypotheses for the backlog (must not duplicate anything in falsified.md — explain why distinct), (4) flag any active path that should be retired now. Return a diff against hypotheses.md — additions, status changes, removals. Do not write code; just plan."
})
```

Record completion: `harness fact-set last_planner_cycle_<YYYY-MM-DD-HH> "<one-line summary>"`.

## Goals & sub-goals

```bash
/home/sdancer/orchestrator/harness goal-add <key> "<title>" \
  --priority 10 --success-fact-key <fact>

/home/sdancer/orchestrator/harness sub-goal-add <sg_key> <goal_key> <agent> "<title>" \
  --instruction-text "<prompt when idle>" \
  --stuck-guidance-text "<prompt when stuck>" \
  --success-fact-key <fact> --priority 10

/home/sdancer/orchestrator/harness goal-remove <key>
/home/sdancer/orchestrator/harness summary
```

## Services

```bash
/home/sdancer/orchestrator/harness services
/home/sdancer/orchestrator/harness service-add <name> \
  --service-type {systemd|http|tcp|ssh_systemd} \
  --check-target <target> --restart-policy {auto|manual}
/home/sdancer/orchestrator/harness service-remove <name> [--delete]
```

## Pane lifecycle (raw biome_term API)

```bash
curl -s -H "X-API-Key: $HARNESS_BIOME_API_KEY" \
  -X DELETE http://localhost:3021/panes/<uuid>
```
