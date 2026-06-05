# The 6-point validity checklist — full rationale

The negative-result critic must answer all 6 of these for any negative claim it's adjudicating. Each point is paired with a case-study anchor from prior orchestrator sessions (encoded as `[[wiki-style-anchor]]` — these reference the project's local memory; if absent, treat them as illustrative).

## 1. Precondition met?

**Was the thing-under-test actually exercised — right substrate, right state, target process alive, traffic flowing, the code path reached?**

This is the single most common cause of false negatives. The probe is "set up" but the *thing being probed* is not actually being touched.

Case-study anchors:
- `[[verify-precondition-before-probe]]` — general rule: confirm preconditions before interpreting a probe result.
- `[[substrate-state-invalidation]]` — 0 fires when the app is at a title screen means nothing; the app is at a different state than the test assumes.

**How to check it (concretely, not abstractly):**
- Is the target process alive (not just installed)? Check via the substrate's own process check, not via "the host is up".
- Is the target in the state the probe assumes? (e.g. "user is on the home screen" — verify with a screenshot or state read.)
- Is traffic flowing? (e.g. for a network probe, is the interface actually carrying the expected packets?)
- Did the code path actually run? (e.g. for an instrumentation hook, did the hook fire even once?)

If any precondition is unverified, the result is **inconclusive**, not negative.

## 2. Mechanism implemented correctly?

**Was the failure the *hypothesis* failing, or the *harness* (wrong offset, wrong tool invocation, a dispatcher fighting itself)?**

Workers default to blaming the hypothesis. The critic's job is to ask: is the test even exercising the hypothesis correctly?

Case-study anchor: `[[xdotool-unity-albion-blocked]]` — the apparent "xdotool blocked by Unity" was self-inflicted Escape-spam from a dispatcher fighting itself, not the substrate.

**How to check it (concretely):**
- Was the tool invoked with the right arguments (not "the obvious defaults")?
- Was the offset/path/selector correct, verified by the tool's own echo or by a side-channel read?
- Is there a dispatcher / queue / retry-loop that could be interfering?
- Did the tool itself succeed at the lowest level (e.g. xdotool reported a successful keystroke), even if the effect wasn't observed?

If the lowest-level invocation didn't succeed, the result is **harness-failure**, not hypothesis-failure. Re-test with a corrected harness before believing the negative.

## 3. Capability/access actually blocked, or privilege-context?

**Verify caps/perms on the host that runs the target, not the orchestrator's shell; remember sudo→root, LKM, and `ptrace_scope` toggles defeat most apparent blocks.**

The orchestrator runs in shell A. The target runs on host B (which may be the same box, a different user, a different container, a different VM, a different device). Caps/perms verified on shell A mean nothing for host B.

Case-study anchor: `[[capability-block-verify-on-host]]` — general rule: verify on the host that runs the target.

**How to check it (concretely):**
- `capsh --print` or `getcap` on the *target's binary*, not on the test harness.
- For ptrace: check `/proc/sys/kernel/yama/ptrace_scope` on the *target host*.
- For network: check `CAP_NET_RAW`, `CAP_NET_ADMIN` on the *target host's user*.
- For filesystem: check the actual user the target runs as (`ps aux | grep`), not the orchestrator's user.
- sudo→root on the orchestrator's host does NOT propagate to a different host, container, or device.

If the cap is missing on the right host, the result is **needs-elevated-privilege**, not "impossible". The remedy is elevation, not closure.

## 4. Is it on the critical path at all?

**Often a sibling formulation sidesteps the "blocker" entirely.**

The worker reports "X is blocked". The critic asks: do we even need X? Is there a sibling formulation that achieves the goal without touching X at all?

**How to check it (concretely):**
- Re-read the goal's success criteria. Is X the only way to satisfy them?
- Are there alternative mechanisms in the same class that the worker hasn't tried (this is the *adversarial enumeration* step — but the critic can flag that the worker's framing assumed X is on the critical path when it might not be)?
- Example: clientless own-DH (Diffie-Hellman) sidesteps reading the client's key. The "can't read the key" blocker is irrelevant if you don't need the key.

If X is off the critical path, the "blocker" is a non-issue — re-frame, don't close.

## 5. Measurement valid?

**Thin-sample noise vs real signal, right detector, reproduced ≥2×?**

A single probe with thin samples is not a measurement. The critic's job is to ask: would a second probe (or a different detector on the same phenomenon) reproduce?

Case-study anchor: `[[revival-rate-thin-sample-trap]]` — thin samples masquerading as negative results are a common cause of premature closure.

**How to check it (concretely):**
- How many probes were run? Is that enough to rule out the noise floor?
- Was the detector the right one? (e.g. detecting "no packet capture" via tcpdump is right; detecting it via "app log didn't mention packets" is wrong.)
- Can a second probe reproduce? If a single run is the only evidence, the result is **inconclusive**, not negative.
- Is there a positive control? (i.e. did the detector fire on a known-good case?)

If measurement is thin, the result is **needs-more-samples**, not "doesn't work".

## 6. First-principles check

**Is the asserted impossibility actually a law, or one mechanism's failure being over-generalized?**

A worker reporting "X is impossible" should be able to cite the law (physical, cryptographic, OS-level) that makes it impossible. If they can only cite "I tried mechanism A and it didn't work", that's a mechanism failure, not an impossibility.

Case-study anchors:
- `[[packet-injection-never-impossible]]` — "packet injection is impossible" is almost always a mechanism-failure, not a law. Layer-2 injection on loopback is well-known; AF_PACKET, raw sockets, eBPF, and LD_PRELOAD shims all exist.
- `[[impossibility-caution]]` — general rule: be skeptical of "impossible" claims, especially from optimistic workers.

**How to check it (concretely):**
- Is the impossibility grounded in a verifiable law (e.g. "AES-128 is computationally infeasible to break")? If yes → the negative is real. If no → it's over-generalization.
- Are there documented mechanisms in the same class that succeed? (e.g. for "X is impossible", search for "X tutorial" / "X PoC" — if hits exist, the impossibility is mechanism-specific, not categorical.)
- Is the worker citing a *single* failed attempt as proof of impossibility, or a *class* of failed attempts?

If the impossibility is over-generalized, the result is **mechanism-failure**, not "impossible" — proceed to adversarial enumeration in the same class.

---

## Putting it together: the verdict

The critic runs all 6 points. The verdict is the *intersection* of the answers:

- If any point fails, the result is not a negative — re-test with the gap fixed.
- If all 6 pass, the negative is **CONFIRMED** (for *this* mechanism, on *this* substrate, with *this* harness). The orchestrator can now proceed to falsification scoping + adversarial enumeration + prior-breakthrough audit.
- If even one point is "unresolved" or "needs re-test", the verdict is **REFUTED** (in the broad sense — the claim as stated is not yet admissible; re-test, do not record closure).

The "CONFIRMED for this mechanism" framing matters: a CONFIRMED negative does not mean "the path is dead". It means "this mechanism in this class is dead". The next step (adversarial enumeration) asks what other mechanisms in the class might still work.
