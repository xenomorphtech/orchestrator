## typed-v4-ack — Extend typed Rust decoder with FRzAckResult variant (S2C len=12, 588 frames)

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v4-ack` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v4-ack`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v4_ack` (new)
- **sub_goal_key**: `add-frzack-result-variant-for-s2c-len12`

## Why this turn exists
Cycle-458 typed-v3-reconcile shipped at 64.63% typed coverage with 7 Packet variants. Cycle-452 closed S2C len=12 (588 frames, ~1.7% of S2C) as `subtype_token u8 + result_code s32_le` (mostly 0x12 + value 3). This bucket is clean enough to add as an 8th typed variant `FRzAckResult` and lift coverage to ~66.3%. No new RE — analysis already done, this is variant integration.

## Hypothesis
Adding a `FRzAckResult { subtype: u8, result_code: i32 }` variant to the typed Packet enum, with classifier rule `direction=S2C AND wire_len=12 AND decoded_len=5`, types all 588 cycle-452 frames identically to the cycle-452 analysis and lifts typed coverage from 64.63% to ≥66.0%.

## Falsification (3 outcomes)
- (a) **Variant added + cargo build OK + typed coverage ≥66.0% + previously-typed-by-v3 frames produce IDENTICAL classification** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v4_<coverage>_ack_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v4_break`.
- (c) **Coverage moves but <66.0% OR fewer than 580 ack frames classified** → name what's defensible. Fact: `dark_december_decoder_rust_typed_v4_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v3-reconcile baseline** from `/home/sdancer/dark-december-typed-v3-reconcile/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Add `FRzAckResult` variant** in `src/lib.rs`:
   ```rust
   FRzAckResult {
       subtype: u8,     // dec[0] — dominant 0x12, with 0x10/0x11/0x1b siblings
       result_code: i32, // dec[1..5] as LE i32 — dominant 3 (success), rare -4/-590
   },
   ```
   - Classification rule: `direction == Direction::S2C && wire_len == 12 && decoded.len() == 5`
   - Decode: `subtype = decoded[0]; result_code = i32::from_le_bytes(decoded[1..5].try_into().unwrap())`.
   - Add `// MEDIUM (cycle-452 single-class fit, 588/35260 S2C frames; result_code dominant value 3)` comment.

3. **Validate**: cargo build --release; run on `first_quest_{c2s,s2c}.tcpstream.bin`; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - Per-variant counts (must include FRzAckResult with ≥580 frames).
   - Confirmation that v3-typed-variant frame counts are UNCHANGED (regression check).

4. **Diff vs v3-typed CSV** (regression check): compare typed packets CSV before/after — every row present in v3-typed must be present in v4-typed with identical fields; v4 only ADDS rows.

5. **Write `analysis/typed_v4_ack_2026-05-16.md`** (≤100 lines):
   - Coverage table (v3 64.63% → v4 X.XX%).
   - FRzAckResult frame count + sample (top 5 frames by occurrence).
   - Regression confirmation (v3 variants frame counts unchanged).
   - Note on FRAGILE: result_code interpretation is empirical; if Rz reflection metadata later names the field differently, this is a documentation update not a behavior change.

6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V4_ACK_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤100 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Do NOT change v3 variant classifications. Do NOT add other variants in this turn (single-variant scope).

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v3-reconcile/`
- Cycle-452 S2C len=12 analysis: `/home/sdancer/dark-december-s2c-len12/analysis/s2c_len12_2026-05-15.md`
- Cycle-458 v3 reconcile artifact: `/home/sdancer/dark-december-typed-v3-reconcile/analysis/rust_typed_decoder_v3_2026-05-15.md`
- Cycle-444 protocol wiki: `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- success-fact: `dark_december_decoder_rust_typed_v4_<coverage>_ack_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v4_break` (b), `dark_december_decoder_rust_typed_v4_partial` (c)
