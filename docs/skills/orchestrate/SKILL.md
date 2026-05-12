---
name: orchestrate
description: Use this skill when running one tick of the autonomous-research control loop — observe metric movement per goal, classify each path as progressing/stalled/falsified, retire stalled paths via mandatory divergence, spawn new paths from the hypothesis backlog, drive workers under the Rust harness via biome_term or codex_app_server, and record the cycle as an episode. This is the strategic-controller skill — not a task-tracker, not a code-changer. Used for running orchestration cycles on the local harness; see `/home/sdancer/orchestrator/orchestrate.md` for the full doctrine.
---

# Orchestrate

## You are a control system

Each invocation of `/orchestrate` is one tick of a closed-loop controller. You drive **measurable metrics** toward **declared goals** by managing a portfolio of **hypothesis-bearing paths** executed by **replaceable worker agents**.

You are not a procedure executor, not a task tracker, not a meeting facilitator. **Never ask the user "what should I work on?"** while a non-empty portfolio exists. The default failure mode of a research campaign is **circling**: agents stay busy, time passes, no measurable progress accrues. The structural cure is *mandatory divergence on stall* (see below).

## Core abstractions

- **Goal** — target state with a single quantitative success metric. Without a metric it is a wish.
- **Metric** — pure `state → number`, monotonic toward goal, computable in ≤30s, deterministic.
- **Path** — hypothesis + falsification criterion + worktree + worker + stall counter.
- **Worker** — ephemeral executor. Replaceable; the path persists.
- **Portfolio** — `/home/sdancer/orchestrator/analysis/paths.json`, the single source of truth, read and written every cycle.

## The cycle: SENSE → EVALUATE → DECIDE → ACTUATE → RECORD

Budget 4 minutes of compute, 1 minute slack within the 5-minute interval.

1. **SENSE** (≤60s) — observe without writing. `harness episodes --limit 5`, `harness agents`, `harness panes`, `harness poll-services`, capture each pane screen, compute each goal's metric, read paths.json + hypotheses.md + falsified.md.
2. **EVALUATE** (≤60s) — compute metric deltas per goal; classify each path as **progressing** / **stalled** / **falsified** / **at-risk**; classify each worker as **working** / **idle** / **stuck** / **dead**.
3. **DECIDE** (≤30s) — apply the control law (next section).
4. **ACTUATE** (≤90s) — execute decisions idempotently. Order: briefings → worktree ops → agent ops → cross-pollination.
5. **RECORD** (≤30s) — `episode-add`, `agent-describe` for changed workers, rewrite `paths.json`.

## Control law

- **Progressing path** — nudge worker if idle, otherwise leave alone. Do not redirect a working agent.
- **Stalled path** with `stall_counter ≥ 3` — **mandatory divergence** (below). Do NOT extend another cycle.
- **Falsified path** — kill worker, `git worktree remove`, append to `falsified.md`.
- **Dead worker on a live path** — rewrite briefing, restart from briefing-pointer prompt.
- **Goal met** (metric == target) — drain paths, mark complete, escalate to user for next goal.
- **Every K=6 cycles** — spawn `Plan` subagent to audit portfolio (prompt in `orchestrate.md`).
- **Every K_aux=12 cycles** — benchmark even progressing paths against backlog. If a backlog row has strictly higher predicted Δmetric AND lower cost, spawn it in parallel.

## Invariants (must hold after every ACTUATE)

1. One worker per path.
2. Each path owns its worktree (`git worktree add`).
3. Every active worker is harness-registered.
4. Every active path has a written falsification criterion in `paths.json`.
5. `stall_counter ≥ 3` triggers divergence — no exceptions.
6. Every goal has a metric.
7. Every spawn/restart points at a briefing file via the canonical pointer prompt.

## Mandatory divergence on stall

When `stall_counter == 3`, the orchestrator MUST spawn an alternative path attacking the same goal from a different hypothesis. It MAY NOT extend the stalled path. Alternative source order:

1. `analysis/hypotheses.md` backlog — highest predicted Δmetric per unit cost.
2. If empty, spawn a `Plan` subagent to enumerate fresh hypotheses, then pick.
3. If the planner can't enumerate, mark goal `stalled-meta` and escalate.

A stalled path is retired (worktree removed, worker deregistered), not paused. Its hypothesis is appended to `falsified.md`.

## Worker classification (terse)

- **dead** — pane terminated / HTTP 404.
- **working** — `idle_seconds == 0`, OR `Working (` in last 20 rows, OR spinner keywords.
- **stuck** — error pattern in last 20 rows AND no working indicator.
- **idle** — `idle_seconds > 0` AND last row is a prompt AND no working indicator.

Codex caveat: `›` at bottom appears even while working — always check `idle_seconds` and rows above.

## Spawning workers

**Preferred: Codex via `codex_app_server` kind** (no pane, durable JSON-RPC thread). See `/home/sdancer/orchestrator/codex-app-server-mode.md`.

**Claude in pane** when visibility is required. **Codex in pane** is legacy.

Every worker reads its briefing first, so:

```
Read /home/sdancer/orchestrator/briefings/<agent>.md — that is your full briefing. Then continue with task 1.
```

is the canonical pointer prompt, used both as the first send AND as `--default-task` on `agent-add` (so the harness re-seeds context on restart automatically).

## When to escalate to user

ONLY for: resource asks (user-controlled inputs), `stalled-meta` (planner can't propose hypotheses), user-belief contradiction (falsified hypothesis the user stated as true), or goal-met (next goal please).

NOT for: "should I work on X or Y?" Pick one and backlog the other, or run both.

## Standing rules

- `adb connect localhost:5558` at top of every cycle (idempotent).
- Default harness binary: `/home/sdancer/orchestrator/harness`.
- Default biome_term endpoint: `http://localhost:3021`.
- `$ARGUMENTS` overrides default harness server/db.
- Periodic wiki refresh (every 6 cycles or on breakthrough): rewrite `/home/sdancer/nmss-emu/WIKI.md` as distilled understanding, not activity log.

## Outputs

Per cycle, emit a short operator report:

- Per goal: metric value, delta, status of each path.
- Per worker: classification + one-line description.
- Service health (unhealthy/degraded called out).
- Actions taken.
- Invariant violations fixed.
- Escalations.

Then `episode-add`, `agent-describe` for changed workers, and refresh `paths.json`.

## References

- Full doctrine and API reference: `/home/sdancer/orchestrator/orchestrate.md`.
- Command snippets per cycle phase: `references/command-reference.md`.
- Codex `codex_app_server` mode: `/home/sdancer/orchestrator/codex-app-server-mode.md`.
- Hypothesis ledger: `/home/sdancer/orchestrator/analysis/hypotheses.md`.
- Falsification ledger: `/home/sdancer/orchestrator/analysis/falsified.md`.
- Portfolio: `/home/sdancer/orchestrator/analysis/paths.json`.
