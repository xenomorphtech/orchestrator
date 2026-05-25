## typed-v7-8400 — Add FRzMovementCoreReply8400 variant for S2C len=39 `8400e1e2` core family (528 frames)

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v7-8400` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v7-8400`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v7_8400` (new)
- **sub_goal_key**: `add-frzmovementcorereply8400-variant-for-s2c-len39`

## Why this turn exists
Cycle 464 typed-v6-9400 landed at 73.08% (+1.52pp). Cycle 449 s2c-len39 analysis identified an `8400e1e2` core family of 528 frames (12.8% of len=39 bucket) — fully stable across all 32 decoded bytes except a small tail, always carries `5e4e16` marker. Most stable family in the bucket. Adding `FRzMovementCoreReply8400` lifts coverage 73.08% → ~74.2% (+1.1pp).

## Hypothesis
Adding `FRzMovementCoreReply8400 { tail: Vec<u8> }` with classifier rule `direction == S2C && wire_len == 39 && decoded.len() == 32 && decoded[0..4] == [0x84, 0x00, 0xe1, 0xe2]` types ≥520 of 528 expected frames and lifts typed coverage 73.08% → ≥74.0%. All v6 typed rows must remain unchanged.

## Falsification (3 outcomes)
- (a) **Variant added + cargo build OK + typed coverage ≥74.0% + ≥520 8400 frames + v6 regression check passes** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v7_<coverage>_8400_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v7_break`.
- (c) **Coverage moves but <74.0% OR fewer than 480 frames classified** → name what's defensible. Fact: `dark_december_decoder_rust_typed_v7_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v6-9400 baseline** from `/home/sdancer/dark-december-typed-v6-9400/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Add `FRzMovementCoreReply8400` variant** in `src/lib.rs`:
   ```rust
   FRzMovementCoreReply8400 {
       tail: Vec<u8>,  // dec[4..32] — fully stable body with 5e4e16 marker
   },
   ```
   - Classifier rule (after FRzEntityNotify9400, before generic fallback): `direction == Direction::S2C && raw.len() == 39 && decoded.len() == 32 && decoded[0] == 0x84 && decoded[1] == 0x00 && decoded[2] == 0xe1 && decoded[3] == 0xe2`
   - Decode: `tail = decoded[4..32].to_vec()`
   - Annotate `// MEDIUM (cycle-449 core fit, 528/4110 S2C len=39 frames; fully stable body with 5e4e16 marker — placeholder name pending Rz reflection mapping)`
   - Match-arms for `id()`/`x()`/`z()`/`rot()` return `None`.

3. **Validate**: cargo build --release; run on `first_quest_{c2s,s2c}.tcpstream.bin`; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - FRzMovementCoreReply8400 frame count (must be ≥520).
   - Confirmation that v6-typed-variant frame counts are UNCHANGED.

4. **Diff vs v6-typed CSV** (regression check): `out_v6_baseline/typed_packets.csv` vs `out_v7_orch/typed_packets.csv` — every row present in v6 must be present in v7 with identical fields; v7 only ADDS rows.

5. **Write `analysis/typed_v7_8400_2026-05-16.md`** (≤80 lines):
   - Coverage table (v6 73.08% → v7 X.XX%).
   - FRzMovementCoreReply8400 frame count + variability summary.
   - Regression confirmation.
   - FRAGILE note: placeholder name; actual UE typename TBD via reflection metadata.

6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V7_8400_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤80 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **IMPORTANT**: After writing the artifact, fact-set, and final TYPED_V7_8400_DONE marker, EXIT IMMEDIATELY. Do NOT continue running additional commands — cycle-464 worker stalled in extra exploration after the artifact was already written. Single-turn means single-turn.
- Do NOT change v6 variant classifications. Single-variant scope.

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v6-9400/`
- Cycle-449 S2C len=39 analysis: `/home/sdancer/dark-december-s2c-len39/analysis/s2c_len39_2026-05-15.md`
- Cycle-464 v6-9400 artifact: `/home/sdancer/dark-december-typed-v6-9400/analysis/typed_v6_9400_2026-05-16.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- success-fact: `dark_december_decoder_rust_typed_v7_<coverage>_8400_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v7_break` (b), `dark_december_decoder_rust_typed_v7_partial` (c)
