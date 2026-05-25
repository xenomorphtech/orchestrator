## typed-v5-9700 — Add FRzEntityNotify9700 variant for S2C len=39 `9700<id>86` family (2,738 frames)

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v5-9700` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v5-9700`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v5_9700` (new)
- **sub_goal_key**: `add-frzentitynotify9700-variant-for-s2c-len39-actor-family`

## Why this turn exists
Cycle 462 typed-v4-ack landed at 65.85% coverage (+1.22pp from v3). Cycle 449 s2c-len39 closure named the `9700<id>86` family (2,738 frames, 66.6% of len=39 bucket) with fixed `010000461103` header after byte 4. This is the single largest remaining unclassified frame class — adding it as variant `FRzEntityNotify9700` lifts coverage 65.85% → ~71.5% (+5.7pp, +2,738 frames). Lowest-cost single-variant extension currently available.

## Hypothesis
Adding `FRzEntityNotify9700 { entity_id: u8, tail: Vec<u8> }` to the typed Packet enum, with classifier rule `direction == S2C && wire_len == 39 && decoded.len() == 32 && decoded[0] == 0x97 && decoded[1] == 0x00 && decoded[3] == 0x86`, types ≥2,700 of 2,738 expected frames and lifts typed coverage 65.85% → ≥70.0%. All v4 typed rows must remain unchanged.

## Falsification (3 outcomes)
- (a) **Variant added + cargo build OK + typed coverage ≥70.0% + ≥2,700 FRzEntityNotify9700 frames + previously-typed-by-v4 frames produce IDENTICAL classification** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v5_<coverage>_9700_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v5_break`.
- (c) **Coverage moves but <70.0% OR fewer than 2,500 9700 frames classified** → name what's defensible. Fact: `dark_december_decoder_rust_typed_v5_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v4-ack baseline** from `/home/sdancer/dark-december-typed-v4-ack/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Add `FRzEntityNotify9700` variant** in `src/lib.rs`:
   ```rust
   FRzEntityNotify9700 {
       entity_id: u8,       // dec[2] — actor/subtype byte (0xac..0xe7 observed)
       tail: Vec<u8>,        // dec[4..32] — opaque body (28 bytes including `010000461103` header @ dec[4..10])
   },
   ```
   - Classifier rule (in the decode/classify function): `direction == Direction::S2C && raw.len() == 39 && decoded.len() == 32 && decoded[0] == 0x97 && decoded[1] == 0x00 && decoded[3] == 0x86`
   - Decode: `entity_id = decoded[2]; tail = decoded[4..32].to_vec()`
   - Annotate `// MEDIUM (cycle-449 family-fit, 2738/4110 S2C len=39 frames; fixed 010000461103 header after byte 4; actor_id varies 0xac..0xe7)`
   - Match-arms for `id()`/`x()`/`z()`/`rot()` etc. return `None` (this is a notification, not a movement packet).

3. **Validate**: cargo build --release; run on `first_quest_{c2s,s2c}.tcpstream.bin`; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - FRzEntityNotify9700 frame count (must be ≥2,500).
   - Confirmation that v4-typed-variant frame counts are UNCHANGED (including FRzAckResult at 588).

4. **Diff vs v4-typed CSV** (regression check): compare `out_v4_baseline/typed_packets.csv` vs `out_v5_orch/typed_packets.csv` — every row present in v4 must be present in v5 with identical fields; v5 only ADDS rows.

5. **Write `analysis/typed_v5_9700_2026-05-16.md`** (≤100 lines):
   - Coverage table (v4 65.85% → v5 X.XX%).
   - FRzEntityNotify9700 frame count + entity_id histogram (top-10 entity_id bytes).
   - Regression confirmation (v4 variants + FRzAckResult unchanged).
   - FRAGILE note: family was named by exact-prefix bucketing; actual UE typename TBD via reflection metadata.

6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V5_9700_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤100 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Do NOT change v4 variant classifications. Do NOT add other variants (single-variant scope per orchestrator pattern).
- Confirm the classifier RULE BEFORE classifying that a frame is FRzAckResult NOT 9700. They have disjoint wire_len (12 vs 39) so no overlap — but verify the if-chain ordering doesn't cause a regression.

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v4-ack/`
- Cycle-449 S2C len=39 analysis: `/home/sdancer/dark-december-s2c-len39/analysis/s2c_len39_2026-05-15.md`
- Cycle-462 v4-ack artifact: `/home/sdancer/dark-december-typed-v4-ack/analysis/typed_v4_ack_2026-05-16.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- success-fact: `dark_december_decoder_rust_typed_v5_<coverage>_9700_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v5_break` (b), `dark_december_decoder_rust_typed_v5_partial` (c)
