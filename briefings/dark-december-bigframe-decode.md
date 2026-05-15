# dark-december-bigframe-decode — Decode the 7 big single-instance server frames

## Role & workdir
You are an offline crypto/protocol decoder. Workdir: `/home/sdancer/dark-december-bigframe-decode/`.

## Current goal / sub-goal
- **goal_key**: `dark_december_minimap_decode` (currently 0.6/1.0, target 1.0)
- **sub_goal_key**: `bigframe-decode` (this path; partially advances 0.6→up to ~0.85 if successful)

## Hypothesis
The 7 single-instance large server-to-client frames captured on the DD :10001 stream are world-state-snapshot deliveries sent once per session entry — and they almost certainly carry entity (monster/NPC) records (id, type, x, y, possibly z, hp, name?). Decoding them via the SAME server-side byte-pair-XOR-collapse affine layer used for the 41-byte activity frames (or a derivative variant) should yield latent bodies with structured records.

**Evidence**: First 40 wire bytes of each big server frame show characteristic *equal-byte runs* (e.g. `486f6c6c6c6c6c6d` = `H` + 7×`l`; `5a6162626262626296` = `Z` + 6×`b`; `4f1714141414141553` = `O` + 7×`0x14`). Under the affine pair-XOR rule, equal byte pairs collapse to **zero** in latent — which is exactly what a padded record-array would look like (long zero-runs between sparse fields).

## Falsification
Either (a) the pair-XOR-collapse rule applied to a big frame yields no zero-rich latent body (i.e., the run pattern was a coincidence and the cipher mask is different), OR (b) the latent body has zero-runs but no structured records emerge under field-cross-correlation across the 7 frames, OR (c) records exist but they are server-internal (textures/geometry/patches) without entity-position fields.

## Success criteria
**Primary**: produce `/home/sdancer/dark-december-bigframe-decode/analysis/bigframe_decode_2026-05-14.md` answering — for each of the 7 big frames — (1) does the affine layer apply (zero-rich latent yes/no), (2) does cross-frame structural analysis reveal records (count + likely shape), (3) are any cross-correlatable with the 11 player-position client45 frames (e.g., does any frame contain `e17f3840` or LE-f32 player-X-near-2.88 fields visible)?

**Fact key on success**: `dark_december_bigframe_decoded_N_of_7` where N is the count of frames yielding zero-rich latent.

**Partial success (acceptable)**: 1-7 frames yield zero-rich latent and at least one structural pattern emerges (e.g., "16-byte records with f32 X and f32 Y starting at offset K, repeated R times" or "8-byte records with u32 id and u8 type"). Set fact even at partial.

**Falsification artifact**: even on full-negative, write the artifact + set fact `dark_december_bigframe_undecodable_no_affine_match`.

## Progress so far
**Turn 1 (cycle 155) COMPLETE — Task 1 done.** Artifact: `/home/sdancer/dark-december-bigframe-decode/analysis/bigframes_raw.json` keyed by length (267/407/436/698/1580/9751/11331), with body_hex + wire_length + body_len + timestamp + pcap for each frame. Extractor script: `scripts/extract_bigframes_raw.py`.

**Now on Turn 2 (cycle 156)**: continue with Task 2 onward; do NOT re-run Task 1, do NOT exit after Task 2.

The minimap-decode work prior to this path:
- Body-decode artifact `/home/sdancer/dark-december-body-decode/analysis/body_decode_2026-05-14.md` characterized the activity-class affine layer: pairs of wire bytes XOR-collapse to single latent bytes; server uses mask 0x11 for the 41-byte server activity frames.
- Bootstrap-decode artifact `/home/sdancer/dark-december-bootstrap-decode/analysis/bootstrap_decode_2026-05-14.md` mapped 5 bootstrap-frame masks (server 0x11; client 0x8a/0x8b bootstrap variants).
- Minimap-decode artifact `/home/sdancer/dark-december-minimap-decode/analysis/minimap_decode_2026-05-14.md` confirmed client45[0..4] = LE f32 player X coord (values 2.88-2.93 in steady movement + 184 burst-end outlier).
- Transport-encoder artifact `/home/sdancer/dark-december-transport-encoder/analysis/encoder_round_trip_2026-05-14.md` shipped 174/174 byte-exact round-trip (Rust crate at `/home/sdancer/dark-december-transport-encoder/`).
- The 7 big frames have NOT yet been semantically decoded. They are in the "unknown_raw=42" bucket of the transport encoder (lossless passthrough, not structural decode).

**Substrate available:**
- `/home/sdancer/dark-december-body-decode/analysis/decoded_frames.jsonl` (167 logical frames, includes `body_hex` for each)
- `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` + `frames_u.jsonl` (raw)
- The 3 pcaps at `/home/sdancer/dark-december-live-capture/captures/` + `/home/sdancer/dark-december-handshake-capture/captures/`
- The 11 unique decoded client45 player-position frames (timestamps + LE f32 values) in the minimap-decode artifact.

## Execution flow — DO NOT EXIT BETWEEN STEPS

Execute Tasks 2 → 3 → 4 in a SINGLE python script + single artifact-write + single fact-set. The Codex worker has historically exited after declaring "Task X complete" — that is the failure mode this briefing is rewritten to avoid. Treat the four steps below as ONE atomic unit. Do not summarize and stop; do not ask for confirmation; run all steps and only then return.

**Task 2** — For EACH of the 7 frames in `analysis/bigframes_raw.json`, apply the affine pair-XOR-collapse rule from `/home/sdancer/dark-december-body-decode/analysis/body_decode_2026-05-14.md`: take `body_hex` bytes (these are already the after-header obfuscated payload), then for each pair of consecutive bytes `(b[2i], b[2i+1])` produce `latent[i] = b[2i] ^ b[2i+1]` and tally:
  - distribution of latent byte values (expect heavy 0x00 bias if hypothesis is correct)
  - longest zero-run
  - top-5 most-common latent byte values
  - latent length and parity (odd body_len leaves 1 trailing byte; record it separately)
Write the per-frame summary table inline into the final artifact (no intermediate artifact needed).

**Task 3** — For each frame whose latent is "zero-rich" (≥30% zero bytes OR ≥8-byte longest zero-run, your choice of threshold but state it):
  - Identify candidate record boundaries from long zero-runs
  - Try LE-f32 readout at every byte offset across the latent; cross-check against player-X values from the minimap artifact (range ~2.88-2.93, plus burst ~184). Save any offset where the f32 looks like a plausible game-world coordinate (-1000 < x < 1000 OR exactly equal to a player position).
  - Try LE-u32 readout for entity-id candidates (small consecutive integers in 0..10000 range, especially at offsets immediately preceding or following the f32 hits)
  - Document any cross-frame invariants: same fixed-shape header bytes across all 7 frames are protocol-level structure markers.
  - If pair-XOR FAILS (no zero-rich latent), also try the two fallbacks listed in Constraints below before declaring negative.

**Task 4 — ALWAYS RUN, regardless of T2/T3 outcome**:
  - Write `/home/sdancer/dark-december-bigframe-decode/analysis/bigframe_decode_2026-05-14.md` with: (a) hypothesis statement + falsification criterion verbatim from this briefing, (b) per-frame T2 zero-rich verdict table, (c) per-frame T3 structural findings (entity-record schema sketch if found, or "no schema" with what was checked), (d) cross-frame invariants section, (e) final verdict: "N of 7 frames yielded zero-rich latent and M of those produced an entity-record schema sketch".
  - Set fact via `/home/sdancer/orchestrator/harness fact-set <key> "<value>"`:
    - If N >= 1: `dark_december_bigframe_decoded_N_of_7` with value summarizing what was decoded.
    - If N == 0: `dark_december_bigframe_undecodable_no_affine_match` with value summarizing fallbacks tried.

Only stop after step 4's fact-set succeeds. Print "BIGFRAME_DECODE_DONE" on the final line of your reply.

## Constraints & gotchas
- **Hard 500 MB memory budget.** Pure pcap-jsonl analytical work in Python. NO large-binary disasm.
- **No device interaction.** No adb, no Frida, no uprobes. Pure offline.
- **No re-decoding from raw pcap.** Work from the already-decoded jsonl. The `body_hex` field IS the affine-input body bytes.
- **Body length = wire length - 6 bytes** (4B total_length + 2B 0x0001 channel header are NOT in `body_hex`; body_hex is just the after-header obfuscated payload).
- **Affine for server is 0x11 for 41-byte activity frames**, but the big frames MIGHT use a different per-class mask. If pair-XOR gives unstructured output, try also: (a) XOR every byte with 0x11 first, then pair-collapse; (b) take latent[i] = b[2i] (drop second byte); (c) consider variable-length records where the affine doesn't apply at all.
- **One Codex turn budget**: ≤8 minutes wall time, single end-to-end script generation + execution. Hand back the artifact + fact.

## Relevant files / references
- decoded jsonl: `/home/sdancer/dark-december-body-decode/analysis/decoded_frames.jsonl`
- body-decode artifact (affine layer rules): `/home/sdancer/dark-december-body-decode/analysis/body_decode_2026-05-14.md`
- bootstrap artifact (mask families): `/home/sdancer/dark-december-bootstrap-decode/analysis/bootstrap_decode_2026-05-14.md`
- minimap artifact (player-X confirmation): `/home/sdancer/dark-december-minimap-decode/analysis/minimap_decode_2026-05-14.md`
- transport encoder (round-trip reference impl): `/home/sdancer/dark-december-transport-encoder/`
- fact key (success): `dark_december_bigframe_decoded_N_of_7`
- fact key (full-negative): `dark_december_bigframe_undecodable_no_affine_match`
