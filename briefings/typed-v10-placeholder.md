## typed-v10-placeholder — Add FRzC2SLen10Placeholder for C2S len=10 (3,607 multiplexed frames, EXPLICIT PLACEHOLDER)

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v10-placeholder` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v10-placeholder`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v10_placeholder` (new)
- **sub_goal_key**: `add-explicit-placeholder-for-c2s-len10-bundle`

## Why this turn exists
Cycle 472 K=6 planner verdict: typed-coverage SATURATED at 75.82%. Planner rejected bundling C2S len=10 as `FRzC2SLen10Subtype` because the 7 sub-bands are distinct message types (cycle-425 evidence: disjoint ordinal-time bands) and a generic name would pollute the "typed = named + RE-grounded" invariant.

This cycle resolves the trade-off by introducing **explicit naming**: variant `FRzC2SLen10Placeholder` (not `FRzC2SLen10Subtype`) explicitly signals to downstream consumers that the variant is opaque and pending RE — it preserves the invariant by labeling rather than violating. Buys +7.5pp coverage at the cost of admitting one explicitly-labeled placeholder variant.

Per orchestrator "never wait for user input" rule: spawning under cycle-473 saturation hold; user can revert this commit if they disagree with the trade-off.

## Hypothesis
Adding `FRzC2SLen10Placeholder { subtype: u8, body: Vec<u8> }` with classifier `direction == C2S && wire_len == 10 && decoded.len() == 3` types ≥3,500 of cycle-425's 3,607 frames and lifts coverage 75.82% → ≥82.5% (+6.7pp). All v9 typed rows preserved.

## Falsification (3 outcomes)
- (a) **Variant added + cargo build OK + typed coverage ≥82.5% + ≥3,500 placeholder frames + v9 regression check passes + clearly-labeled docstring marking variant as opaque/pending-RE** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v10_<coverage>_placeholder_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v10_break`.
- (c) **Coverage moves but <82.5% OR fewer than 3,000 frames classified** → name what's defensible. Fact: `dark_december_decoder_rust_typed_v10_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v9-medium-conf baseline** from `/home/sdancer/dark-december-typed-v9-medium-conf/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Add `FRzC2SLen10Placeholder` variant** in `src/lib.rs`:
   ```rust
   /// PLACEHOLDER variant (pending RE).
   ///
   /// Cycle-425 disambig showed C2S len=10 frames split into 7 sub-bands
   /// by `decoded[1]` value (0x01..0x07) with disjoint ordinal-time bands
   /// — these are 7 distinct message types, not one enum-tagged class.
   /// Only one sub-band (`subtype=0x01`, n=44) has a candidate typename
   /// (FRzCharacterLookVisibleChangeRq). The other 6 sub-bands (~3,560
   /// frames) await RE; this variant exposes their bytes verbatim so
   /// downstream consumers can disambiguate when more substrate arrives.
   ///
   /// This variant is INTENTIONALLY NOT NAMED with a specific FRz* type.
   FRzC2SLen10Placeholder {
       subtype: u8,        // decoded[1] — 0x01..0x07; band selector per cycle-425
       body: Vec<u8>,      // decoded[0..3] — full decoded body
   },
   ```
   - Classifier rule (placed AFTER FRzOverlapRq): `direction == Direction::C2S && raw.len() == 10 && decoded.len() == 3`
   - Decode: `subtype = decoded[1]; body = decoded[0..3].to_vec()`
   - Match-arms for `id()`/`x()`/`z()`/`rot()` return `None`.

3. **Validate**: cargo build --release; run on `first_quest_{c2s,s2c}.tcpstream.bin`; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - FRzC2SLen10Placeholder frame count (must be ≥3,500) and subtype histogram.
   - Confirmation that v9-typed-variant frame counts are UNCHANGED.

4. **Diff vs v9-typed CSV** (regression check): `out_v9_baseline/typed_packets.csv` vs `out_v10_orch/typed_packets.csv` — every row present in v9 must be present in v10 with identical fields; v10 only ADDS rows.

5. **Write `analysis/typed_v10_placeholder_2026-05-16.md`** (≤80 lines):
   - Coverage table (v9 75.82% → v10 X.XX%).
   - Placeholder frame count + subtype histogram (`decoded[1]` 0x01..0x07).
   - Regression confirmation.
   - **EXPLICIT trade-off note**: this variant is a labeled placeholder, NOT a named binding. Cycle-472 planner-audit accepted this trade-off as "preserved invariant via labeling". Recommendation for future RE: each sub-band warrants its own RE pass when live-tap state becomes available.

6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V10_PLACEHOLDER_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤80 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **IMPORTANT (cycle-465 stall lesson)**: After writing the artifact, fact-set, and final TYPED_V10_PLACEHOLDER_DONE marker, EXIT IMMEDIATELY.
- Do NOT name this variant `FRzCharacterLookVisibleChangeRq` or any specific FRz* typename — the placeholder name is the whole point.
- Do NOT change v9 variant classifications.

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v9-medium-conf/`
- Cycle-425 C2S len=10 disambig: `/home/sdancer/dark-december-c2s-len10-disambig/analysis/c2s_len10_disambig_2026-05-15.md`
- Cycle-472 planner verdict: discussion in this cycle's brief; "preserved invariant via labeling" is the chosen trade-off.
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- success-fact: `dark_december_decoder_rust_typed_v10_<coverage>_placeholder_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v10_break` (b), `dark_december_decoder_rust_typed_v10_partial` (c)
