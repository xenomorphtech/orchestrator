## typed-v11-long-tail — Discovery + batch-add placeholder variants for remaining long-tail length classes

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v11-long-tail` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v11-long-tail`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v11_long_tail` (new)
- **sub_goal_key**: `discovery-plus-placeholder-for-long-tail-length-classes`

## Why this turn exists
Cycle 476 v10-placeholder shipped FRzC2SLen10Placeholder at 83.33% coverage (+7.51pp via labeled-placeholder pattern). Remaining 7,998 frames (16.67%) live in long-tail length classes that didn't qualify for the catalog-binding push (v3→v9). This cycle applies the v10 pattern (labeled placeholder) to ANY remaining length class with ≥50 unclassified frames, bundled or sub-bundled appropriately based on what discovery reveals.

The classifier doesn't gain new RE knowledge — it gains a structural acknowledgment that "frames of length N exist in the corpus" without claiming to name the underlying typename. Same trade-off as v10 (preserved invariant via labeling).

## Hypothesis
At least 4 unclassified length classes (across C2S and S2C combined) have ≥50 unclassified frames each. Adding labeled-placeholder variants for each lifts coverage 83.33% → ≥90.0% (+6.67pp combined).

## Falsification (3 outcomes)
- (a) **≥4 placeholder variants added + cargo build OK + coverage ≥90.0% + v10 regression check passes** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v11_<coverage>_long_tail_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v11_break`.
- (c) **Coverage moves but <90.0% (e.g., insufficient large unclassified classes)** → name what's defensible. Fact: `dark_december_decoder_rust_typed_v11_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v10-placeholder baseline** from `/home/sdancer/dark-december-typed-v10-placeholder/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Discovery sub-step (≤4 min)**:
   - Run v10 decoder on `first_quest_{c2s,s2c}.tcpstream.bin`.
   - Tabulate **unclassified** frames (those landing in the `Unknown` variant) by `(direction, raw.len())`.
   - Report top-15 (direction, wire_len) tuples by unclassified frame count.
   - For each tuple with ≥50 frames, examine `decoded[0..2]` distinct values — single shape vs multiplexed.

3. **Add labeled-placeholder variants** for each qualifying length class. Naming convention:
   ```rust
   /// PLACEHOLDER (pending RE) — N frames in first_quest corpus at (direction=X, wire_len=W).
   /// Decoded body shape: single | M-way multiplexed by decoded[K].
   FRzC2SLen<W>Placeholder { ... }  // for C2S
   FRzS2CLen<W>Placeholder { ... }  // for S2C
   ```
   - Each variant carries a `body: Vec<u8>` of the decoded bytes.
   - If multiplexed, also include `subtype: u8` taken from the discriminating byte.
   - Classifier rules: `direction == X && raw.len() == W && decoded.len() == W-7` (or W-6 for non-standard framing if any).

4. **Validate**: cargo build --release; run on streams; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - Per-new-variant frame counts.
   - Confirmation that v10-typed-variant frame counts are UNCHANGED.

5. **Diff vs v10-typed CSV** (regression check): `out_v10_baseline/typed_packets.csv` vs `out_v11_orch/typed_packets.csv` — every row present in v10 must be present in v11 with identical fields.

6. **Write `analysis/typed_v11_long_tail_2026-05-16.md`** (≤150 lines):
   - Discovery table: top-15 unclassified (direction, wire_len, count, decoded shape).
   - Eligibility decision per class (added vs skipped reason).
   - Coverage table (v10 83.33% → v11 X.XX%).
   - Per-variant frame counts.
   - Regression confirmation.
   - Saturation discussion: what fraction of remaining frames is now classified, what's left, and at what frame-count threshold further bundling stops being defensible.

7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V11_LONG_TAIL_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤150 lines.
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **IMPORTANT**: After writing the artifact, fact-set, and final TYPED_V11_LONG_TAIL_DONE marker, EXIT IMMEDIATELY.
- Do NOT name placeholder variants after specific FRz* typenames — the placeholder name is the whole point.
- Do NOT change v10 variant classifications.
- Floor for adding a variant: 50 frames. Skip anything smaller (sub-pp gain).

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v10-placeholder/`
- Cycle-408 S2C inventory: `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
- Cycle-390 C2S decoder: `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md`
- Cycle-476 v10 placeholder artifact: `/home/sdancer/dark-december-typed-v10-placeholder/analysis/typed_v10_placeholder_2026-05-16.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- success-fact: `dark_december_decoder_rust_typed_v11_<coverage>_long_tail_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v11_break` (b), `dark_december_decoder_rust_typed_v11_partial` (c)
