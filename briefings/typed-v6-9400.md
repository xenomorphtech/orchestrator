## typed-v6-9400 — Add FRzEntityNotify9400 variant for S2C len=39 `9400<id>86` minor family (730 frames)

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v6-9400` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v6-9400`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v6_9400` (new)
- **sub_goal_key**: `add-frzentitynotify9400-variant-for-s2c-len39-minor-family`

## Why this turn exists
Cycle 463 typed-v5-9700 landed at 71.56% coverage (+5.71pp from v4) — bullseye on predicted Δ. Same s2c-len39 analysis (cycle 449) named a `9400<id>86` minor family of 730 frames (17.8% of len=39 bucket). Structurally identical pattern to 9700: fixed opcode byte at dec[0]=0x94, dec[1]=0x00, actor/subtype at dec[2], class byte 0x86 at dec[3]. Adding as a separate variant `FRzEntityNotify9400` lifts coverage 71.56% → ~73.1% (+1.52pp).

## Hypothesis
Adding `FRzEntityNotify9400 { entity_id: u8, tail: Vec<u8> }` with classifier rule `direction == S2C && wire_len == 39 && decoded.len() == 32 && decoded[0] == 0x94 && decoded[1] == 0x00 && decoded[3] == 0x86`, types ≥700 of 730 expected frames and lifts typed coverage 71.56% → ≥72.8%. All v5 typed rows must remain unchanged.

## Falsification (3 outcomes)
- (a) **Variant added + cargo build OK + typed coverage ≥72.8% + ≥700 9400 frames + v5 regression check passes** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v6_<coverage>_9400_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v6_break`.
- (c) **Coverage moves but <72.8% OR fewer than 600 9400 frames classified** → name what's defensible. Fact: `dark_december_decoder_rust_typed_v6_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v5-9700 baseline** from `/home/sdancer/dark-december-typed-v5-9700/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Add `FRzEntityNotify9400` variant** in `src/lib.rs`:
   ```rust
   FRzEntityNotify9400 {
       entity_id: u8,    // dec[2]
       tail: Vec<u8>,    // dec[4..32]
   },
   ```
   - Classifier rule (placed AFTER FRzAckResult, near FRzEntityNotify9700): `direction == Direction::S2C && raw.len() == 39 && decoded.len() == 32 && decoded[0] == 0x94 && decoded[1] == 0x00 && decoded[3] == 0x86`
   - Decode: `entity_id = decoded[2]; tail = decoded[4..32].to_vec()`
   - Annotate `// MEDIUM (cycle-449 minor family fit, 730/4110 S2C len=39 frames; same structure as FRzEntityNotify9700 with opcode 0x94 instead of 0x97)`
   - Match-arms for `id()`/`x()`/`z()`/`rot()` return `None`.

3. **Validate**: cargo build --release; run on `first_quest_{c2s,s2c}.tcpstream.bin`; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - FRzEntityNotify9400 frame count (must be ≥700).
   - Confirmation that v5-typed-variant frame counts are UNCHANGED.

4. **Diff vs v5-typed CSV** (regression check): same pattern as v5 — `out_v5_baseline/typed_packets.csv` vs `out_v6_orch/typed_packets.csv` — every row present in v5 must be present in v6 with identical fields; v6 only ADDS rows.

5. **Write `analysis/typed_v6_9400_2026-05-16.md`** (≤100 lines):
   - Coverage table (v5 71.56% → v6 X.XX%).
   - FRzEntityNotify9400 frame count + entity_id histogram (top-10).
   - Regression confirmation.
   - FRAGILE note: same family-naming caveat as v5 (exact-prefix bucket name, actual UE typename TBD).

6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V6_9400_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤100 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Do NOT change v5 variant classifications. Single-variant scope.
- Verify if-chain ordering preserves FRzAckResult-before-Notify9700-before-Notify9400 precedence (no overlap on wire_len anyway, but defensive ordering is good).

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v5-9700/`
- Cycle-449 S2C len=39 analysis: `/home/sdancer/dark-december-s2c-len39/analysis/s2c_len39_2026-05-15.md`
- Cycle-463 v5-9700 artifact: `/home/sdancer/dark-december-typed-v5-9700/analysis/typed_v5_9700_2026-05-16.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- success-fact: `dark_december_decoder_rust_typed_v6_<coverage>_9400_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v6_break` (b), `dark_december_decoder_rust_typed_v6_partial` (c)
