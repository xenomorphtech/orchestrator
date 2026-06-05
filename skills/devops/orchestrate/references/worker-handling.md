# Worker handling — refresh, restart, retire

## Context refresh (use sparingly)

1. Send: `Summarize your current goal, what you've accomplished, and the exact next 2-3 tasks. Be concise.`
2. Read the worker's response from the pane screen.
3. Rewrite `/home/sdancer/orchestrator/briefings/<agent>.md` using the worker's summary + the latest facts and episode context.
4. Send `/clear`.
5. Send the canonical briefing-pointer prompt:
   ```
   Read /home/sdancer/orchestrator/briefings/<agent>.md — that is your full briefing. Then continue with task 1.
   ```

Never paste the summary inline. Always drive workers off the briefing file so the briefing-pointer prompt works for both fresh spawns and post-clear restarts.

## Restart from briefing-pointer prompt

When a worker dies on a live path:

1. Read the path's last brief at `briefings/<agent>.md`. If the brief is stale, rewrite it FIRST (worker-briefings → [SQL-backed pattern](worker-briefings.md)).
2. Re-register the agent with `harness agent-add` (or re-attach if the row already exists).
3. Send the canonical briefing-pointer prompt.

## Retire a worker

When a path is `done` or `path-dropped` (full audit done):

1. Stop sending prompts.
2. `harness agent-remove <name>` (deregisters; also clears `biome_pane_id` for housekeeping).
3. `git worktree remove <worktree>` and `git branch -d <branch>` once the deliverable is folded to `main` (see orchestrator's Worktree lifecycle rule).
4. Append the path's closure row to `analysis/falsified.md` (with mechanism-scoped reasoning + adversarial-pair + prior-breakthrough paper trails) if it was a falsification.

## When NOT to refresh

- `idle_seconds > 0` AND last row is a clean prompt AND the path is progressing — just send `Continue.`
- The worker is mid-execution (working indicator present) — do not interrupt.
- Context is low but progress is fast — let auto-compaction handle it.

## When NOT to restart

- The path is `stalled` with `stall_counter ≥ 3` — the divergence rule wants a fresh path, not a restart of the same one.
- The path is `path-dropped` — restarting would relitigate a closed closure. Spawn the replacement path instead.
- The worker's failure mode is environmental (substrate down, account banned) — fix the substrate first, then restart.
