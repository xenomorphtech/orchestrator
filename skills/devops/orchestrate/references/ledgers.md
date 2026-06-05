# Hypothesis & falsification ledgers

## `analysis/hypotheses.md`

Standing list of testable hypotheses. The planner edits this file; the orchestrator reads from it whenever the divergence rule fires or a path completes.

### Status vocabulary (strict — `dropped` alone is no longer valid)

| Status | Meaning |
|---|---|
| `active` | Currently worked on a worktree. |
| `backlog` | Queued; not yet spawned. Still accrues stall + subject to per-tick brainstorm. |
| `done` | Success metric reached / artifact committed. |
| `mechanism-dropped` | One specific mechanism failed; siblings in the class are still viable. Path stays open under a renamed entry that names the next mechanism. Adversarial-pair audit MUST have run. |
| `path-dropped` | Entire mechanism class enumerated and falsified, OR the hypothesis itself is contradicted regardless of mechanism. Requires prior-breakthrough audit + adversarial-pair audit both clean. |
| `revived` | Was previously `mechanism-dropped` / `path-dropped`, then a sibling mechanism or rewritten hypothesis succeeded. **Track these — high revival count is a quality signal that the orchestrator's prior closures were premature.** |

### Row schema

```
| status              | path-name              | hypothesis (one line)                            | predicted Δmetric | falsification (mechanism-scoped)                          | est cost |
| active              | trace-diff             | per-instr trace diff yields algorithm slice      | +N% certs         | slice produces no synthesizable Rust                      | 6h       |
| backlog             | symbolic-rep           | symbolic execution of cert path                  | +M% certs         | state-space explodes / unsolvable                         | 8h       |
| done                | minimization           | PROT_NONE demand-trace yields touched-page set   | +data-dep map     | (consumed: 31655 pages identified)                        | -        |
| mechanism-dropped   | launcher-signup-xdo    | xdotool keystrokes reach Unity password field    | +1 account        | mechanism `xdotool XTestFakeKeyEvent` filtered by Unity TMP_InputField ContentType.Password; siblings (clipboard paste, uinput, RFB) untried | 2h |
| path-dropped        | static-disasm          | static dataflow recovers frag1                   | +X% certs         | entire static-dataflow class enumerated; wrapper-tree wall after 14d; adversarial worker returned 0 alternatives | - |
| revived             | launcher-signup-clip   | xclip+ctrl+v paste reaches Unity password field  | +1 account        | (revival of `launcher-signup-xdo`; verified 2026-05-19)   | -        |
```

## `analysis/falsified.md`

Append-only ledger of falsified mechanisms and paths. Every retired row lands here with **mechanism-scoped** reasoning and a paper trail.

### Required row format

```
- YYYY-MM-DD — path `<name>`, scope `<mechanism-dropped|path-dropped>`.
  Hypothesis: <one line>.
  Mechanism falsified: `<specific mechanism + class>` — <observed reason>.
  Untried siblings enumerated: <comma-separated list of ≥3 untried alternatives in same class>.
  Adversarial-pair audit: `<path-to-analysis/<path>_adversarial_alternatives.md>` (or "n/a — class fully enumerated, see <file>").
  Prior-breakthrough audit: clean (or: "REFUSED, revived as `<new-path-name>`").
  Replaced by: `<successor-path-name>` (if any).
```

The planner MUST read `falsified.md` before proposing new paths, to avoid re-spawning falsified hypotheses under new names. The planner ALSO checks the `Untried siblings` field — if any row lists untried siblings, those are first-class backlog candidates the planner SHOULD propose before generating fresh hypotheses.

## `analysis/revivals.md`

Append-only log of revivals — a `mechanism-dropped` / `path-dropped` row that a later session proved was reachable after all. Each entry:

```
- YYYY-MM-DD — original `<path>` (dropped YYYY-MM-DD by `<wname>`, mechanism `<mech>`).
  Revived as `<new-path>` via mechanism `<new-mech>`.
  Evidence: `<artifact path + sha256 prefix>`.
  Root cause of premature closure: <one line — e.g. "mechanism-not-path conflation", "missing adversarial audit", "stale facts not cross-checked">.
```

**Revival rate** is the orchestrator-quality metric. Report it in the cycle report every K=6 cycles as `revivals_this_K / falsifications_this_K`. A rate >0.2 means the orchestrator is closing paths too aggressively — the Falsification scoping and Adversarial enumeration rules need tightening (more candidate mechanisms in briefings, longer adversarial budget, stricter audit triggers). A rate of 0 over multiple K windows means closures are well-grounded.
