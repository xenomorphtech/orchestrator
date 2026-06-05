# Periodic planner (every K=6 cycles)

Every K=6 cycles (≈30 min), OR any time the divergence rule fires with an empty backlog, spawn:

```
Agent({
  description: "Path portfolio audit",
  subagent_type: "Plan",
  prompt: "Read `harness path-list --json`, analysis/hypotheses.md, analysis/falsified.md, /home/sdancer/nmss-emu/WIKI.md, and `harness facts | tail -40`. For each goal: (1) name the metric value vs target, (2) classify each active path as progressing/stalled/at-risk with one-line justification grounded in observable facts, (3) propose 1-3 fresh hypotheses for the backlog (must not duplicate anything in falsified.md — explain why distinct), (4) flag any active path that should be retired now. Return a diff against hypotheses.md plus any required `harness path-set/path-add/path-remove` operations. Do not write code; just plan."
})
```

Apply the returned diff. Record a fact `last_planner_cycle_<YYYY-MM-DD-HH>` with a one-line summary so concurrent orchestrator instances don't double-spawn.

At the same K=6 cadence, run `harness briefing-archive-unused` (briefing housekeeping — archives any briefing with no live agent; non-destructive + reversible; see `references/worker-briefings.md`).

## Planner outputs (what the orchestrator does with them)

1. **Diff against `hypotheses.md`** — apply row updates, status changes, new backlog entries.
2. **`harness path-set` / `path-add` / `path-remove` operations** — execute in ACTUATE; persist to DB.
3. **Retire flag for any active path** — treat as a falsification trigger: run the prior-breakthrough audit first; if clean, spawn the adversarial-pair worker; only then retire.
4. **1-3 fresh hypotheses** — write into `hypotheses.md` with explicit "why distinct from anything in falsified.md" justification. Do not accept hypotheses that mechanically duplicate a dropped row.
