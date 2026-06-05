---
name: critic
description: "Use when validating a worker's claim — either a POSITIVE approach choice about to be committed to (framing critic) or a NEGATIVE result about to be recorded, propagated, or relayed (mandatory negative-result critic). Refuses to let the loop sink ticks into an unchallenged framing or write a false negative into a ledger."
version: 1.0.0
author: Hermes Agent (extracted from orchestrate.md)
license: MIT
platforms: [linux]
metadata:
  hermes:
    tags: [critic, validity, falsification, framing, negative-results, multi-agent, orchestrator]
    related_skills: [orchestrate, plan, systematic-debugging, requesting-code-review]
---

# Critic

The critic is the orchestrator's **validity-gate machinery** for two distinct claim types that both have a common failure mode: they get *believed* and *written down* before they're checked, and once they're in a ledger or cross-pollinated as a fact, they prune the search space for every future session.

The two critics:

| Sub-procedure | Trigger | Question it answers |
|---|---|---|
| **Framing critic** (positive approach-choices) | Before a worker sinks >1 tick into an approach, OR before the orchestrator reports a state-derived claim | *Is the approach itself well-framed — or are we about to commit to a wrong surface, an unconfirmed assumption, or an unchallenged "obvious" choice?* |
| **Negative-result critic** | Before any negative claim is accepted, recorded, propagated, or relayed | *Is the worker's "X is impossible / blocked / exhausted" claim actually true, or is it a false negative?* |

Both are **BLOCKING** — they must pass before the action they're gating. Both can be run inline (routine) or by spawning an independent critic worker (path-/goal-level).

This skill was extracted from `orchestrate.md` (the `Framing critic on POSITIVE approach-choices` and `Mandatory critic on EVERY negative result` sections). The orchestrator that triggers them lives in the `orchestrate` skill; load it for context on *when* in the control loop these fire.

---

## When to Use

- A worker (or you, in a worker role) is about to commit to an approach that hasn't been challenged.
- A worker has reported something "doesn't work", "is blocked", "is impossible", "is exhausted" — and you need to decide whether to believe it before recording it.
- A state-derived claim (a metric value, a status, a "done" assertion) is about to be relayed — and you want to verify the ledgers are not stale.
- A path has churned ≥2 ticks on the same mechanism without a metric move (churn is the signal the framing — not just the mechanism — may be wrong).

**Do NOT use for:**
- Positive successes that are already verified by concrete artifacts (no critic needed).
- Capability claims that are **positive** ("I can read X", "the key is Y") — those need verification, not this critic.
- The "what else should I try?" question — that's *adversarial enumeration*, a different procedure. See `orchestrate` skill → *Adversarial enumeration*.
- The "has this goal already been achieved elsewhere?" question — that's *prior-breakthrough audit*, also a different procedure. See `orchestrate` skill → *Prior-breakthrough audit*.

---

## Sub-procedure 1: Framing critic on positive approach-choices

The most frequent real-world miss in a control loop is the **opposite of failure**: committing ticks of effort to an **unchallenged framing** — wrong surface, wrong tool, an unconfirmed assumption, stale self-reported state, or doctrine already in memory that the loop ignored. These are not "negatives," so they bypass the negative critic entirely, and the **user ends up supplying the reframe** the loop should have generated.

Same-session evidence: pixel-poking a Unity form for ~6 ticks when an in-process call existed; testing account-creation on the *web form* when the pipeline uses the *Linux client*; "work around" an unconfirmed email failure instead of empirically confirming it; goals-table/DB-path drift reported as truth; an existing memory entry that the user had to re-surface.

**HARD RULE: before a worker sinks >1 tick into an approach — and before the orchestrator reports a state-derived claim — run the framing critic.** It asks, of the chosen *approach* (not the failure):

1. **Right surface?** Web vs native client, UI vs in-process/API/memory, pixel-poke vs programmatic call. The visible/easy surface is usually NOT the right one. **Name the surface explicitly and justify it over the alternatives.**
2. **Assumption empirically confirmed, or just assumed?** If the approach rests on an unverified premise ("the email isn't arriving", "the field needs a click"), run the *cheapest empirical test FIRST* — do not build on or "work around" an unconfirmed premise.
3. **More-direct mechanism available?** Is there a programmatic/in-process path that skips the brittle outer layer entirely (inject-the-call vs drive-the-GUI; own-DH vs read-the-key)?
4. **Does memory/doctrine already prescribe this?** Grep the memory index + facts for the decision area BEFORE defaulting to the easy tool — applicable doctrine recalled reactively (after a user nudge) is a process miss.
5. **Is the state I'm reporting reconciled to ground truth THIS tick?** Ledgers (DB paths, goals table, metrics) drift. Before reporting a metric/status as fact, cross-check it against the latest fact/worktree/screenshot evidence — never relay stale metadata as current truth.

**Verdict format:** Record a one-line framing verdict in the episode:

```
Framing verdict for <worker/path>: <PROCEED | REFINE | REFUSE> — <one-line reason, naming surface/assumption/mechanism/doctrine/state>
```

When to record the verdict:
- A worker is about to commit >1 tick to a non-obvious surface/tool.
- A path has churned ≥2 ticks on the same mechanism without a metric move (churn is the framing signal).
- A state-derived claim is about to be relayed as fact.

A worker defaulting to xdotool/web-form/requests/manual-clicks without the orchestrator having surfaced the more-direct surface is a **framing-critic miss**.

---

## Sub-procedure 2: Mandatory critic on every negative result

A **negative result** is any worker output that asserts something did not / cannot work: `blocked`, `falsified`, `impossible`, `no path forward`, `unreachable`, `exhausted`, but ALSO the quieter forms — "no effect", "0 samples / 0 fires", "test failed", "didn't advance", "not detected", "inconclusive", "no movement", "doesn't apply", a capability/access claim ("can't read X / no permission / cap missing"), or a relayed-from-a-worker "X is not possible".

**Negative results are the single most expensive thing to get wrong** — a false negative written to a ledger or cross-pollinated as a fact prunes the search space for every future session and silently kills live paths. They are also the orchestrator's most common error.

**HARD RULE: no negative result may be accepted, recorded, propagated, or relayed until it passes a critic.** This gate runs BEFORE the adversarial-enumeration and prior-breakthrough-audit steps (those decide *what else to try* and *whether it was already done*; the critic decides *whether the negative result is even true*). The orchestrator MUST NOT pass a worker's negative claim through to the user, to `falsified.md`/`hypotheses.md`/DB path rows, to a `*_blocked.md`/`*_falsified.md` artifact, or to a `fact-set` until the critic has run.

The critic challenges the **validity** of the negative result (distinct from enumerating alternatives). It must answer all of the 6 points in the checklist below. For the full checklist with the rationale and case-study anchors behind each point, see [references/six-point-checklist.md](references/six-point-checklist.md).

### The 6-point validity checklist

1. **Precondition met?** Was the thing-under-test actually exercised — right substrate, right state, target process alive, traffic flowing, the code path reached? (0 fires when the app is at a title screen means nothing.)
2. **Mechanism implemented correctly?** Was the failure the *hypothesis* failing, or the *harness* (wrong offset, wrong tool invocation, a dispatcher fighting itself)?
3. **Capability/access actually blocked, or privilege-context?** Verify caps/perms on **the host that runs the target**, not the orchestrator's shell; remember sudo→root, LKM, and `ptrace_scope` toggles defeat most apparent blocks.
4. **Is it on the critical path at all?** Often a sibling formulation sidesteps the "blocker" entirely.
5. **Measurement valid?** Thin-sample noise vs real signal, right detector, reproduced ≥2×?
6. **First-principles check:** is the asserted impossibility actually a law, or one mechanism's failure being over-generalized?

### How to run it (scale to the claim)

**Routine / mechanism-level negative** (a single probe came back empty):
- Run the 6-point checklist **inline this tick** and record the verdict in the episode.
- If any point is unresolved, the result is **not yet a negative** — it's an inconclusive probe; re-run with the gap fixed before believing it.

**Path- or goal-level negative**, or any negative about to hit a ledger/fact/user report:
- Spawn an **independent critic worker** in its own worktree (no shared state with the original worker), 20-min cap. The prompt template is in [references/critic-worker-prompt.md](references/critic-worker-prompt.md).
- The critic worker returns `analysis/<path>_critic.md` with a verdict: **CONFIRMED** or **REFUTED**.

### Verdicts

- **REFUTED** → the negative is rejected. Re-task the original worker with the critic's re-test; do NOT record any closure. Log it (a refuted negative is a near-revival — track toward the revival-rate quality metric).
- **CONFIRMED** → the negative is real *for this mechanism*. Now proceed to Falsification-scoping + Adversarial-enumeration + Prior-breakthrough-audit as usual (those live in the `orchestrate` skill). The critic's `analysis/<path>_critic.md` becomes part of the closure paper trail.

### The discipline

The critic is the orchestrator's own discipline, not the worker's — workers are optimistic about their own negatives ("I tried, it didn't work"); the orchestrator's job is to refuse to believe a negative until it has survived an adversarial validity check. **Never relay a worker's negative result to the user as fact without having run this gate.**

---

## What this skill is NOT

| Procedure | What it answers | Where it lives |
|---|---|---|
| **Framing critic** (this skill) | *Is the approach well-framed?* | This skill |
| **Negative-result critic** (this skill) | *Is the negative claim valid?* | This skill |
| **Adversarial enumeration** | *What else in the same mechanism class is untried?* | `orchestrate` skill |
| **Prior-breakthrough audit** | *Has this goal-area already been achieved elsewhere?* | `orchestrate` skill |
| **Falsification scoping** (mechanism ≠ path) | *What status does the closure deserve — `mechanism-dropped` or `path-dropped`?* | `orchestrate` skill |

The critic runs FIRST. The others run only after a critic verdict of CONFIRMED.

---

## Common Pitfalls

1. **Skipping the critic because "the worker sounded sure".** Workers are optimistic about their own negatives. The whole point of the critic is to be skeptical *especially* when the worker is sure.
2. **Running the framing critic on a negative result, or the negative critic on a positive approach.** They are different questions with different checklists. Use the right one.
3. **Letting the critic become "let me think about it" and then accepting whatever the worker said.** The critic must produce an explicit verdict (PROCEED/REFINE/REFUSE for framing; CONFIRMED/REFUTED for negative) and the verdict must be recorded.
4. **Spawning a critic worker for routine mechanism-level negatives.** Routine = inline. Spawning a critic worker is for path-/goal-level claims and any claim about to hit a ledger. Spawning for a routine probe wastes the 20-min cap and dilutes the audit's value.
5. **Stopping after the critic REFUTES a negative.** REFUTED is a near-revival — the original worker gets re-tasked with the re-test, the search space is reopened, and the refutation itself becomes a tracked metric.
6. **Using the critic to enumerate alternatives.** The critic adjudicates validity ("is this claim even true?"). Enumerating alternatives ("what else should I try?") is a separate worker and a separate audit; running them together produces a muddled verdict.
7. **Letting state-derived claims skip the framing critic.** "The goal is at metric 4/5" sounds factual but may be reading stale ledger data. The state-reconciliation question (#5 in the framing checklist) exists exactly for this.
8. **Treating a "verdict recorded in the episode" as a substitute for the actual check.** A verdict without a per-point rationale (which of the 6 points, what did you check, what's the result) is theater. Inline the rationale.

---

## Verification Checklist

Before accepting a critic verdict as gating, confirm:

**For framing verdicts:**
- [ ] Each of the 5 framing questions has an explicit answer (not "looks fine").
- [ ] The surface is named explicitly (not "the obvious one") and the alternative is named.
- [ ] If the approach rests on an unverified premise, the cheapest empirical test was named (and ideally run).
- [ ] Memory / facts were actually grepped, not assumed to be empty.
- [ ] State-reconciliation step actually re-read the latest fact/worktree/screenshot, not a cached summary.
- [ ] Verdict (`PROCEED` / `REFINE` / `REFUSE`) is recorded in the episode with a one-line reason.

**For negative-result verdicts (inline routine case):**
- [ ] Each of the 6 points has a concrete answer ("precondition: yes — process alive, traffic seen, log line N confirms reach" — not "looks ok").
- [ ] At least one alternative mechanism in the same class was considered (not necessarily enumerated — just *considered*).
- [ ] If any point is unresolved, the result was reclassified as **inconclusive** (not yet a negative) and the gap was named.
- [ ] Verdict (CONFIRMED / REFUTED) is recorded in the episode.

**For negative-result verdicts (spawned critic worker case):**
- [ ] The critic worker has its own worktree (no shared state with the original worker).
- [ ] The critic worker is time-boxed (20-min cap) and will be retired regardless of completion.
- [ ] The critic's output lives at `analysis/<path>_critic.md` and references each of the 6 points.
- [ ] The verdict is binary (CONFIRMED or REFUTED) and the re-test for REFUTED is concrete enough to re-task the original worker.
- [ ] The critic's output is part of the closure paper trail (or the rejection rationale) — not orphaned in a worker pane.

---

## One-Shot Recipes

### Inline framing critic before committing to an approach

```bash
# Worker says: "Let's pixel-poke the form to drive button X."
# Surface = pixel-read of the form, then a synthetic mouse click at coord Y.
echo "Framing verdict for worker/wpath:"
echo "  1. Right surface? Form is Unity (in-process call exists in PlayerLoop). FAIL — surface is wrong."
echo "  2. Assumption? That the click reaches the bound handler. Unconfirmed."
echo "  3. More-direct mechanism? SendMessage / in-process reflection. YES — try that first."
echo "  4. Doctrine? 'one-fresh-account-per-instance / pivot-to-programmatic-runner' — applies."
echo "  5. State reconciled? Fact: last verification of form bound 2026-05-XX — current."
echo "  Verdict: REFUSE — pivot to in-process call before sinking ticks into pixel-poke."
# → Send corrective directive to worker.
```

### Inline 6-point critic on a routine "0 samples" negative

```bash
# Worker says: "I ran the packet capture 5x and got 0 samples — packet injection impossible."
echo "Critic verdict on negative 'packet injection impossible':"
echo "  1. Precondition? Target process alive? pcap interface correct? → NO — wrong interface. Inconclusive."
echo "  2. Mechanism correct? tcpdump filter matches expected traffic? → Not verified."
echo "  3. Capability? CAP_NET_RAW on the host that runs the target? → Not checked on target host (only on orchestrator shell)."
echo "  4. Critical path? Yes if pcap is the only observability. But app-side logging also exists."
echo "  5. Measurement? 5 captures thin. Reproduced? Once."
echo "  6. First principles? Layer-2 injection on loopback is well-known to need raw sockets or AF_PACKET. Not impossible."
echo "  Verdict: REFUTED. Re-run with correct interface, app-side log, and CAP_NET_RAW verified on target host."
# → Do NOT record 'packet-injection-impossible' as a fact. Re-task worker.
```

### Spawn a critic worker for a path-level negative

```bash
# Worker pane contains: "Path X is exhausted, no path forward."
# → Spawn an independent critic worker before accepting the closure.
WORKTREE=/path/to/critic-worktree
git -C <repo> worktree add "$WORKTREE" -b critic-<path>-<ts>
# Persist critic briefing (own worktree, no shared state, 20-min cap)
# Use the prompt template at references/critic-worker-prompt.md, substituting:
#   <wname>, <claim>, <path>
harness send critic-<path> "<prompt from references/critic-worker-prompt.md>"
# Wait for analysis/<path>_critic.md; read verdict; record in episode.
# On CONFIRMED → proceed to falsification-scoping + adversarial-enum + prior-breakthrough (in orchestrate skill).
# On REFUTED → re-task original worker with the re-test.
```

---

## See also

- `orchestrate` skill — the control loop that triggers both critics and owns the surrounding machinery (adversarial enumeration, prior-breakthrough audit, falsification scoping).
- `references/six-point-checklist.md` — the 6-point negative-result checklist with rationale and case-study anchors behind each point.
- `references/critic-worker-prompt.md` — drop-in prompt template for spawning an independent critic worker on path-/goal-level negatives.
- `/home/sdancer/orchestrator/orchestrate.md` — original full text (source of truth, ~700 lines).
