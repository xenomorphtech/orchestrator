# dd-transport-encoder — Rust encoder/decoder for DD :10001 frames, byte-exact round-trip

## ONE TASK
Build a Rust crate at `/home/sdancer/dark-december-transport-encoder/` that decodes captured DD :10001 frames AND re-encodes them byte-exactly, then round-trip-test across all 167 captured frames. Goal: ≥165/167 frames pass round-trip. Pure offline, zero device dependency, zero AC exposure.

## CRITICAL — memory budget
**Hard limit 500 MB.** Pure code generation + pcap I/O. No large-binary disasm.

## Known substrate (canonical, do NOT re-discover)
Three frame classes already characterized + decoded:

1. **Outer framing** (100% closed):
   - `u32_le total_length + u16=0x0001 channel + N-byte obfuscated body`
   - Byte-exact, no variants.

2. **Heartbeats** (8B): single XOR mask per session
   - Mid-session: client mask `0x9d`/`0x9f`, server mask `0x11`
   - Session B: client `0x8c`/`0x8d`/`0x8e`, server `0x11`
   - Body of 2 bytes obfuscated → 1-byte latent payload

3. **Mid-session activity** (server41 / client45 affine layer):
   - server41: 35 wire body bytes → 17 latent (byte-pair-class XOR collapse)
   - client45: 39 wire body bytes → 22 latent
   - Cell-collapse rule recovered (cycle 113 body-decode artifact)
   - client45 first 4 latent bytes = LE f32 player X coord (confirmed cycle 134)

4. **Bootstrap frames** (5 lengths: 54/14/202/14/179, only on fresh handshake):
   - Client uses 0x8a/0x8b masks (bootstrap variant)
   - Server uses 0x11 (same as activity)
   - Latent bodies extracted per cycle-113 bootstrap-decode artifact
   - Semantic labels: F0/F2/F4=client stages A/B/C, F1/F3=server ACKs

## Inputs available
- 3 pcaps (167 total frames):
  - `/home/sdancer/dark-december-live-capture/captures/dd_live_10001_1778769976.pcap` (47 frames)
  - `/home/sdancer/dark-december-live-capture/captures/dd_live_10001_u_1778770093.pcap` (36 overlap with above)
  - `/home/sdancer/dark-december-handshake-capture/captures/dd_handshake_10001_1778771059.pcap` (84 frames inc. 5 bootstrap)
- Decoded latent jsonl: `/home/sdancer/dark-december-body-decode/analysis/decoded_frames.jsonl`
- Existing Python decoder (reference): `/home/sdancer/dark-december-minimap-decode/decoder.py`
- Closure artifacts: 
  - `/home/sdancer/dark-december-body-decode/analysis/body_decode_2026-05-14.md` (affine layer)
  - `/home/sdancer/dark-december-bootstrap-decode/analysis/bootstrap_decode_2026-05-14.md` (bootstrap masks)
  - `/home/sdancer/dark-december-minimap-decode/analysis/minimap_decode_2026-05-14.md` (X-coord confirmed)

## Success criteria
Deliverable: `/home/sdancer/dark-december-transport-encoder/Cargo.toml` + `src/lib.rs` + `src/bin/round_trip.rs` (or similar) that:
1. Parses pcap → extracts :10001 frames (TCP stream reassembly)
2. `decode(frame: &[u8]) -> DecodedFrame` (outer framing + body family classification + affine collapse)
3. `encode(decoded: &DecodedFrame) -> Vec<u8>` (inverse — produces original wire bytes)
4. Round-trip test: for each of the 167 captured frames, assert `encode(decode(frame)) == frame`
5. Report: # frames passed / 167

Closing artifact: `/home/sdancer/dark-december-transport-encoder/analysis/encoder_round_trip_2026-05-14.md` with:
- Per-frame-class pass rate (heartbeat / activity / bootstrap)
- Specific frames that don't round-trip + reason
- Final ratio (target ≥165/167 = 98.8%)

Set fact `nmss_transport_encoder_167_round_tripped` on success (≥165/167) OR `nmss_transport_encoder_partial_<N>_of_167` with the gap analysis.

## Concrete approach
- Use `pcap-parser` or `pnet` crate for pcap reading + TCP stream reassembly
- Define `enum FrameClass { Heartbeat, Activity, Bootstrap }` with discriminator based on length+direction
- Encode is the inverse of decode (apply affine expansion in reverse: latent body → byte-pair expanded with mask)
- The mask family depends on session — for the affine layer, use `mask = body[0] ^ body[1]` for each pair (already in jsonl)

## Constraints
- ONE crate build + test run. ≤8 minutes wall time.
- ≤500 MB memory budget.
- If `cargo` toolchain is unavailable: fall back to Python decoder + verify round-trip in Python. Document fallback in artifact.
- No device interaction.
- No new captures (use the 3 pcaps above).

## Falsification (acceptable outcomes)
- ≥165/167 frames round-trip → success, fact `nmss_transport_encoder_167_round_tripped`.
- 100-164/167 → partial, document which frame class fails: heartbeat (most likely all OK), activity (likely most pass), bootstrap (possible failures due to mixed-mask tails).
- <100/167 → fundamental misunderstanding of the affine layer; review the body_decode artifact's collapse rules.

## Relevant files / references
- worktree: `/home/sdancer/dark-december-transport-encoder/`
- pcaps: listed above
- reference decoder: `/home/sdancer/dark-december-minimap-decode/decoder.py`
- fact key (success): `nmss_transport_encoder_167_round_tripped`
