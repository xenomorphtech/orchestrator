## rust-decoder-typed — Upgrade darkdec_decoder to type-aware output using the rz binding

## Role & workdir
Rust port worker. Workdir: `/home/sdancer/dark-december-rust-decoder-typed` (worktree of `/home/sdancer/darkdecember/`, branch `rust-decoder-typed`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed` (new)
- **sub_goal_key**: `embed-rz-binding-into-rust-decoder-for-typed-output`

## Why this turn exists
Cycle 403 produced a bit-exact Rust port of the Python decoder (`/home/sdancer/dark-december-rust-decoder`) — outputs raw `Frame { dir, ord, off, raw, dec }`. Cycle 412 bound 5 length classes to specific Rz typenames. Combining them: the Rust decoder should emit **typed packet records** (`FRzMoveRq`, `FRzMoveBr`, `FRzSkillMoveSyncRq`, `FRzMoveDuringSkillRq`, `FRzControlProjectileRq` for the high-confidence bindings) instead of raw bytes. This is the natural upgrade path: takes the validated decoder and makes it production-grade for downstream tooling (live minimap-sniffer at `darkdecember/main`, packet replay, fuzzing harness).

## Hypothesis
Adding a typed `Packet` enum + parsers for the 5 high-confidence-bound classes (`FRzMoveRq`, `FRzMoveBr`, `FRzMoveDuringSkillRq`, `FRzSkillMoveSyncRq`, `FRzControlProjectileRq`) to `/home/sdancer/dark-december-rust-decoder/src/lib.rs` is a strictly additive change that does not affect the existing bit-exact CSV output. Each typed parser reads from the decoded buffer using the wire layout from cycle-403's offset analysis and cycle-412's Serialize size match. A new CLI flag `--emit-typed` produces a third output CSV `analysis/typed_packets.csv` with at least 50% of the 47,987 frames typed.

## Falsification (3 outcomes)
- (a) **≥50% of frames typed via the 5 bound classes + the original 3 CSVs remain byte-exact against the Python baseline** → SUCCESS. Fact: `dark_december_decoder_rust_typed_<percent>_coverage`.
- (b) **Typed parsing breaks existing CSV byte-exactness** (e.g., float-string formatting regresses) → FALSIFIED on regression; existing baseline preserved by gating typed code behind a CLI flag. Fact: `dark_december_decoder_rust_typed_regression`.
- (c) **Some classes have field-layout mismatches** between our cycle-403 inferred offsets and the cycle-412 +8 wire model → partial coverage; document mismatches. Fact: `dark_december_decoder_rust_typed_partial_<n>_classes`.

## Success criteria
**Primary**: extend `/home/sdancer/dark-december-rust-decoder-typed/` (copy or branch from cycle-403's `dark-december-rust-decoder/`) with:

1. **Copy crate baseline** from `/home/sdancer/dark-december-rust-decoder/` (Cargo.toml, src/lib.rs, src/main.rs). Verify `cargo build --release` succeeds in the new worktree.
2. **Add typed Packet model** to `src/lib.rs`:
```rust
pub enum Packet {
    FRzMoveRq { entity_id: u64, x: f32, z: f32, rot: f32, ... },
    FRzMoveBr { entity_id: u8, x: f32, z: f32, rot: f32, ... },
    FRzSkillMoveSyncRq { ... },
    FRzMoveDuringSkillRq { ... },
    FRzControlProjectileRq { ... },
    Unknown { dir: Direction, raw_len: usize, dec_prefix: [u8; 4] },
}
pub fn classify(frame: &Frame) -> Packet { ... }
```
3. **Field layouts** derived from cycle-403 inferred offsets (player_track x@9, z@17, rot@25 in the decoded view) AND from the c2s-len45 field map at `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`. Cross-reference cycle-407's len=61 shifted-insertion model (coord@33, heading@41, subtype@49, flag@53). For `FRzSkillMoveSyncRq` at len=61 (53B Serialize), expect signature similar to FRzMoveRq with extra fields.
4. **CLI flag** `--emit-typed` on the existing binary that writes `<out>/typed_packets.csv` with columns: `ord, dir, packet_type, entity_id, x, z, rot, extra_json`.
5. **Baseline preservation**: without `--emit-typed`, the 3 existing CSVs remain byte-exact. Verify with `diff -r out_rust /home/sdancer/orchestrator/darkdec_output_streams/`.
6. **Coverage report**: run on the first_quest streams and count typed-vs-unknown frames. Report percentage typed.
7. **Write `analysis/rust_typed_decoder_2026-05-15.md`** with the implementation overview + coverage stat + closing fact via `harness fact-set`.

Print `RUST_DECODER_TYPED_DONE` on the final line.

## Execution flow

**Step 1 — Copy + verify baseline:**
```bash
cp -r /home/sdancer/dark-december-rust-decoder/{Cargo.toml,Cargo.lock,src} /home/sdancer/dark-december-rust-decoder-typed/
cd /home/sdancer/dark-december-rust-decoder-typed
cargo build --release
./target/release/darkdec_decoder_cli --c2s /home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin \
  --s2c /home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin --out out_rust
diff -r out_rust /home/sdancer/orchestrator/darkdec_output_streams
# Must be empty diff.
```

**Step 2 — Add typed enum + classifier** to `src/lib.rs`. Classify by `(direction, raw.len(), dec[0..4])`.

**Step 3 — Per-class parser** based on cycle-403 + cycle-407 + cycle-412 evidence. Be conservative: only emit a typed record when the wire shape matches the canonical signature; unknown frames go to `Packet::Unknown`.

**Step 4 — Add CLI flag** `--emit-typed`. Implement `write_typed_csv()`.

**Step 5 — Run + report coverage**.

**Step 6 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Rust is efficient; cargo + LSP fine.
- **NO new disasm. NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤300 lines (code itself is unbounded).
- **One Codex turn budget: ≤25 min wall time.** Single-turn completion.
- **DO NOT modify** the existing `/home/sdancer/dark-december-rust-decoder/` worktree — that branch is shipped at `darkdecember/rust-decoder-port`.
- **Baseline preservation is non-negotiable**: the 3 existing CSVs must remain byte-exact without `--emit-typed`. If you must change shared code paths, verify the diff stays empty.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-rust-decoder-typed/` (branch `rust-decoder-typed`).
- Source crate (COPY FROM): `/home/sdancer/dark-december-rust-decoder/` (branch `rust-decoder-port`, shipped).
- RZ binding artifact (PRIMARY REFERENCE for typenames): `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`
- C2S len=45 field map (PRIMARY REFERENCE for FRzMoveRq fields): `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
- C2S len=61 shifted-insertion model (for FRzSkillMoveSyncRq fields): `/home/sdancer/dark-december-c2s-len61-decode/analysis/c2s_len61_decode_2026-05-15.md`
- S2C inventory (FRzMoveBr smoke test reference): `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- Baseline (validation target): `/home/sdancer/orchestrator/darkdec_output_streams/`
- success-fact key: `dark_december_decoder_rust_typed_<percent>_coverage` (a)
- block-fact keys: `dark_december_decoder_rust_typed_regression` (b), `dark_december_decoder_rust_typed_partial_<n>_classes` (c)
