## minimap-sniffer-typed — Wire the typed Rz Packet decoder into the live minimap-sniffer

## Role & workdir
Rust integration worker. Workdir: `/home/sdancer/dark-december-minimap-sniffer-typed` (worktree of `/home/sdancer/darkdecember/`, branch `minimap-sniffer-typed`).

## Current goal / sub-goal
- **goal_key**: `dark_december_minimap_sniffer_typed` (new)
- **sub_goal_key**: `integrate-rust-typed-decoder-into-egui-minimap`

## Why this turn exists
The egui minimap-sniffer at `minimap-sniffer/` (922 lines of Rust, with `darkdec.rs` + `state.rs` + `tcp_reassembly.rs`) currently decodes Dark December packets with the old Python-ported logic and a per-frame adjacent-XOR view. It produces a working minimap from live pcap on Waydroid loopback, but it does NOT know about specific Rz typenames (FRzMoveBr, FRzMoveRq, etc.) — it treats every frame as raw bytes and infers entity positions heuristically.

Cycle 422 shipped the typed Rust decoder (`darkdecember/rust-decoder-typed-v2`) with 7 high-confidence Rz typename bindings and 64.63% coverage on the frozen first_quest corpus. Wiring this into the minimap-sniffer is the operational deliverable that pulls the entire RE campaign together — live in-game gameplay surfaces real-time as typed FRzMoveBr entity-update events with named field positions from cycle 415.

## Hypothesis
Replacing the heuristic packet decoder in `minimap-sniffer/src/darkdec.rs` + `state.rs` with the typed Packet enum from `rust-decoder-typed-v2/src/lib.rs` produces:
- Identical minimap behaviour on the first_quest pcap replay (entity dots in the same positions).
- New typed event stream emitted from `update_from_packet()` that subscribers (egui UI, web UI) can render with packet-type-aware semantics (e.g. green dot for FRzMoveBr, red for FRzControlProjectileRq).

## Falsification (3 outcomes)
- (a) **Builds + runs on first_quest stream replay; entity-track CSV diff against the frozen baseline at `darkdec_output_streams/entity_tracks.csv` is empty or differs only by float-string formatting** → SUCCESS. Fact: `dark_december_minimap_sniffer_typed_<percent>_coverage`.
- (b) **Builds but minimap entity positions diverge from the baseline** → integration breaks the existing decoder's behavior. Fact: `dark_december_minimap_sniffer_typed_regression`.
- (c) **Build fails** (Cargo dependency or API mismatch) → integration not feasible in one turn; report blockers. Fact: `dark_december_minimap_sniffer_typed_build_blocked`.

## Success criteria — SINGLE TURN, do all before stopping

**Primary**: extend `/home/sdancer/dark-december-minimap-sniffer-typed/minimap-sniffer/` with:

1. **Read the existing minimap-sniffer Rust files** (922 lines total):
   - `src/lib.rs` (5 lines, re-exports)
   - `src/main.rs` (261 lines, egui app entry)
   - `src/darkdec.rs` (76 lines, current decoder)
   - `src/packet.rs` (103 lines, packet representation)
   - `src/state.rs` (176 lines, app state — where entity positions live)
   - `src/tcp_reassembly.rs` (155 lines, pcap → byte stream)
   - `src/web.rs` (146 lines, web UI)
   - `Cargo.toml` (features: live-pcap, egui)

2. **Add a `darkdec_typed` module** to the minimap-sniffer crate (NOT a new dependency on the rust-decoder-typed-v2 crate — copy or inline the relevant types). Inline the typed Packet enum + classifier from `/home/sdancer/dark-december-rust-decoder-typed-v2/src/lib.rs`. Cite the source crate in a top-of-file comment.

3. **Wire `darkdec_typed::classify(frame)` into `state.rs`**:
   - Where `update_from_packet()` (or equivalent) currently parses raw bytes for entity positions, replace with a `match` on the typed `Packet` enum.
   - For `Packet::FRzMoveBr { actor_handle, x, z, rot, .. }`: emit an entity update (existing UI handler).
   - For `Packet::FRzMoveRq { actor_handle, x, z, rot, .. }`: emit a player update.
   - For `Packet::FRzControlProjectileRq`, `FRzSkillMoveSyncRq`, `FRzMoveDuringSkillRq`: emit a typed event (UI can choose to render).
   - For `Packet::Unknown`: fall back to the existing heuristic decoder OR drop silently (your call — document choice).

4. **Replay sanity check**: add or extend a CLI subcommand `darkdec-egui-minimap --replay <pcap-or-tcpstream>` that reads the frozen first_quest stream and prints entity positions to stdout. Diff against `/home/sdancer/orchestrator/darkdec_output_streams/entity_tracks.csv` (or the cycle-422 `typed_packets.csv`). If exact match, perfect; if float-string-only diff, document.

5. **Build + run**:
```bash
cd /home/sdancer/dark-december-minimap-sniffer-typed/minimap-sniffer
cargo build --release
# Run replay sanity check
./target/release/darkdec-egui-minimap --replay /home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin > replay_out.csv
diff replay_out.csv /home/sdancer/orchestrator/darkdec_output_streams/entity_tracks.csv  # acceptable: empty or float-string-only
```

6. **Write artifact** `analysis/minimap_sniffer_typed_2026-05-15.md` summarizing:
   - Files modified + lines added.
   - Build success.
   - Replay diff result.
   - Per-packet-type counts from the replay.
   - Any TODO / known gaps.

7. **Set closing fact** via `harness fact-set dark_december_minimap_sniffer_typed_<n>_typed true`. Print `MINIMAP_SNIFFER_TYPED_DONE` on the final line.

## Execution flow

**Step 1** — Read all minimap-sniffer source files in full.
**Step 2** — Read the cycle-422 typed Packet enum + classifier from `/home/sdancer/dark-december-rust-decoder-typed-v2/src/lib.rs`.
**Step 3** — Inline as a new module `darkdec_typed.rs` (avoid cross-crate deps for simplicity).
**Step 4** — Wire into `state.rs` event handlers.
**Step 5** — Add replay CLI flag.
**Step 6** — Build + run + diff.
**Step 7** — Write artifact + fact-set + DONE.

## Constraints & gotchas
- **HARD memory budget: 1 GB.** Rust + cargo + egui can be heavier than pure-Python.
- **NO new disasm. NO Frida. NO live device.** This is a Rust integration task only; replay against the frozen pcap is the validation.
- **HARD output cap**: artifact ≤300 lines (code itself unbounded).
- **ONE Codex turn budget: ≤25 min wall time. SINGLE-TURN COMPLETION REQUIRED** — do all 7 steps before stopping; do not pause at task boundaries.
- **Egui crate compatibility**: the minimap-sniffer pins `eframe = 0.34.2`. Don't bump versions; just integrate the new decoder module.
- **DO NOT modify** the rust-decoder-typed-v2 worktree at `/home/sdancer/dark-december-rust-decoder-typed-v2/`. Copy the typed enum + classifier source via inline definition.
- The `pcap` feature is optional; the replay sanity check should work without live pcap (use `tcp_reassembly.rs` or read the pre-reassembled stream directly).

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-minimap-sniffer-typed/` (branch `minimap-sniffer-typed`).
- Source minimap-sniffer: `/home/sdancer/dark-december-minimap-sniffer-typed/minimap-sniffer/`
- Typed decoder source (INLINE FROM): `/home/sdancer/dark-december-rust-decoder-typed-v2/src/lib.rs`
- FRzMoveBr field layout (cycle 415): `/home/sdancer/dark-december-frzmove-br-decode/analysis/frzmove_br_decode_2026-05-15.md`
- Rz binding (cycle 412): `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- Baseline (replay validation target): `/home/sdancer/orchestrator/darkdec_output_streams/entity_tracks.csv`
- success-fact key: `dark_december_minimap_sniffer_typed_<n>_typed` (a)
- block-fact keys: `dark_december_minimap_sniffer_typed_regression` (b), `dark_december_minimap_sniffer_typed_build_blocked` (c)
