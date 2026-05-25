## s2c-disambig — Promote 10 medium-confidence S2C bindings via stream-order lift

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-s2c-disambig` (worktree of `/home/sdancer/darkdecember/`, branch `s2c-stream-order-v2`).

## Current goal / sub-goal
- **goal_key**: `dark_december_s2c_bindings_v2` (new)
- **sub_goal_key**: `promote-medium-conf-s2c-bindings-via-temporal-lift`

## Why this turn exists
Cycle 412's `rz-typename-binding` produced **5 high-confidence + 10 medium-confidence** length-class → Rz typename bindings. The medium ones (`FRzInteractionBr` at S2C len=28, `FRzTeleportRp` at S2C len=50, `FRzTriggerActiveBr` at S2C len=27, `FRzRotationBr` at S2C len=40, `FRzCoolTimeResetNoti` at S2C len=32, and ~5 more) were scored on **size match alone**. The missing evidence is **temporal lift**: every server `Br` / `Rp` should fire within N frames after a structurally-compatible client `Rq`. Cycle 412's c2s-s2c-join produced a stream-byte-fraction timeline + ±20-ordinal lift matrix. Joining the two artifacts moves these bindings from medium → high.

## Hypothesis
For each medium-confidence (S2C-class, candidate-typename) pair from cycle-412's binding table, computing P(S2C-class fires within ±20 ordinals after a C2S-class whose typename is the matching Rq) divided by the marginal probability produces a lift score that distinguishes correct bindings (lift ≥ 5) from spurious size-matches (lift near 1).

## Falsification (3 outcomes)
- (a) **≥5 medium-conf bindings clear lift ≥ 5** against their hypothesized matching Rq, with the rest of the 822-typename catalog producing lift < 2 → SUCCESS. Fact: `dark_december_s2c_bindings_v2_<n>_promoted`.
- (b) **No medium-conf binding clears lift ≥ 2 above marginal** → the response-pairing assumption is wrong; the S2C broadcasts are server-pushed and not request-correlated. Fact: `dark_december_s2c_bindings_v2_no_temporal_pairing`.
- (c) **Partial promotion** (1–4 bindings clear lift ≥ 5) → name the promoted set, document which medium ones stay low-confidence and why. Fact: `dark_december_s2c_bindings_v2_partial_<n>_promoted`.

## Success criteria — SINGLE TURN, do all of these before stopping
**Primary**: write `/home/sdancer/dark-december-s2c-disambig/analysis/s2c_bindings_v2_2026-05-15.md` with:

1. **Load cycle-412 medium-conf table** from `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md` and the support JSON. Extract the 10 medium-confidence (S2C length, candidate typename) pairs.
2. **Load cycle-412 c2s-s2c-join lift matrix** from `/home/sdancer/dark-december-c2s-s2c-join/analysis/c2s_s2c_pairs.csv`. This gives the conditional probabilities P(S2C class | preceding C2S class within ±20 ordinals).
3. **For each medium-conf (S2C-len, candidate-typename) pair**: identify the matching Rq (e.g. `FRzInteractionBr` matches `FRzInteractionRq` at C2S len=32 per the cycle-412 size table). Compute the temporal lift specifically for that C2S→S2C pair.
4. **Output `analysis/s2c_stream_order_lift.csv`** with columns: `s2c_len, s2c_typename_candidate, matching_c2s_len, c2s_typename_candidate, hits, conditional_prob, marginal_prob, lift, verdict_high_med_low`.
5. **Promote bindings** with lift ≥ 5 to high; demote those with lift < 2 to low; keep medium for 2 ≤ lift < 5. Report per-binding the promotion.
6. **Cross-check**: for each promoted binding, take 3 frame examples from the C2S+S2C streams showing the pairing in stream-byte order; include in the artifact for human verification.
7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `S2C_BINDINGS_V2_DONE` on the final line.

## Execution flow

**Step 1** — Read the rz_binding artifact + the c2s-s2c-join CSV.
**Step 2** — For each medium-conf candidate, compute lift specifically vs the matched Rq. Compare against same-length C2S-class lift to control for ambient ACK-style pairing.
**Step 3** — Build the CSV + markdown.
**Step 4** — Single fact-set at end.

## Constraints & gotchas
- **HARD memory budget: 250 MB.** All inputs are CSVs and small JSONs.
- **NO disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤300 lines.
- **ONE Codex turn budget: ≤25 min wall time. SINGLE-TURN COMPLETION REQUIRED** — do all 7 steps before stopping; do not pause at step boundaries.
- Reference artifacts (READ-ONLY):
  - cycle-412 rz binding: `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md` + `rz_binding_support_2026-05-15.json`
  - cycle-412 c2s-s2c join: `/home/sdancer/dark-december-c2s-s2c-join/analysis/c2s_s2c_pairs.csv` + `c2s_s2c_pairs_2026-05-15.md`
  - cycle-408 s2c inventory: `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
  - 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-s2c-disambig/` (branch `s2c-stream-order-v2`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- success-fact key: `dark_december_s2c_bindings_v2_<n>_promoted` (a)
- block-fact keys: `dark_december_s2c_bindings_v2_no_temporal_pairing` (b), `dark_december_s2c_bindings_v2_partial_<n>_promoted` (c)
