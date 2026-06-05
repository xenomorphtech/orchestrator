# Critic-worker prompt template

Drop-in template for spawning an independent critic worker to adjudicate a path-/goal-level negative. The critic worker is its own worktree, no shared state with the original worker, 20-minute hard cap.

## Template

```
Worker `<wname>` reports negative result `<claim>` for path `<name>`.

Your job is to FALSIFY THE NEGATIVE — find the most likely reason this
"failure" is actually a false negative.

Work the 6-point validity checklist:

  1. Precondition met? (right substrate, right state, target alive, code path reached)
  2. Mechanism implemented correctly? (harness-correctness: right offset, right tool, no self-inflicted interference)
  3. Capability/access actually blocked, or privilege-context? (verify on the host that runs the target)
  4. Is it on the critical path at all? (sibling formulation may sidestep the "blocker")
  5. Measurement valid? (thin-sample vs real signal, right detector, reproduced ≥2×)
  6. First-principles check: is the asserted impossibility actually a law, or one mechanism's failure being over-generalized?

For each point, document:
  - What you checked
  - The evidence (with paths to artifacts, log lines, or process checks)
  - Pass / Fail / Inconclusive

Return `analysis/<path>_critic.md` with a verdict:

  - CONFIRMED: negative is real, with the evidence that survives each check.
    Include the mechanism scope ("CONFIRMED for mechanism `<mech>` in class `<class>`;
    siblings `<sib1>`, `<sib2>` remain untried").

  - REFUTED: here is the false-negative cause + the cheapest re-test.
    Include: which of the 6 points was the giveaway, what specifically was wrong,
    and a concrete re-test command sequence the original worker can run.

Do NOT enumerate new mechanisms — that's a separate worker; only adjudicate this claim.

20-minute hard cap. Write the verdict file before you run out of time, even if
some points are still inconclusive — name the inconclusive points explicitly.
```

## Substitution points

| Token | Replace with |
|---|---|
| `<wname>` | The name of the worker that reported the negative (e.g. `codex-runner-2`). |
| `<claim>` | The worker's exact claim, quoted. e.g. `"xdotool blocked by Unity password field"` or `"static dataflow exhausted after 14 days"`. |
| `<path>` | The path's DB key (e.g. `launcher-signup-xdo`). Used in the output filename. |
| `<mech>`, `<class>`, `<sib1>`, `<sib2>` | Only fill these in the CONFIRMED verdict (the critic doesn't need them in the prompt — it discovers them by working the checklist). |

## What the orchestrator does with the verdict

- **CONFIRMED** → the negative is real for this mechanism. Now proceed to Falsification-scoping + Adversarial-enumeration + Prior-breakthrough-audit (all in the `orchestrate` skill). The critic's output becomes part of the closure paper trail — link it from the new `falsified.md` row.
- **REFUTED** → the negative is rejected. Re-task the original worker with the critic's re-test; do NOT record any closure. Log it (a refuted negative is a near-revival — track toward the revival-rate quality metric). The critic's output is the rationale for the rejection — keep it in `analysis/<path>_critic.md` even if no closure row is written.

## When to use this template

Use it for **path- or goal-level negatives**, or **any negative about to hit a ledger/fact/user report**. Don't use it for routine mechanism-level probes (run the 6-point checklist inline and record the verdict in the episode — see the main `critic` SKILL.md).

Triggers that should escalate from inline to spawned critic worker:
- The worker's pane output contains tokens like "structurally impossible / unreachable / exhausted / no path forward / hypothesis-class exhaustion" applied to the *path* OR *goal* (not just a single hypothesis or mechanism).
- A worker proposes writing a `*_exhaustion_report.md`, `*_final_verdict.md`, `*_blocked.md`, or `*_falsified.md` file at the path or goal level.
- The orchestrator is about to append a row to `analysis/falsified.md` OR flip a `hypotheses.md` row to `path-dropped` / `dropped`.
- The orchestrator is about to label any goal `stalled-meta`.
- The divergence rule fires (because divergence often means accepting a "this approach didn't work" framing — make sure that framing is valid first).
