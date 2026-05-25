# rust-decoder-port — Port darkdec_decoder.py to Rust with bit-exact parity

## Role & workdir
Pure-Rust port worker. Workdir: `/home/sdancer/dark-december-rust-decoder` (worktree of `/home/sdancer/darkdecember/`, the darkdecember repo).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_port` (new — planner-proposed Goal 2)
- **sub_goal_key**: `port-python-decoder-to-rust-with-bitexact-parity`

## Why this turn exists
The Python decoder `darkdec_decoder.py` (cycle 383, validated against 47,987 frames) is the canonical wire-format parser, but Python is slow and not embeddable into the existing native `egui` minimap viewer the user pushed at `darkdecember/main` (a07d248). A Rust port unlocks:
- Embedding the decoder into the native minimap viewer for real-time use
- Faster pcap → minimap pipeline (Python is the bottleneck on the 47K-frame test corpus)
- Reusable library for any downstream tool (live capture eBPF userspace, replay server, packet test harness)

## Hypothesis
A line-by-line Rust port of `darkdec_decoder.py`'s core functions (`adjacent_xor`, `app_frames`, `extract_tracks`) running over `streams/first_quest/{c2s,s2c}.tcpstream.bin` produces **bit-exact parity** with the Python baseline at `darkdec_output_streams/player_track.csv`, `entity_tracks.csv`, `entities_summary.csv`.

## Falsification (3 outcomes)
- (a) **All 6013 player updates + all 17555 entity-movement updates match the Python baseline byte-for-byte (CSV diff = 0)** → SUCCESS. Fact: `dark_december_rust_decoder_bitexact_parity_<n_frames>`.
- (b) **Frame parsing matches but coord values diverge by >0.001 from baseline** → arithmetic precision issue, likely from f32 vs f64 handling or endianness assumption. Fact: `dark_december_rust_decoder_precision_drift`.
- (c) **Frame count itself diverges** → reassembly or framing logic differs. Fact: `dark_december_rust_decoder_frame_count_mismatch`.

## Success criteria
**Primary**: produce a Rust crate at `/home/sdancer/dark-december-rust-decoder/` with:
1. `Cargo.toml` with `[lib]` for `darkdec_decoder` and `[[bin]]` for a CLI matching the Python interface (`darkdec_decoder_cli --c2s c2s.bin --s2c s2c.bin --out out_dir`).
2. `src/lib.rs` containing:
   - `pub fn adjacent_xor(body: &[u8]) -> Vec<u8>` — implements `decoded[i] = body[i] ^ body[i+1]`
   - `pub struct Frame { pub dir: Direction, pub ord: usize, pub off: usize, pub raw: Vec<u8>, pub dec: Vec<u8> }`
   - `pub fn parse_stream(buf: &[u8], dir: Direction) -> Vec<Frame>` — frame split by 4B LE length prefix
   - `pub fn extract_player_tracks(frames: &[Frame]) -> Vec<PlayerUpdate>` — finds S2C 41-byte frames with decoded prefix `12 02 60 6d`
   - `pub fn extract_entity_tracks(frames: &[Frame]) -> HashMap<u8, Vec<EntityUpdate>>`
3. `src/main.rs` calling above and writing CSVs in the same format as the Python decoder.
4. `tests/parity_test.rs` that runs against the streams and `assert_eq!`s the row counts.
5. **Verification**: run `cargo run --release -- --c2s ../streams/.../c2s.tcpstream.bin --s2c ../streams/.../s2c.tcpstream.bin --out out_rust` then `diff -r out_rust ../darkdec_output_streams/`. The diff must be empty (or differ only by file ordering / float formatting — document any allowed deltas).
6. Write `analysis/rust_port_2026-05-15.md` summarizing what was ported, the diff result, and any precision/format notes.
7. Set the closing fact via `harness fact-set`.

Print `RUST_DECODER_PORT_DONE` on the final line.

## Execution flow

**Step 1 — Set up Cargo crate:**
```bash
cd /home/sdancer/dark-december-rust-decoder
# Use Rust 2024 edition
cargo init --lib darkdec_decoder
# OR start from an existing scaffold:
ls /home/sdancer/dark-december-transport-encoder/ 2>/dev/null && \
  echo "(transport-encoder scaffold exists; can copy its Cargo.toml + src/lib.rs as a starting point)"
```

**Step 2 — Port adjacent_xor + frame parser:**
```rust
// src/lib.rs
pub fn adjacent_xor(body: &[u8]) -> Vec<u8> {
    if body.is_empty() { return Vec::new(); }
    (0..body.len()-1).map(|i| body[i] ^ body[i+1]).collect()
}

pub fn parse_stream(buf: &[u8]) -> Vec<Frame> {
    let mut frames = Vec::new();
    let mut off = 0;
    let mut ord = 0;
    while off + 4 <= buf.len() {
        let total = u32::from_le_bytes(buf[off..off+4].try_into().unwrap()) as usize;
        if total < 6 || off + total > buf.len() { break; }
        let raw = buf[off..off+total].to_vec();
        let dec = adjacent_xor(&raw[6..]);
        frames.push(Frame { ord, off, raw, dec });
        off += total;
        ord += 1;
    }
    frames
}
```

**Step 3 — Extract player / entity tracks** matching the Python `extract_tracks` exactly (see `/home/sdancer/orchestrator/darkdec_decoder.py:134-160`).

**Step 4 — CSV writer matching darkdec_output_streams format:**
- player_track.csv: columns `i,rot,t,x,z`
- entity_tracks.csv: columns `i,id,rot,t,x,z`
- entities_summary.csv: columns `first_t,id,last_t,last_x,last_z,updates`

**Step 5 — Validate:** `diff` against `/home/sdancer/orchestrator/darkdec_output_streams/`. Report `wc -l` row counts and any differences.

**Step 6 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 1 GB.** Rust is more memory-efficient than Python but cargo + LSP can spike.
- **NO new device interaction. NO Frida. NO MCP aeon.**
- **HARD output cap**: analysis artifact ≤300 lines (the Rust code itself is unbounded).
- **One Codex turn budget: ≤30 min wall time.**
- **Edition**: Use Rust 2024 (matches existing `dark-december-transport-encoder` scaffold).
- **Endianness**: All multi-byte fields are LITTLE-ENDIAN (length prefix, coords, IDs). Same as Python `struct.unpack_from('<...')`.
- **Adjacent-XOR offset**: `decoded[i] = raw_body[i] ^ raw_body[i+1]`, where `raw_body = frame[6:]`. The decoded buffer is 1 byte SHORTER than the raw body.
- **Coordinate validity gate**: Python rejects coords where `abs(value) >= 100000` or NaN/inf via `ok_coord`. Rust must replicate exactly.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-rust-decoder/`
- Python source (REFERENCE): `/home/sdancer/orchestrator/darkdec_decoder.py`
- Python baseline outputs (DIFF TARGET): `/home/sdancer/orchestrator/darkdec_output_streams/`
- TCP streams: `/home/sdancer/orchestrator/streams/first_quest/{first_quest_c2s.tcpstream.bin, first_quest_s2c.tcpstream.bin}` (490 KB c2s + 1.6 MB s2c)
- Existing Rust scaffold (may copy from): `/home/sdancer/dark-december-transport-encoder/src/lib.rs` + `Cargo.toml`
- success-fact key: `dark_december_rust_decoder_bitexact_parity_<n_frames>` (a)
- block-fact keys: `dark_december_rust_decoder_precision_drift` (b), `dark_december_rust_decoder_frame_count_mismatch` (c)
