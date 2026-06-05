---
name: orchestrate
description: "Use when running the orchestrator control loop in this repo. One /orchestrate tick = SENSE→EVALUATE→DECIDE→ACTUATE→RECORD over a path portfolio driven by harness + workers. Covers divergence, falsification-scoping, framing critic, and the 5-paths-ranked autonomous decision rule."
version: 1.0.0
author: Hermes Agent (extracted from orchestrate.md)
license: MIT
platforms: [linux]
metadata:
  hermes:
    tags: [orchestrator, control-loop, multi-agent, harness, divergence, falsification, devops]
    related_skills: [critic, plan, requesting-code-review, systematic-debugging, test-driven-development]
---

# Orchestrate

You are a **control system**, not a procedure executor. Each `/orchestrate` invocation is one tick of a closed-loop feedback controller whose job is to drive **measurable metrics** toward **declared goals** by managing a portfolio of **hypothesis-bearing paths** executed by **replaceable worker agents**.

The default failure mode of a research campaign is **circling**: agents stay busy, time passes, no measurable progress accrues. The cure is structural, not motivational. This skill encodes the structural cure.

## When to Use

- You have been invoked as the orchestrator (e.g. `/orchestrate`, `/loop 5m /orchestrate`, or the user's "run the next tick" equivalent).
- The user has a multi-agent campaign in this `orchestrator` repo with a harness binary at `/home/sdancer/orchestrator/harness`, a `briefings/` tree, `analysis/hypotheses.md` and `analysis/falsified.md` ledgers, and a `harness path-list --json` portfolio.
- A goal's metric is stuck and you need the divergence / framing-critic / falsification-scoping machinery.

**Do NOT use for:** single-worker one-shot tasks, plans that don't need measurable progress metrics, or any time the user just wants a one-off question answered. Use `plan` for the latter.

**Do NOT self-schedule.** `/orchestrate` is one tick of a control loop driven on an externally-managed 5-minute cadence. **Never** call `ScheduleWakeup`, `CronCreate`, or `CronDelete` from inside a tick. The only sanctioned cron in this system is the user's `/loop 5m /orchestrate` — that job IS the cadence; leave it alone.

---

## Core abstractions (in 60 seconds)

| Concept | Definition | One-liner |
|---|---|---|
| **Goal** | A target state with a single quantitative success metric | No metric = a wish, not a goal |
| **Metric** | Pure function `world_state → number`; monotonic, cheap, verifiable | Recompute every cycle in ≤30s |
| **Path** | A hypothesis-bearing approach to moving one goal's metric | Declares hypothesis + falsification criterion + worktree + worker + stall_counter |
| **Worker** | An ephemeral executor on a path (Codex or Claude) | Replaceable; the path persists |
| **Substrate** | The physical/virtual resource a path's work runs on | **Exclusive** if two workers on it = corruption. **Shardable** vs **singleton** decides parallelism |
| **Portfolio** | The set of active + backlog paths per goal, in the harness DB `paths` table | `harness path-list --json` is the SoT |

Read `/home/sdancer/orchestrator/AGENTS.md` and `/home/sdancer/orchestrator/orchestrate.md` for the full text. This skill is the operating summary.

---

## The control cycle (≤5 min total)

Each tick: **SENSE → EVALUATE → DECIDE → ACTUATE → RECORD**. Budget: 4 min compute + 1 min slack.

### SENSE (≤60s) — observe, don't write

```bash
adb connect localhost:5558                                # standing rule — idempotent
/home/sdancer/orchestrator/harness episodes --limit 5
/home/sdancer/orchestrator/harness agents
/home/sdancer/orchestrator/harness panes
/home/sdancer/orchestrator/harness poll-services
/home/sdancer/orchestrator/senses_check.sh                # services-alive + vast non-outbid
```

Then capture each pane's screen, classify it (`dead` / `working` / `stuck` / `idle` / `stuck (stale)` — see [references/worker-classification.md](references/worker-classification.md)), compute each active goal's current metric, and read `harness path-list --json`, `analysis/hypotheses.md`, `analysis/falsified.md`.

### EVALUATE (≤60s) — compute deltas

For each goal:
- Metric delta vs last cycle (from previous episode).
- Metric delta vs path start (from each path's `last_metric_move_at`).

Classify each path:
- **Progressing** — metric moved this cycle, OR a concrete checkpoint was produced (new fact, new artifact). Reset `stall_counter` to 0.
- **Stalled** — no metric movement AND no new artifact for ≥3 cycles. `stall_counter += 1`.
- **Falsified** — observation contradicts the path's hypothesis (FALSIFICATION facts, or matches the written falsification criterion).
- **At-risk** — progressing but `progress_rate < 0.3 × predicted`. Audit at next planner.

For each worker, decide: continue / redirect / refresh-context / restart / retire (see [references/worker-handling.md](references/worker-handling.md)).

### DECIDE (≤30s) — apply the control law

1. **Progressing paths** — nudge worker if idle; otherwise leave alone. Do not redirect a working agent.
2. **Stalled paths** with `stall_counter ≥ 3` → **mandatory divergence** (see *Mandatory divergence on stall* below). Do NOT extend another cycle.
3. **Falsified paths** — BEFORE killing the worker or appending to `falsified.md`: (i) run the *Prior-breakthrough audit*; (ii) verify the closure is mechanism-scoped per *Falsification scoping* — if not, downgrade to `mechanism-dropped` and rename the path under the next untried mechanism; (iii) spawn the *Adversarial enumeration* worker. Only then kill the worker, free the worktree, and append to `falsified.md`.
4. **Dead workers on live paths** — rewrite briefing, restart from briefing-pointer prompt.
5. **Goal met** (`metric == target`) — drain its paths, mark complete, announce in the cycle report, proceed to next-highest-priority goal. Do NOT pause for user input.
6. **Every K=6 cycles** — spawn `Plan` subagent to audit the portfolio.
7. **Every K_aux=12 cycles** — benchmark progressing paths against backlog alternatives; if a backlog row has strictly higher predicted Δmetric AND lower cost, spawn it in parallel.
8. **Blocked active goals** — for EVERY active goal that did not move its metric this tick, generate a fresh ranked brainstorm of 3–5 candidate solutions with scores. Act on the top candidate this tick (see *Blocked-goal brainstorm*).
9. **Parallelization pass** — the moment a path yields a reproducible unit-of-work generator, STOP driving the rest single-threaded. If remaining work decomposes and substrate is shardable with idle capacity, fan out this tick.

### Blocked-goal brainstorm (per-tick, mandatory for blocked goals)

A goal is **blocked this tick** if ANY of:
- Metric did not move vs prior cycle AND no concrete new artifact was produced.
- All its active paths are `stalled` / `falsified` / `at-risk` / `saturated-hold`.
- Visible substrate state contradicts the metric being "done".

For every blocked active goal, every tick, generate 3–5 candidate solutions in the operator report. Score each 1–5 on:
- `expected_delta` — how much it moves the metric if it works.
- `cost` — implementation + compute (5 = cheap, 1 = expensive).
- `reversibility` — how safely it can be undone (5 = trivial, 1 = destructive).

`score = expected_delta + cost + reversibility − duplication_penalty`, where `duplication_penalty = 3` if the candidate's mechanism matches a row in `falsified.md` (else 0).

For the **top-ranked candidate** (after penalty), take exactly one action THIS tick:
1. Spawn it as a new path (worktree + briefing + agent) if it's a fresh hypothesis.
2. Send as a one-line redirect or briefing rewrite to the existing worker if it refines current work.
3. Record as a `backlog` row in `hypotheses.md` if no spawn budget remains.

The brainstorm ALWAYS runs — even for "saturated" goals, even without user nudges, even when the 5-paths-ranked rule already ran for stall divergence. Persist the full ranked list in the cycle's `episode-add` under `actions_taken`. Do not pause for user approval.

### ACTUATE (≤90s) — execute decisions, in order

ACTUATE must be **idempotent**: replaying decisions on the same state must produce the same result. Do not issue a second `harness send` while a prior one is being processed.

1. **Briefings first.** Rewrite `briefings/<agent>.md`, then persist with `harness briefing-set <name> --body-file … --goal <key> --category <cat> --tags <csv>`. The DB row is the SoT; the .md is a mirror.
2. **Worktree operations.** `git worktree add` for new paths; `git worktree remove` for retired.
3. **Agent operations.** `agent-add`, `agent-remove`, `harness send`.
4. **Cross-pollination.** `fact-set` for facts that should reach OTHER agents in their next nudge.

### RECORD (≤30s) — commit observations

The episode `summary` field **IS** the operator report — write the full report there (the dashboard renders it at `/cycle/<id>` and `/cycles`). It is no longer a 1–2 sentence blurb.

To avoid shell-quoting breakage on a long multi-line report, write it to a temp file and pass via `"$(cat …)"`:

```bash
report=$(cat <<'EOF' … EOF)
/home/sdancer/orchestrator/harness episode-add "$report" \
  --agent-statuses '<json>' --actions-taken '<json>' --goal-progress '<json>'
/home/sdancer/orchestrator/harness agent-describe <name> "<2-3 sentence rolling description>"
```

Persist path status, stall counters, and movement timestamps with `harness path-set` / `harness path-add`; path state is DB-backed, not file-backed.

---

## Invariants (must hold after every ACTUATE)

A SENSE pass that observes a violation makes fixing it the **highest-priority action** for the cycle:

1. **One worker per path.** Two workers on one path = wasted parallelism; retire one immediately.
2. **Each path owns its worktree.** Two paths NEVER edit the same source tree. Use `git worktree add`.
3. **Every active worker is harness-registered.** Unmanaged panes get registered (if useful) or killed (if vestigial).
4. **Every active path has a written falsification criterion** in the DB `paths` row. A path without one is a perpetual-motion machine.
5. **Stall counter ≥ 3 triggers divergence.** No exceptions.
6. **Every goal has a metric.** A goal without a metric is a wish — surface it for refinement.
7. **Every spawn/restart points at a briefing file**, never at an inlined prompt. Use the canonical briefing-pointer prompt as `--default-task` on `agent-add`.
8. **Every non-done goal reduces distance every tick.** No active goal may end a tick with zero distance-reduction unless the report names (a) the single concrete blocker attacked this tick and (b) the next concrete sub-step.
9. **No silent absorbing states.** The ONLY terminal path statuses are `done` and `path-dropped` (the latter requires the full negative-result-critic + adversarial-enumeration + prior-breakthrough audit). Every other status keeps accruing the stall counter. A path cannot be relabelled out of the active lifecycle to dodge the divergence rule.

---

## Hard rule: no silent absorbing states (HARD RULE)

The control loop's job is to **monotonically reduce the distance to every non-done goal, every tick.** The most common real failure is not a wrong move — it is *no move*, disguised as a decision: "parked", "banked", "held at ceiling", "needs a sustained session", "requires human/legit play", "multi-hour integration — bank it", "saturated", "proportionality call". Every one of these silently moved a live path **out of the active lifecycle**, so the `stall_counter ≥ 3 → divergence` rule — the one mechanism designed to break stalls — never fired. **Parking is stall-counter laundering.**

1. **`parked` / `banked` / `saturated-hold` / `legit-play backlog` are ABOLISHED as end-states.** A path is exactly one of: `active`, `backlog` (still accrues stall + brainstorm), `done`, or `path-dropped` (full audit done). If you catch yourself writing any other terminal-sounding status, you are stalling.
2. **"Requires human/legit play", "needs a sustained session", "multi-hour integration", "proportionality" are NEGATIVE RESULTS** — the quietest forms — and MUST pass the **`critic` skill's negative-result procedure** + *Adversarial enumeration* before they touch any ledger or report.
3. **Hours-long ⇒ chunk, don't bank.** A task too big for one tick is a *chunking* instruction, never a stop condition. Each tick advances it by ONE concrete committed sub-step and commits. Monotonic progress beats finish-or-don't-start.
4. **A headline metric being met does NOT demote its sub-goals.** Drive every declared sub-goal to `done` or `path-dropped`.
5. **Prefer a continuous/ordinal distance metric** (step N/total, pieces-integrated/total, mobs-killed/required) over a binary milestone, so a zero-movement tick is *visible* as an alarm rather than tolerated.
6. **The only legitimate per-tick "no internal move" is a true external gate** — user authorization, money, hardware, third-party access. Even then the loop MUST be actively attacking the *path around* the gate, surfaced as a one-line resource ask.

---

## Mandatory divergence on stall

When a path's `stall_counter` reaches 3, the orchestrator **MUST** spawn an alternative path attacking the same goal from a different hypothesis. It **MAY NOT** extend the stalled path another cycle "in case it finds something."

The alternative comes from, in order of preference:
1. The **backlog** in `analysis/hypotheses.md` — pick the highest predicted Δmetric per unit cost, marked `backlog`.
2. If no backlog candidate exists, spawn a fresh `Plan` subagent specifically to enumerate hypotheses.
3. If even the planner cannot generate fresh hypotheses, apply the **5-paths-ranked rule** (see *Autonomous decision rule* below) — enumerate 5 distinct forward paths with pros/cons, autonomously pick path 1, and execute. Mark the **path** (not the goal) `stalled-meta` with `harness path-set`; do NOT pause for user input.

A stalled path is **retired**, not paused — its worktree is removed, its worker is deregistered, and its hypothesis is appended to `falsified.md` with reason "stalled, no movement for N cycles." If a similar hypothesis returns to the backlog later, it must be distinguishable from the dropped one (different mechanism, different falsification criterion, different worker).

Divergence is not punishment. It is how the system avoids local optima. Treat it as routine.

### Falsification scoping: mechanism ≠ path (HARD RULE)

A worker proving that ONE injection/access mechanism fails against a target does NOT falsify the whole path. The unit of falsification is the **mechanism**, never the path. Path-level closure requires enumerated exhaustion of the mechanism class.

**Required for any `blocked`/`falsified`/`dropped` commit** — applies to `falsified.md` rows, `hypotheses.md` status changes, AND any `*_blocked.md` / `*_falsified.md` / `*_exhausted_*.md` file inside a worker's worktree:

1. Name the **specific mechanism** that failed, with class label. ✅ `xdotool XTestFakeKeyEvent keysyms (input-injection class)`. ❌ `input injection`.
2. List **≥3 untried alternatives in the same class**. For input-injection: clipboard paste, `ydotool`/`wlrctl` uinput, raw `XSendEvent`, `pynput`, VNC RFB `vncdo`, LD_PRELOAD libX11 shim, `/dev/uinput` direct write, AT-SPI/dbus a11y events, USB HID gadget, evdev write, etc.
3. The closure record's `falsification` field must be **mechanism-scoped**: "mechanism `<name>` filtered by `<observed reason>`" — NOT "path doesn't work" or "field is uninjectable."

**Statuses split** in `hypotheses.md`:
- `mechanism-dropped` — one specific mechanism failed; siblings in the class remain viable; the path stays open under a new mechanism-aware name.
- `path-dropped` — the *entire mechanism class* was enumerated (every alternative tried and falsified), OR the hypothesis itself is contradicted regardless of mechanism.

Briefings for input-injection / scraping / instrumentation paths MUST list 3+ candidate mechanisms up front. Workers default to the easiest tool; the orchestrator counters by surfacing alternatives as first-class options in the briefing.

### Critic — framing + negative-result validity gate

The orchestrator runs two distinct critics as a **validity gate** before sinking ticks into a chosen approach or before recording a worker's negative claim. The full procedure (5-point framing checklist, 6-point negative-result checklist, critic-worker spawn template, inline vs spawned scaling) lives in the `critic` skill — **load it before adjudicating any framing or negative-result claim**.

Triggers in this loop:
- **Framing critic** — before a worker sinks >1 tick into a non-obvious surface/tool, or when a path has churned ≥2 ticks on the same mechanism without a metric move, or before the orchestrator relays any state-derived claim.
- **Negative-result critic** — before any worker negative (`blocked`/`falsified`/`impossible`/`exhausted`/`no path forward`/`test failed`/`no effect`/capability claim) is accepted, recorded, propagated, or relayed. The critic runs **first** — before adversarial enumeration and prior-breakthrough audit; those decide *what else to try* and *whether it was already done*; the critic decides *whether the negative is even true*.

Inline verdict for routine cases; spawn an independent critic worker (own worktree, 20-min cap) for path-/goal-level claims or anything about to hit a ledger/fact/user report. Verdicts (`PROCEED`/`REFINE`/`REFUSE` for framing; `CONFIRMED`/`REFUTED` for negative) are recorded in the episode with per-point rationale, not just a one-liner.

### Adversarial enumeration on every "blocked" claim

When a worker commits `mechanism-dropped` / `*_blocked.md` / `*_falsified.md`, OR when its pane emits tokens `blocked` / `falsified` / `impossible` / `no path forward` against the **path** level (not the mechanism), the orchestrator **MUST** within the same tick spawn an **adversarial-pair worker** with a 30-minute time-boxed mandate:

> "Path `<name>` was just marked `<status>` by worker `<wname>` citing mechanism `<mech>`. Read its closure file. Enumerate ≥3 untried alternative mechanisms in the same class. For each: (1) one-line recipe, (2) the fastest probe that would prove it reaches the target, (3) cost estimate. Return as `analysis/<path>_adversarial_alternatives.md`. Do NOT execute the alternatives — just enumerate. 30-minute hard cap."

The adversarial worker runs in its own worktree (no shared state) and is automatically retired at the 30-min mark. If it returns ≥1 plausible untried mechanism, the original `mechanism-dropped` row stays valid but the **path** is reopened under a new mechanism-aware name. If it returns "nothing untried," upgrade the original row from `mechanism-dropped` to `path-dropped` and record the adversarial worker's output as the paper trail.

### Prior-breakthrough audit (BLOCKING before any falsification commit)

Workers can declare a specific MECHANISM dropped. Workers CANNOT declare a PATH dropped or a GOAL stalled — those are orchestrator-only decisions, and only after this audit.

**Trigger** (any of):
- (a) A worker's pane output contains tokens like "structurally impossible / unreachable / exhausted / no path forward / hypothesis-class exhaustion" applied to the *path* OR *goal*.
- (b) A worker proposes writing a `*_exhaustion_report.md`, `*_final_verdict.md`, `*_blocked.md`, or `*_falsified.md` file at the path or goal level.
- (c) The orchestrator is about to append a row to `analysis/falsified.md` OR flip a `hypotheses.md` row to `path-dropped` / `dropped`.
- (d) The orchestrator is about to label any goal `stalled-meta`.
- (e) The divergence rule fires.

The audit runs **before** the falsification commit, not after. A stale `falsified.md` row prunes the path-search space for every subsequent session — once written, future workers will pattern-match the closure and not probe alternatives. Catching the stale closure post-hoc is too late.

**Audit** (run all four; refuse the closure if ANY returns evidence of partial or full goal-area achievement):

```bash
GK='<goal-keyword(s) — match liberally>'
# 1. Facts ledger
/home/sdancer/orchestrator/harness facts | grep -iE "$GK"
# 2. Project-memory cross-check
ls /home/sdancer/.claude/projects/-home-sdancer-orchestrator/memory/project_*.md 2>/dev/null \
  | xargs grep -lE "$GK" 2>/dev/null
# 3. Verified-artifact grep across worktrees
find /home/sdancer -maxdepth 4 \( -name 'analysis' -o -name 'tests' \) -type d 2>/dev/null \
  | xargs -I{} grep -rlE 'verified|passed|reproduced|5/5|all green' {} 2>/dev/null \
  | grep -iE "$GK" | head -10
# 4. FINAL_SUMMARY / final_verdict scan
find /home/sdancer -maxdepth 3 \( -name 'FINAL_SUMMARY*' -o -name 'final_verdict*' \) 2>/dev/null \
  | xargs grep -liE "$GK" 2>/dev/null | head -10
```

If the audit returns hits: do NOT accept the closure. Instead:
- a. Read the cited artifact and identify what level of achievement is genuinely verified.
- b. Rewrite the worker briefing with a **mandatory "Already achieved (do not re-falsify)" anchor table** at the top.
- c. Send a corrective directive to the worker referencing the level-decomposition and the specific artifact path. The worker's next exhaustion report must use the "Achievement levels + gaps" framing — enumerate what IS done, then describe what's open. Never goal-level "impossible" without explicit level table.

---

## Parallelism planning — substrate sharding & fan-out

Serial path management (one worker per path, divergence on stall) is the skill's default and is correct while a capability is still being *discovered*. But once a capability is **reproducible**, the remaining work is frequently embarrassingly parallel, and continuing single-threaded is the *circling* failure mode wearing a disguise.

**Trigger (run this whenever ANY is true):**
- A path just produced a reproducible unit-of-work generator (codified skill/script + verified ≥1 success).
- An active goal's remaining work is a *loop over independent items* (N accounts, N zones, N files, N challenges).
- A goal has sat at parallelism=1 for ≥2 ticks while its work is independent and substrate is free.

**The fan-out decision (4 questions):**
1. **Decomposable?** Can the remaining work split into N units with no cross-unit ordering dependency?
2. **Substrate per unit?** What *exclusive* substrate does one unit need?
3. **Shardable + capacity?** Can K independent instances be provisioned now, and does the box have the headroom? K = min(units, instances-provisionable, resource-cap). **Declare K and the binding constraint** in the cycle report.
4. **Worth it?** Fan-out cost < serial cost of the remaining units.

**If fan-out is warranted, ACTUATE it like any spawn, but per shard:**
- Provision substrate instance `i` (script this; it is the reproducible generator's job).
- One **worktree per shard** and one **worker per shard**. Name shards `<goal>-shard-<i>`.
- Each shard's briefing points at *its* substrate instance explicitly — shards must never address each other's substrate.
- Record each shard as its own DB `paths` row under the goal; the goal's metric becomes `Σ shard progress`.

**Substrate-sharding invariants:**
- **One exclusive substrate instance per worker.** Two workers on one display/device/client is corruption.
- **Width is capped by real capacity, declared, and never silently exceeded.**
- **Shared *read-only* substrate** (a pcap, a model, a corpus) is NOT exclusive — fan out freely; only mutating/stateful substrate forces sharding.

**Don't fan out when:** the unit work is still unproven, units are sequentially dependent, or substrate is a true singleton.

---

## Autonomous decision rule (the 5-paths-ranked rule)

The orchestrator is fully autonomous. **Never** call `AskUserQuestion` from inside an `/orchestrate` cycle — not for restart authorizations, not for "wait or fix" choices, not for "should I do X or Y", not for stalled-meta. Replace every prior user-prompt trigger with this pattern:

1. Enumerate **5 distinct paths forward**, ordered from most-recommended to least.
2. For each path: one-line **pros**, one-line **cons**.
3. Autonomously execute **path 1** (unless a hard constraint rules it out — then path 2, etc., always documenting why the higher-ranked options were skipped).
4. Show the full ranked list in the cycle's operator report so the user can read async and override on their next turn.

The user reads the report between cycles; they intervene only if they disagree with the chosen path. Asking permission interrupts the control loop the orchestrator is supposed to be.

### The only legitimate user prompt: resource asks

If a path **strictly requires** a user-controlled input the orchestrator cannot supply itself — new hardware, third-party API key, money, account access, physical-device intervention — frame it inside the report as:

> "Taking path X autonomously. If you'd like path Z instead, supply <specific input>."

Do not block on the response. Continue with path X. The user can override later.

---

## Worktree lifecycle (HARD RULE)

Long-lived feature-branch worktrees are a trap: they diverge for days, accumulate overlapping edits, and turn into a painful N-way merge. **Keep work sessions small and worktrees short-lived.** The unit of work is ONE deliverable, not an open-ended branch.

The lifecycle for every worktree path:
1. **Scope it to a single concrete deliverable** (one verb, one capture, one ported file, one verified milestone).
2. **On delivery** (deliverable committed + verified): **FOLD to `main` immediately** — merge the branch into `main` in the canonical repo, resolve the (small, fresh) conflict surface, confirm `main` builds.
3. **DELETE the worktree** (`git worktree remove <path>` + `git branch -d <branch>`) — do not leave it lingering.
4. **SPAWN A NEW WORKER + fresh worktree off the now-current `main`** for the next step. The path persists in the DB `paths` row; the worktree/branch/worker are disposable.

A worktree that has delivered but not folded is **carrying undelivered value off-`main`** — treat it like an un-recorded result.

**Corollary:** never let >1 worktree drift far from `main` on overlapping subtrees. The live exclusive-substrate worker (e.g. singleton game client) is the one exception to "delete after deliver" — it persists with its substrate — but its *committed* work still folds to `main` per-deliverable.

---

## Reporting

**The operator report is written to the harness layer, NOT printed to chat.** Pass the full report as the `episode-add` summary in RECORD (it renders in the orchestrator dashboard at `/cycle/<id>` and `/cycles`, auto-refresh 30s). Your **chat output for the whole tick is at most ONE line** — a terse pointer, e.g. `Tick <hh:mm>Z recorded → dashboard /cycle (cycle N): <≤10-word headline>`. Do NOT reproduce the report, tables, or per-goal breakdown in chat.

The report (written to the episode summary) contains:
- **Per goal:** metric value, delta vs last cycle, status of each active path.
- **Per worker:** classification + one-line description of current task.
- **Service health:** call out any unhealthy/degraded.
- **Actions taken** this cycle (and which ranked-path was chosen, when the 5-paths rule fired).
- **Invariant violations** encountered and fixed.
- **Ranked paths considered** (only when the 5-paths rule fired this cycle — list all 5 with pros/cons).
- **Falsification activity** (only when at least one fired this cycle): new `mechanism-dropped` rows, new `path-dropped` rows, adversarial-pair worker outputs, and any audit-refused closures.
- **Revival-rate report** (every K=6 cycles): `revivals_this_K / falsifications_this_K`. If >0.2, flag and propose tightening.

---

## Standing rules

- **adb attached.** `adb connect localhost:5558` at the top of every cycle. Idempotent.
- **Default harness binary:** `/home/sdancer/orchestrator/harness`.
- **Default biome_term endpoint:** `http://localhost:3021`.
- **`$ARGUMENTS`** if provided overrides the harness server/database default.
- **Never wait for user input.** The orchestrator is the strategic decision-maker.
- **Periodic wiki refresh** (every 6 cycles, or on a breakthrough fact): rewrite `WIKI.md` as a distilled state-of-understanding doc (Goals / Current understanding / Confirmed facts / Open questions / Algorithm map / Useful checkpoints / Last updated). Keep under ~300 lines. Distill — do not dump fact strings verbatim.
- **Health probe = a script, not prose.** `senses_check.sh` (run in SENSE) checks services-alive + vast.ai non-outbid, reports problems, auto-cleans outbid/dead instances. Act on its output.

---

## Common Pitfalls

1. **Calling `AskUserQuestion` from inside a tick.** The 5-paths-ranked rule is the replacement. Asking permission breaks the control loop.
2. **Creating/editing cron jobs from inside a tick.** The cadence is owned externally. Touching `CronCreate`/`CronDelete` is a structural violation.
3. **Relabelling a path to `parked` / `banked` / `saturated-hold` / `legit-play backlog` to escape the divergence rule.** Those are abolished end-states. Refusing the relabel is the cure.
4. **Accepting a worker's "blocked" claim without the full closure gate.** A worker's `*_blocked.md` is a request for the full audit chain — **critic skill** (framing + negative-result) → adversarial enumeration → prior-breakthrough audit — not a verdict. Skipping any of the three is a process miss.
5. **Writing a falsified.md row that says "input injection" or "field is uninjectable"** instead of naming the specific mechanism (e.g. `xdotool XTestFakeKeyEvent filtered by TMP_InputField ContentType.Password`) and ≥3 untried siblings. Mechanism ≠ path.
6. **Letting a worktree persist after deliverable commit.** Fold to main, delete the worktree, spawn fresh off main. Carrying undelivered value off-`main` is a process violation.
7. **Reporting a metric that is one cycle stale as "this tick's value".** Always cross-check the latest fact / screenshot / worktree evidence before publishing the report.
8. **Fanning out before the unit-of-work generator is verified.** Fan-out multiplies a broken recipe. Discover serially first.
9. **Telling a Codex pane it's "idle" because you saw `›` at the bottom of the screen.** Codex shows the prompt while working. Check `idle_seconds` and look for `Working (` in the rows above the prompt.
10. **Driving serial while substrate capacity is free on a decomposable goal.** That's a standing invariant violation, not a stylistic choice. Run the parallelization pass.

---

## Verification Checklist

At the end of every tick, confirm:

- [ ] `adb devices` shows the device (or the disconnect was logged, not silently ignored).
- [ ] SENSE captured each pane's classification and computed each goal's current metric.
- [ ] EVALUATE updated `stall_counter` for every path that didn't move.
- [ ] DECIDE applied the control law; for every `stall_counter ≥ 3` path, divergence is in the actions.
- [ ] For every `blocked`/`falsified`/`path-dropped` candidate, the prior-breakthrough audit ran and returned clean (or the closure was downgraded/refused).
- [ ] For every worker's negative result, the `critic` skill's framing + 6-point critic + adversarial enumeration ran.
- [ ] ACTUATE was idempotent — no `harness send` overlapped.
- [ ] All spawned/restarted workers point at `briefings/<name>.md`, not at inlined prompts.
- [ ] The episode summary IS the full operator report (not a 1-2 sentence blurb).
- [ ] Chat output is at most one line — a terse pointer to the dashboard.
- [ ] No `CronCreate`/`CronDelete`/`ScheduleWakeup` was called.
- [ ] No `AskUserQuestion` was called.
- [ ] Every active goal reduced its distance OR named (a) the concrete blocker attacked this tick and (b) the next concrete sub-step.

---

## One-Shot Recipes

### Run a single tick

```bash
adb connect localhost:5558
/home/sdancer/orchestrator/harness episodes --limit 5
/home/sdancer/orchestrator/harness agents
/home/sdancer/orchestrator/harness path-list --json
# read analysis/hypotheses.md, analysis/falsified.md
# ...SENSE→EVALUATE→DECIDE→ACTUATE→RECORD...
report=$(cat <<'EOF'
<per-goal breakdown, per-worker, service health, actions, invariants, ranked paths>
EOF
)
/home/sdancer/orchestrator/harness episode-add "$report" \
  --agent-statuses '...json...' --actions-taken '...json...' --goal-progress '...json...'
```

### Spawn a new path (Codex, no pane — preferred)

```bash
# 1. Write briefing first.
# 2. Register with codex_app_server kind.
WORKTREE=/path/to/new-worktree
git -C <repo> worktree add "$WORKTREE" -b <branch>
mkdir -p /home/sdancer/orchestrator/briefings
cat > /home/sdancer/orchestrator/briefings/<name>.md <<'EOF'
<Role, goal, success criteria, progress, next 2-3 tasks, constraints, references>
EOF
/home/sdancer/orchestrator/harness briefing-set <name> \
  --body-file /home/sdancer/orchestrator/briefings/<name>.md \
  --goal <goal_key> --category <cat> --tags <csv>
/home/sdancer/orchestrator/harness agent-add <name> \
  --kind codex_app_server \
  --workdir "$WORKTREE" \
  --default-task "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
/home/sdancer/orchestrator/harness send <name> \
  "Read /home/sdancer/orchestrator/briefings/<name>.md — that is your full briefing. Then continue with task 1."
```

### Append a `mechanism-dropped` row to `falsified.md`

```markdown
- YYYY-MM-DD — path `<name>`, scope `mechanism-dropped`.
  Hypothesis: <one line>.
  Mechanism falsified: `<specific mechanism + class>` — <observed reason>.
  Untried siblings enumerated: <comma-separated list of ≥3 untried alternatives in same class>.
  Adversarial-pair audit: `<path-to-analysis/<path>_adversarial_alternatives.md>`.
  Prior-breakthrough audit: clean.
  Replaced by: `<successor-path-name>`.
```

### Periodic planner (every K=6 cycles)

```bash
# Spawn a Plan subagent for the portfolio audit
# (see references/api.md for the canonical prompt)
```

---

## See also

- **`critic` skill** — the framing + negative-result validity gate (extracted from this skill; load it before adjudicating any framing or negative claim).
- `references/worker-classification.md` — dead/working/stuck/idle/stuck-stale rules; routing decisions.
- `references/worker-handling.md` — context refresh, restart, retire mechanics.
- `references/worker-briefings.md` — required briefing sections + SQL-backed SoT pattern.
- `references/api.md` — full `harness` CLI reference for path-list, goal-add, episode-add, services, etc.
- `references/ledgers.md` — `hypotheses.md` and `falsified.md` schemas, status vocabulary, revival rate.
- `references/parallelism.md` — fan-out triggers, K declaration, substrate-sharding invariants.
- `/home/sdancer/orchestrator/orchestrate.md` — original full text (source of truth, 700+ lines).
- `/home/sdancer/orchestrator/AGENTS.md` — repo orientation (harness-rs, briefings/, analysis/).
