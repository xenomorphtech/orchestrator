---
name: orchestrate
description: Use this skill when the task is to monitor, classify, and drive Claude or Codex agents running in biome_term under the Rust harness. It is for running an orchestration cycle, recovering context from prior episodes, checking service health, nudging idle agents, handling unmanaged panes, cross-pollinating findings, and recording a new episode.
---

# Orchestrate

## Overview

Use this skill to run one orchestrator cycle against the local harness and `biome_term`.
It is for operator work, not product code changes.

## Workflow

1. Recover context from the harness before touching agents.
2. Discover managed agents and all biome_term panes.
3. Capture each pane screen and classify it as `working`, `idle`, `stuck`, or `dead`.
4. Poll service health.
5. Run `harness run-once-biome --execute`.
6. Handle unmanaged panes separately.
7. Share important findings with facts, index important artifacts if any, then record an episode.

## Core Rules

- Treat `/home/sdancer/orchestrator/harness` as the default harness binary.
- Treat `http://localhost:3021` as the default `biome_term` endpoint.
- If `$ARGUMENTS` is present, use it as the harness server or database override instead of the default.
- When spawning Codex agents, always use `codex --dangerously-bypass-approvals-and-sandbox`.
- Do not use a visible Codex `›` prompt by itself to classify a pane as idle.
- For Codex panes, check for `Working (` in recent rows and use `idle_seconds` from the panes API.
- Nudge idle agents even if their context appears low; only do a context refresh when the agent is idle, still has useful work, and needs compaction help.
- Keep nudges short. If an agent already stated the next step, send `Continue.` or a one-line continuation rather than re-explaining the task.
- Do not restart or redirect work that prior episodes show is already in progress unless the current pane state contradicts that history.

## Classification Rules

- `dead`: pane is terminated or the pane lookup returns `404`.
- `working`: `idle_seconds == 0`, or recent rows contain `Working (`, or a clear spinner/processing indicator.
- `stuck`: recent rows show an error pattern and there is no active working indicator.
- `idle`: `idle_seconds > 0`, the last non-empty row looks like a prompt, and there is no working indicator.
- `stuck (stale)`: pane output has not changed for 10 or more minutes across cycles.

Error patterns include `traceback`, `exception`, `error:`, `permission denied`, `segmentation fault`, and `command not found`.

## Commands

Run these in order unless you have a concrete reason to skip a step:

```bash
/home/sdancer/orchestrator/harness episodes --limit 5
/home/sdancer/orchestrator/harness agents
curl -s http://localhost:3021/panes
/home/sdancer/orchestrator/harness poll-services
/home/sdancer/orchestrator/harness run-once-biome --execute
```

For detailed command snippets, unmanaged-pane handling, spawn examples, and episode recording, read `references/command-reference.md`.

## Idle Agent Handling

When an agent is idle and its last output already names the next step:

- Send `Continue.`
- Or send `Continue. <one-line summary of the agent's own next step>`

Only redirect the agent if another pane already produced information that changes the plan.

## Context Refresh

Use this sparingly for idle agents with clear remaining work:

1. Ask for a concise summary of goal, completed work, and next 2-3 tasks.
2. Read the summary from the pane.
3. Send `/clear`.
4. Re-inject the goal, completed work, and numbered next tasks.
5. Tell the agent to continue with task 1.

## Outputs

At the end of the cycle, produce a short operator report covering:

- managed agent statuses
- unmanaged panes and their status
- service health, especially any `unhealthy` or `degraded` services
- actions taken this cycle
- goal progress
- alerts that need manual attention

Then record the cycle with `episode-add` and update any stale agent rolling descriptions with `agent-describe`.
