# dd-minimap-decode — Turn 2: second coord + server41 entity decode + minimap renderer

## Role & workdir
Protocol decoder, second turn. Workdir: `/home/sdancer/dark-december-minimap-decode`. Existing artifact `analysis/minimap_decode_2026-05-14.md` has Task 1 closure. **This turn must complete Tasks 2-5 — do not stop after one task. Append to the same artifact; set the fact at the end.**

## Current goal / sub-goal
- Goal: `dark_december_minimap_decode` — render minimap with player + monsters + NPCs.
- Sub-goal: Turn 2 = decode the rest of client45 + server41 frames to enable minimap.

## Confirmed from Turn 1 (canonical)
- **client45 latent[0..4] = LE f32 = player X coordinate.** Two smooth monotonic ramps observed: 2.882805→2.886984 (burst 1, 4 frames, ~100ms cadence) and 2.924907→2.935379 (burst 2, 5 frames). Deltas ~1.4e-3 per tick = slow walking.
- 2 frames have 184.x at the same offset — different client45 subtype (event end / state transition).
- BE f32 / LE u32 / LE i32 all falsified.

## Success criteria
Append "## Task 2 — Y/Z coord", "## Task 3 — server41 entity decode", "## Task 4 — decoder build", "## Task 5 — sample minimap" sections to the existing artifact.

Deliverable: a working Rust crate (`Cargo.toml` + `src/main.rs`) OR Python decoder script that reads one of the existing pcaps and emits minimap-renderable game state.

Set fact `dark_december_minimap_decoder_ships` on success OR `dark_december_minimap_partial_<what_missing>` if some piece needs live capture (e.g., monsters never visible in idle captures).

## Concrete tasks (DO ALL IN THIS TURN, do not stop after Task 2)

### TASK 2 — second coord in client45
Test latent[4..8] and latent[8..12] as LE f32. For each:
- Across the same 11 unique client45 frames, compute the f32 value
- Check for smooth-trajectory behavior (similar to Task 1's X coord)

Expected layout: `(x, y)` 2D = 8 bytes, or `(x, y, z)` 3D = 12 bytes. Document which offsets give plausible game coords. The first burst is slow walking — second coord should change smoothly OR stay roughly constant if movement is along one axis.

### TASK 3 — server41 entity decode
The 22 client45 frames are paired with 29 server41 frames in the captures (per the dataset table). Server41 latent body = 17 bytes.

For each server41 latent body, try the same f32 hypothesis at multiple offsets [0..4], [4..8], [8..12]. If we see multiple distinct (id, x, y) triples that move smoothly across consecutive server41 frames — those are entity updates.

Look specifically for:
- u16/u32 entity_id near the start (would repeat across frames for the same entity)
- f32 x, f32 y at some fixed offset
- u8/u16 kind/state byte

Group server41 frames by inferred entity_id (whatever consistent u16/u32 word appears) — if same entity has continuous position deltas, that's the entity_position semantic.

### TASK 4 — build decoder
Minimal Rust crate or Python script:
- Reads pcap
- Extracts :10001 frames
- Applies known framing (u32_le len + u16 0x0001 + body)
- Applies known affine layer (server `0x11`, client `0x9d`)
- Emits JSON stream of `{timestamp, type, fields}` per frame
- Renders ASCII minimap (player at center, entities as `*`, etc.) at end-of-capture state

### TASK 5 — close
Append "## Verdict" with per-frame-type confidence. Set `dark_december_minimap_decoder_ships` fact via `/home/sdancer/orchestrator/harness fact-set`.

## CRITICAL: stay in this turn
- Do NOT stop after Task 2 thinking "Task 2 is complete".
- Tasks 2, 3, 4, 5 must ALL run before the turn ends.
- If a task hits a clean falsification, document it in 1-2 paragraphs and proceed to the next task.
- The turn should produce: appended analysis sections + a working decoder + a sample render + a fact-set.

## Constraints & gotchas
- **No device interaction.** All pcap data is on disk.
- **Memory budget**: ≤500 MB. Don't load full pcaps into a single buffer; use the existing decoded_frames.jsonl.
- **No source-tree scanning.** Don't read Unreal Engine source or anything similar.
- If existing captures lack monster/NPC frames (only idle player movement): set fact `dark_december_minimap_decoder_needs_live_gameplay_pcap` and document the gap. The decoder skeleton should still ship even if entity decoding is partial.

## Falsification (acceptable outcomes)
- Y/Z coords clearly identified at [4..8] or [8..12] → confirmed; decoder emits full position.
- No second-coord pattern → mark as 1D position; document what byte ranges WERE explored.
- Server41 entity decoding works → entity records emitted.
- Server41 has no consistent entity_id structure → set fact `dark_december_server41_entity_id_not_localized` with what was tried; the decoder skeleton ships with player-only state.

## Relevant files / references
- worktree: `/home/sdancer/dark-december-minimap-decode/`
- existing artifact (APPEND to it): `analysis/minimap_decode_2026-05-14.md`
- decoded jsonl: `/home/sdancer/dark-december-body-decode/analysis/decoded_frames.jsonl`
- pcaps: `/home/sdancer/dark-december-live-capture/captures/dd_live_10001_1778769976.pcap`, `/home/sdancer/dark-december-handshake-capture/captures/dd_handshake_10001_1778771059.pcap`
- prior closure: `/home/sdancer/dark-december-body-decode/analysis/body_decode_2026-05-14.md`
- fact key (success): `dark_december_minimap_decoder_ships`
