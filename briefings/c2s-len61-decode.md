# c2s-len61-decode — Decode the third dominant C2S class (len=61 composite)

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-c2s-len61-decode` (worktree of `/home/sdancer/darkdecember/`, branch `c2s-len61-decode`).

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_len61_field_semantics` (new — planner-proposed Rank 3)
- **sub_goal_key**: `byte-decode-c2s-len61-composite`

## Why this turn exists
Cycle 390 closed C2S decoder with 3 dominant length classes named: len=45 (6022 movement), len=10 (3607 ACK), len=61 (952 composite). Cycle 395 fully field-mapped len=45 (8/8 positions). Cycle 397 catalogued the 21 minor-class buckets (97.4% coverage). **Len=61 is the last untouched dominant class** — 952 frames, expected to carry skill-cast / item-use / interaction-trigger payloads.

## Hypothesis
Len=61 frames are **movement-frame + 16-byte action-tail**: bytes 0-37 statistically match the cycle-395 len=45 field map (token@0 stable, marker@6=0x60, class_b@9..12 dominant words, subtype@33 small int, tail@37 boolean), and the trailing 16 bytes (offsets 38-53 in the decoded body) carry a discrete action payload — likely (action_type:u8, target_id:u32, params:u8[11]) or (skill_id:u16, target_id:u32, x/y deltas, flags).

## Falsification (3 outcomes)
- (a) **bytes 0-37 of len=61 match the len=45 field map (≥90% byte-position correspondence) AND tail bytes 38-53 have ≥3 low-variance positions identifiable as action_type / target_id** → SUCCESS. Fact: `dark_december_c2s_len61_composite_<n>_tail_named`.
- (b) **bytes 0-37 entropy/value distributions are INCOMPATIBLE with the len=45 field map** (token@0 not stable, marker@6 absent, class_b@9..12 distinct) → len=61 is a wholly distinct opcode, not movement+tail. Fact: `dark_december_c2s_len61_standalone_opcode`.
- (c) **bytes 0-37 match len=45 BUT tail bytes 38-53 are uniformly high-entropy (no low-variance positions)** → tail is encrypted or composite-of-composites. Fact: `dark_december_c2s_len61_tail_opaque`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-c2s-len61-decode/analysis/c2s_len61_decode_2026-05-15.md` with:

1. **Filtered frame set**: 952 C2S len=61 frames from `streams/first_quest/first_quest_c2s.tcpstream.bin`. Confirm count.
2. **Prefix-overlap test**: for each of the 8 named len=45 field positions (0,1,6,9..12,17,25,33,37), compute the value distribution in the len=61 frames and compare against the len=45 baseline. Report per-position correspondence score (0-1).
3. **Tail byte-position variance** (offsets 38-53 in decoded body, total 16 bytes): for each offset, distinct-value count, top-3 values, modal frequency. Identify low-variance positions = type/subtype codes.
4. **Tail u32 candidates**: at offsets 38, 42, 46, 50 (4-byte aligned), interpret as u32 LE and check (a) range — small ints likely action_type/skill_id; (b) high values likely target_id/entity_id; (c) check against the entity_tracks.csv id list from cycle-390.
5. **Tail f32 candidates**: same offsets, interpret as f32 LE. Check finite, magnitudes 0-100000 → likely delta-x / delta-z / rotation.
6. **ASCII detection** in tail (4-15 ASCII chars run): unlikely but worth a sweep.
7. **Output CSV** `analysis/c2s_len61_decoded.csv` with all 952 records, columns: `ord, prefix_match_score, tail_hex_38_53, tail_u32_38, tail_u32_42, tail_u32_46, tail_u32_50, candidate_class`.
8. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `C2S_LEN61_DECODE_DONE` on the final line.

## Execution flow

**Step 1 — Load + filter:**
```python
import sys
sys.path.insert(0, '/home/sdancer/orchestrator')
import darkdec_decoder as dd
from pathlib import Path
c2s = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin').read_bytes()
payloads = [('C2S', 0, 0, 0.0, '192.168.1.36', 65261, '158.101.105.58', 10001, c2s)]
frames61 = [f for f in dd.app_frames(payloads) if f['dir']=='C2S' and len(f['raw'])==61]
print(f'len=61 frames: {len(frames61)}')  # expect 952
```

**Step 2 — Prefix overlap test** (use cycle-395's named positions): build distributions, compare KL divergence or Jaccard overlap of top-10 values per position.

**Step 3 — Tail variance table** (cycle-390 methodology applied to offsets 38..53):
```python
from collections import Counter
def tail_variance(frames, start=38, end=54):
    n = len(frames)
    table = []
    for i in range(start, end):
        c = Counter(f['dec'][i] for f in frames if len(f['dec']) > i)
        table.append((i, len(c), c.most_common(3)))
    return table
```

**Step 4 — Tail u32/f32 sweep** (Python struct.unpack_from `<I` and `<f`).

**Step 5 — Target-id cross-reference**: load entity_tracks.csv from `/home/sdancer/orchestrator/darkdec_output_streams/`, get the 70 distinct ids, check if any tail u32 value matches an entity id (modulo 8-bit truncation).

**Step 6 — Write markdown + CSV + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 250 MB.** 952 frames is tiny.
- **NO new disasm. NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤400 lines.
- **One Codex turn budget: ≤20 min wall time.**
- Use the cycle-395 c2s-len45 analysis as the **statistical methodology template** — same byte-position variance + r²-cross-correlation patterns.
- Decoded body is `dec` (length = raw_body_length - 1, due to adjacent-XOR collapse). For len=61 raw frame, raw_body = 55 bytes, dec = 54 bytes. So tail offsets 38..53 are valid.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-len61-decode/` (branch `c2s-len61-decode`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin`
- Decoder (REUSE): `/home/sdancer/orchestrator/darkdec_decoder.py`
- Cycle-390 c2s-decoder analysis: `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md`
- Cycle-395 c2s-len45 field analysis (PRIMARY REFERENCE for field map): `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
- Entity id reference: `/home/sdancer/orchestrator/darkdec_output_streams/entities_summary.csv`
- success-fact key: `dark_december_c2s_len61_composite_<n>_tail_named` (a)
- block-fact keys: `dark_december_c2s_len61_standalone_opcode` (b), `dark_december_c2s_len61_tail_opaque` (c)
