# c2s-decoder — Decode client-to-server frame types in the dark-december stream

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-c2s-decoder`.

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_decoder` (new — extends the now-closed `dark_december_move_message_decoded`)
- **sub_goal_key**: `catalog-c2s-frame-types-and-decode-fields`

## Why this turn exists
Cycle 383 validated the documented `decoded[i] = raw_body[i] ^ raw_body[i+1]` adjacent-XOR wire cipher against `streams/first_quest/first_quest_s2c.tcpstream.bin` and produced full S2C decoder outputs: 6,013 player updates + 17,555 entity updates across 71 IDs. The C2S direction (`streams/first_quest/first_quest_c2s.tcpstream.bin`, 12,727 frames) was NOT decoded — that's the value left on the table.

C2S frame-length histogram from cycle 383:
- `len=45`: 6022 instances — matches the cycle-346 FRzMoveRq/FRzStandRq layout (39-byte body + 6-byte header)
- `len=10`: 3607 — short body, probably input ACKs or keepalives
- `len=61`: 952
- `len=109`: 342
- `len=18`: 320

## Hypothesis
The C2S frames follow the SAME wire format as S2C (4B LE length + 2B channel + adjacent-XOR body). Decoded bodies carry MOVEMENT REQUESTS (corresponding to S2C movement broadcasts), action commands, chat, and protocol-level control messages. The dominant `len=45` frames decode to a structure compatible with cycle-346's recovered FRzMoveRq layout: msg_type:u16 + seq_or_kind:u32 + actor_handle:u64 + 6×(u32 coord/param) + flag:u8.

## Falsification (3 outcomes)
- (a) **≥3 distinct C2S frame-type signatures decoded with named semantic fields, each with ≥10 instances in the corpus** → SUCCESS. Fact: `dark_december_c2s_decoder_<n>_types_<m>_total_frames_decoded`.
- (b) **Frame lengths catalog but decoded body bytes don't match any recognizable pattern** → cipher might differ for C2S, or body is binary protobuf-like → document the gap. Fact: `dark_december_c2s_body_format_uncertain`.
- (c) **C2S adjacent-XOR decode fails consistency checks** → C2S uses a different cipher than S2C (unlikely, but possible — the cycle-358 c2s "body[2]==body[3]" finding was raw, supporting same scheme). Fact: `dark_december_c2s_cipher_differs_from_s2c`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md` with:
1. **Frame-type catalog**: for each distinct (length, decoded-body-first-4-bytes) signature with ≥10 instances:
   - Length, prefix, count, fraction of all C2S frames
   - Candidate semantic name (movement request, action, chat, etc.)
   - Decoded-body byte-position layout
2. **Field inference** for the top 3-5 types: for each 1/2/4/8-byte slot, is it u8 / u16 / u32 / u64 / f32? Cross-frame variance analysis tells you.
3. **Python decoder** (≤200 lines) that parses C2S frames and produces typed records.
4. **CSV output** at `analysis/c2s_decoded_top_types.csv` with at least 1000 decoded records.
5. **Cross-correlation with S2C**: do C2S MoveRq timestamps align with S2C player-position broadcasts? Match by actor_handle if recoverable.
6. Verdict matched to (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `C2S_DECODER_DONE` on the final line.

## Execution flow

**Step 1 — Reuse darkdec_decoder.py's frame parser:**
```python
import sys
sys.path.insert(0, '/home/sdancer/orchestrator')
import darkdec_decoder as dd
from pathlib import Path

c2s = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin').read_bytes()
payloads = [('C2S', 0, 0, 0.0, '192.168.1.36', 65261, '158.101.105.58', 10001, c2s)]
frames = dd.app_frames(payloads)
c2s_frames = [f for f in frames if f['dir'] == 'C2S']
print(f'parsed {len(c2s_frames)} c2s frames')
```

**Step 2 — Catalog by (length, decoded-body-prefix):**
```python
from collections import Counter
sig = Counter((len(f['raw']), f['dec'][:4].hex()) for f in c2s_frames)
print('top signatures:')
for (length, prefix), count in sig.most_common(20):
    print(f'  len={length} prefix={prefix} count={count}')
```

**Step 3 — Pick top 3-5 signatures by count and decode their bodies:**
For each top-N signature, gather all instances, decode bodies, look at byte distributions per position:
- Position with low entropy → likely a fixed type-ID or flag
- Position with high entropy + small u16 range → counter
- Position with float-like 32-bit values → coordinate
- Position with monotonic 32/64-bit values → timestamp / sequence

**Step 4 — Cross-reference with cycle-346 FRzMoveRq layout:**
- The cycle-346 layout: offset 0 = msg_type:u16 (0x0385 Stand, 0x0386 Move), offset 2 = seq_or_kind:u32, offset 6 = actor_handle:u64, offset 14-37 = 6×u32 coords/params, offset 38 = flag:u8.
- For len=45 (39-byte body) C2S frames, verify the decoded body has msg_type in `{0x0385, 0x0386}` (LE: `85 03` or `86 03` at decoded offset 0).

**Step 5 — Write decoder + CSV + markdown + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Pure Python, small dataset (490 KB c2s stream).
- **NO new disasm. NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time.**
- The cycle-322/346 corpus from cycles 320-372 (the 47-frame `frames.jsonl` in `dark-december-body-decode/`) DOES NOT cleanly cross-reference with this new stream because the cycle-322 capture was a different session with different actor handles. Use ONLY `streams/first_quest/` data as ground truth.
- **The adjacent-XOR decoder** is the validated decoder from cycle-383 (`darkdec_decoder.py:adjacent_xor`). Do NOT re-implement.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-decoder/`
- Streams (read-only): `/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin`, `first_quest_s2c.tcpstream.bin`
- S2C decoder + frame parser (REUSE): `/home/sdancer/orchestrator/darkdec_decoder.py`
- S2C reference output (for cross-correlation): `/home/sdancer/orchestrator/darkdec_output_streams/player_track.csv`
- Cycle-346 cycle-372 FRzMoveRq field layout (for hypothesis cross-check): `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`
- README: `/home/sdancer/orchestrator/README.md` (mentions packet shape, decoded prefixes)
- success-fact key: `dark_december_c2s_decoder_<n>_types_<m>_frames_decoded` (a)
- block-fact keys: `dark_december_c2s_body_format_uncertain` (b), `dark_december_c2s_cipher_differs_from_s2c` (c)
