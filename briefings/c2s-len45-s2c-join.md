# c2s-len45-s2c-join — Name C2S len=45 fields via cross-correlation with S2C player track

## Role & workdir
Pure-Python protocol field-identification worker. Workdir: `/home/sdancer/dark-december-c2s-len45-s2c-join`.

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_len45_field_semantics` (new — planner-proposed cycle 391)
- **sub_goal_key**: `correlate-c2s-fields-with-s2c-player-coords`

## Why this turn exists
Cycle 390 closed `dark_december_c2s_decoder` at outcome (a): 6022 C2S len=45 frames classified as movement-request candidates with 14 low-variance positions. Count parity with S2C player updates is `6022 vs 6013` (delta +9, near 1:1). The remaining gap is naming the field semantics — particularly the candidate-coord position at decoded offset 17 whose range "overlaps S2C player x".

Cross-correlating len=45 C2S frames against their TEMPORAL NEIGHBORS in the S2C player track (the player's NEXT or PREVIOUS broadcast position) should pin which decoded byte positions carry the request x/z/yaw vs server-confirmed broadcast x/z/yaw.

## Hypothesis
Each C2S len=45 frame is a movement REQUEST sent before the corresponding S2C player-position BROADCAST. The decoded body at offset 17 is a LE-f32 request-x value that strongly correlates with the NEXT S2C player x. By computing r² across all 6022 pairs, we identify offset 17 as `request_x`, similar positions for z/yaw, and confirm or refute the remaining low-variance bytes as type-codes / sequence-numbers / flags.

## Falsification (3 outcomes)
- (a) **≥6 of 8 candidate field positions (token@0, subtype@1, marker@6, class_b@9-12, coord@17, scalar@25, subtype@33, tail@37) verified with documented semantics + numeric residuals** → SUCCESS. Fact: `dark_december_c2s_len45_fields_<n>_named`.
- (b) **Some fields confirmed but ≥2 stay "opaque"** → partial success; document the remaining unknowns. Fact: `dark_december_c2s_len45_fields_partial_<n>`.
- (c) **C2S frames don't temporally align with S2C player broadcasts** (e.g. they're for different actors or out of stream order) → cross-correlation fails. Fact: `dark_december_c2s_len45_temporal_misalignment`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md` with:
1. **Pair construction**: for each C2S len=45 frame at stream position `i_c2s`, pair it with the nearest S2C player-position frame at stream position `j_s2c` (use ordinal index since timestamps are 0.0 from our payload synthesis). Method: parse C2S + S2C frame ordinals together via `darkdec_decoder.app_frames`, sort by ordinal-in-direction, then for each C2S len=45 frame find the next S2C 41-byte player-position frame.
2. **For each candidate field position** in {17, 25, 13..16, 21..24, 29..32}, interpret the 4 bytes as LE-f32, LE-u32, LE-i32. Compute Pearson r² against:
   - paired-S2C player_x
   - paired-S2C player_z
   - paired-S2C player_yaw
   - DIFFERENCE from prior C2S frame's same field (delta x, delta z) — likely captures heading vector
3. **Identify the best (offset, interpretation) → semantic_meaning mapping** for each field position.
4. **Field-position summary table** with columns: offset, interpreted_type, best_match_target, r², candidate_name, confidence.
5. **Cross-validate** the named position by reconstructing 5 sample frames and showing their decoded values match expected player movement.
6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `C2S_LEN45_FIELDS_DONE` on the final line.

## Execution flow

**Step 1 — Load both streams via the existing decoder:**
```python
import sys, struct, csv
sys.path.insert(0, '/home/sdancer/orchestrator')
import darkdec_decoder as dd
from pathlib import Path
c2s = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin').read_bytes()
s2c = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin').read_bytes()
payloads = [
    ('C2S', 0, 0, 0.0, '192.168.1.36', 65261, '158.101.105.58', 10001, c2s),
    ('S2C', 0, 1, 0.0, '158.101.105.58', 10001, '192.168.1.36', 65261, s2c),
]
frames = dd.app_frames(payloads)
```

**Step 2 — Build paired list**: walk frames in stream order. Each time you see a C2S len=45 frame, store its current decoded body. Each time you see an S2C 41-byte player-position frame (`dec[:4]=='1202606d'`), pop the most recent unmatched C2S frame and pair them.

**Step 3 — Compute correlations**: for each of 5 candidate 4-byte slots in C2S decoded body, try 6 interpretations × 4 target signals × Pearson r. Pick best.

**Step 4 — Generate output CSV `analysis/paired_records.csv`** with columns: c2s_ord, s2c_ord, c2s_dec_at_offset_17_f32, c2s_dec_at_offset_25_f32, ..., s2c_x, s2c_z, s2c_yaw.

**Step 5 — Markdown summary table + verdict + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Pure-Python; small data.
- **NO new disasm. NO new memdump. NO Frida. NO live device. NO MCP aeon.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time.**
- The S2C player frames carry coord at `f32 @ dec_offset 9` and `f32 @ dec_offset 17` per the cycle-383 darkdec_decoder validation (NOT 17, 25 — double-check by reading darkdec_decoder.extract_tracks). Use the same constants as the validated decoder.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-len45-s2c-join/`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/{c2s,s2c}.tcpstream.bin`
- Validated decoder (REUSE): `/home/sdancer/orchestrator/darkdec_decoder.py`
- Cycle-383 baseline output: `/home/sdancer/orchestrator/darkdec_output_streams/player_track.csv`
- Cycle-390 c2s analysis (variance table, sample records): `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md`
- success-fact key: `dark_december_c2s_len45_fields_<n>_named` (a)
- block-fact keys: `dark_december_c2s_len45_fields_partial_<n>` (b), `dark_december_c2s_len45_temporal_misalignment` (c)
