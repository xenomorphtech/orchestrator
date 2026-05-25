## raw-reconstruct — Reconstruct raw_body bytes from decoded view + seed byte; resolve 3 opaque coord_params

## Role & workdir
Pure-Python statistical-analysis worker. Workdir: `/home/sdancer/dark-december-raw-reconstruct` (worktree of `/home/sdancer/darkdecember/`, branch `raw-reconstruct`).

## Current goal / sub-goal
- **goal_key**: `dark_december_frzmove_br_raw_reconstruction` (new)
- **sub_goal_key**: `invert-adjacent-xor-to-read-all-6-coord-params`

## Why this turn exists
Cycle 433 found that the adjacent-XOR decoded view is NOT a byte-for-byte image of the 6 serialized `coord_param_f32` slots in FRzMoveBr. The transform `dec[i] = raw_body[i] ^ raw_body[i+1]` loses 1 byte of information (`raw_body` is `dec_len+1` bytes). If we can **seed** `raw_body[0]` from a known constant, we can reconstruct the full raw_body and read all 6 coord_param floats at their canonical Serialize offsets.

Cycle 415 named `FRzMoveBr::Serialize` as `msg_type u16 (offsets 0..1) + actor_handle u64 (2..9) + 6 × f32 (10..33) + move_flag u8 (33)`. Cycle 415 also found `FRzMoveBr::GetType` returns `0x0386` → little-endian raw_body[0]=`0x86`, raw_body[1]=`0x03`. THAT is the seed.

## Hypothesis
Setting `raw_body[0] = 0x86` for every FRzMoveBr frame and reconstructing `raw_body[i+1] = raw_body[i] ^ dec[i]` gives a full 33-byte raw_body whose f32 reads at offsets `[10, 14, 18, 22, 26, 30]` produce 6 clean, finite, bounded floats per slot. Per-slot distribution analysis then names the previously-opaque `coord_param_1/_3/_5`.

## Falsification (3 outcomes)
- (a) **All 6 reconstructed slots are finite + bounded + show distinct semantic structure** (coord vs heading vs velocity vs padding) → SUCCESS. Fact: `dark_december_frzmove_br_raw_reconstructed_6_slots_named`.
- (b) **The seed `raw_body[0]=0x86` produces nonsense reads** (most slots NaN / Inf / wildly out of range) → seed byte assumption is wrong; try alternates (0x12 — observed prefix; 0x00; 0xff). If none work, the wire format has a different inner framing. Fact: `dark_december_frzmove_br_raw_reconstruction_seed_falsified`.
- (c) **Reconstruction works but slots remain opaque** (finite floats but no semantic structure beyond cycle 433's findings) → reconstruction is correct but those slots genuinely don't carry geometric data. Fact: `dark_december_frzmove_br_raw_reconstruction_<n>_named`.

## Success criteria — SINGLE TURN, do all before stopping

**Primary**: write `/home/sdancer/dark-december-raw-reconstruct/analysis/raw_reconstruct_2026-05-15.md` with:

1. **Load all 23,568 FRzMoveBr frames** from the first_quest streams via `darkdec_decoder.py`. Filter strictly for `dir==S2C and raw_len==41 and dec.startswith([0x12, 0x02, 0x60, 0x6d])` (the player shape) PLUS the entity shape `[0x12, <id>, 0x86, 0x01, 0x00, 0x00, 0x00, 0x46, 0x11]`. Verify count.

2. **Try 4 seed bytes** for `raw_body[0]`: `0x86` (cycle-415 packet_id_low), `0x12` (the observed decoded prefix), `0x00`, `0xff`. For each seed, reconstruct the full 33-byte raw_body for the first 10 frames and check whether `raw_body[0..1]` reads as `0x0386` little-endian (the expected packet_id from `GetType`). Report seed evaluation.

3. **For the winning seed** (the one whose reconstruction yields raw_body[0..1] == [0x86, 0x03]):
   - Reconstruct all 23,568 frames.
   - At Serialize offsets `[10, 14, 18, 22, 26, 30]` read 6 little-endian f32 values per frame.
   - Compute per-slot stats: `min / max / mean / std / median / p1 / p99 / % finite / % near-zero / % bounded`.

4. **Cross-validate**: the f32 at offset 10 (coord_param_0) should reproduce the existing `player_track.csv` x column for the 6013 player frames. If it does, the reconstruction is verified.

5. **Per-slot range classification**:
   - **coord-like**: range > 1000, distinct count > 1000, no clustering → x / y / z position
   - **angle-like** (radians): range ⊂ [-π, π] or [0, 2π] → yaw / pitch / roll
   - **normalized**: range ⊂ [-1, 1] → direction cosine / heading
   - **velocity-like**: smaller bounded range with quantization → velocity component
   - **padding-like**: mostly 0 or constant → reserved / unused

6. **Final 6-slot naming table** with confidence + range evidence.

7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `RAW_RECONSTRUCT_DONE` on the final line.

## Execution flow

**Step 1** — Filter frames + verify count.
**Step 2** — Per-seed reconstruction test (4 seed bytes × first 10 frames).
**Step 3** — Bulk reconstruction with winning seed.
**Step 4** — Per-slot stats + player_track.csv cross-validation.
**Step 5** — Range classification.
**Step 6** — Naming table + verdict + fact-set.

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Per-frame 33-byte reconstruction + numpy float arrays — well within budget.
- **NO disasm. NO Frida. NO live device.** Pure-data inversion.
- **HARD output cap**: artifact ≤300 lines.
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- The cycle-403 `FRzMoveBr x@9` empirical anchor is the ground truth: reconstruction at Serialize offset 10 must match decoded offset 9's existing player_track x values EXACTLY (modulo any subtle endian / alignment difference).
- Adjacent-XOR inversion: `raw_body[i+1] = raw_body[i] ^ dec[i]` (chain forward from seed). `raw_body` length is `dec_len + 1`.
- Cross-pollination memory: [[project_dark_december_wire_framing_plus8]] (+8 outer frame), [[project_dark_december_wire_decoder]] (adjacent-XOR).

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-raw-reconstruct/` (branch `raw-reconstruct`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- Baseline (cross-validation target): `/home/sdancer/orchestrator/darkdec_output_streams/{player_track,entity_tracks}.csv`
- FRzMoveBr Serialize layout (cycle 415): `/home/sdancer/dark-december-frzmove-br-decode/analysis/frzmove_br_decode_2026-05-15.md`
- Coord_param disambig (cycle 433): `/home/sdancer/dark-december-coord-param-disambig/analysis/coord_param_disambig_2026-05-15.md`
- success-fact key: `dark_december_frzmove_br_raw_reconstructed_6_slots_named` (a)
- block-fact keys: `dark_december_frzmove_br_raw_reconstruction_seed_falsified` (b), `dark_december_frzmove_br_raw_reconstruction_<n>_named` (c)
