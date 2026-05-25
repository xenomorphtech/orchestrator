## rust-decoder-typed-v2 — Add FRzTriggerActiveBr + FRzCoolTimeResetNoti to the typed Rust decoder

## Role & workdir
Rust port worker. Workdir: `/home/sdancer/dark-december-rust-decoder-typed-v2` (worktree of `/home/sdancer/darkdecember/`, branch `rust-decoder-typed-v2`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v2` (new)
- **sub_goal_key**: `incorporate-cycle-419-promotions-into-typed-decoder`

## Why this turn exists
Cycle 417 shipped the typed Rust decoder with 5 high-confidence bindings at 63.90% coverage (`darkdecember/rust-decoder-typed`). Cycle 419 promoted 2 additional medium-confidence S2C bindings to high via temporal lift: `FRzTriggerActiveBr` (S2C len=27 / 195 frames) and `FRzCoolTimeResetNoti` (S2C len=32 / 159 frames). Adding both to the typed decoder is a strict incremental win on the existing pipeline.

## Hypothesis
Extending the typed decoder's classifier in `src/lib.rs` with two new variants (`Packet::FRzTriggerActiveBr` and `Packet::FRzCoolTimeResetNoti`) using S2C-direction-and-length signatures bumps coverage from 30,662 / 47,987 = 63.90% to approximately 31,016 / 47,987 = 64.63% (~+354 frames typed), with the 3 existing baseline CSVs remaining byte-exact when `--emit-typed` is absent.

## Falsification (3 outcomes)
- (a) **Both new bindings + 5 existing bindings type at expected counts (195 ± 5% for TriggerActiveBr, 159 ± 5% for CoolTimeResetNoti), AND diff -r out_rust darkdec_output_streams is empty without --emit-typed** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v2_<percent>_coverage`.
- (b) **TriggerActiveBr or CoolTimeResetNoti classifier matches the wrong S2C length-class signature, miscounts, or breaks the 3 baseline CSVs** → typed-classifier regression. Fact: `dark_december_decoder_rust_typed_v2_regression`.
- (c) **Both new typings work cleanly but coverage is below the predicted threshold** → partial outcome, document why. Fact: `dark_december_decoder_rust_typed_v2_partial_<percent>`.

## Success criteria — SINGLE TURN, do all before stopping

**Primary**: extend `/home/sdancer/dark-december-rust-decoder-typed-v2/` with:

1. **Copy baseline from cycle-417**:
```bash
cp -r /home/sdancer/dark-december-rust-decoder-typed/{Cargo.toml,Cargo.lock,src} /home/sdancer/dark-december-rust-decoder-typed-v2/
```
Verify `cargo build --release` succeeds. Run with the streams, diff out_rust against `/home/sdancer/orchestrator/darkdec_output_streams/` — must be empty.

2. **Add two new variants** to the Packet enum in `src/lib.rs`:
```rust
pub enum Packet {
    // ... existing 5 variants ...
    FRzTriggerActiveBr { entity_id: u8, extra: Vec<u8> },
    FRzCoolTimeResetNoti { ... },
    Unknown { ... },
}
```
Classify:
- `FRzTriggerActiveBr`: S2C, raw.len() == 27, dec.starts_with([0x1b, 0x03, 0x01]) (the cycle-408 dominant prefix at 98.5%).
- `FRzCoolTimeResetNoti`: S2C, raw.len() == 32, dec.starts_with([0x9b, 0x00, 0x01, 0x60]) (cycle-408 prefix at 93.1%).

3. **Extend typed CSV writer** in `src/main.rs` to emit the new variants in `typed_packets.csv`. Field selection: for both Br/Noti, just write the typename + first few decoded bytes as extra_json (no kinematic fields known yet for these).

4. **Build + run**:
```bash
cargo build --release
./target/release/darkdec_decoder_cli --c2s ... --s2c ... --out out_rust --emit-typed
diff -r out_rust /home/sdancer/orchestrator/darkdec_output_streams  # MUST stay empty without --emit-typed
```

5. **Coverage report**: count typed frames by variant. Expected:
- FRzMoveBr: 23,568
- FRzMoveRq: 6,022
- FRzSkillMoveSyncRq: 952
- FRzMoveDuringSkillRq: 104
- FRzControlProjectileRq: 16
- **FRzTriggerActiveBr: ~195 (NEW)**
- **FRzCoolTimeResetNoti: ~159 (NEW)**
- Unknown: ~16,971
- Total typed: ~31,016 / 47,987 = ~64.63%

6. **Write artifact** `analysis/rust_typed_decoder_v2_2026-05-15.md` with the implementation diff overview + per-variant counts + coverage stat + closing fact.

7. **Set closing fact** via `harness fact-set dark_december_decoder_rust_typed_v2_<percent>_coverage true`.

Print `RUST_DECODER_TYPED_V2_DONE` on the final line.

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Rust + cargo fine.
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤200 lines (code unbounded).
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED** — do all 7 steps before stopping.
- **DO NOT modify** the cycle-417 worktree at `/home/sdancer/dark-december-rust-decoder-typed/`. Copy from it, don't edit it.
- **Baseline preservation is non-negotiable**: without `--emit-typed`, the 3 existing CSVs must be byte-exact.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-rust-decoder-typed-v2/` (branch `rust-decoder-typed-v2`).
- Source crate (COPY FROM): `/home/sdancer/dark-december-rust-decoder-typed/` (branch `rust-decoder-typed`, cycle-417 baseline).
- New binding evidence (cycle 419): `/home/sdancer/dark-december-s2c-disambig/analysis/s2c_bindings_v2_2026-05-15.md`
- S2C bucket reference (cycle 408): `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
- RZ binding (cycle 412): `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- Baseline (preservation target): `/home/sdancer/orchestrator/darkdec_output_streams/`
- success-fact key: `dark_december_decoder_rust_typed_v2_<percent>_coverage` (a)
- block-fact keys: `dark_december_decoder_rust_typed_v2_regression` (b), `dark_december_decoder_rust_typed_v2_partial_<percent>` (c)
