# c2s-minor-classes — Catalog the 2,146 C2S frames outside the three dominant length classes

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-c2s-minor-classes`.

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_minor_classes` (new — planner-proposed Goal 4)
- **sub_goal_key**: `enumerate-secondary-c2s-length-buckets`

## Why this turn exists
Cycle 390 closed `dark_december_c2s_decoder` with 10,581 of 12,727 C2S frames classified across 3 dominant lengths: len=45 (6022 movement), len=10 (3607 ACK), len=61 (952 composite). That leaves **2,146 frames (17%)** in less-common length buckets uncategorized. These are likely chat / inventory / interact / quest-progress / NPC-dialog / cast-skill / pickup-item / equipment-swap — the second-most-valuable set of messages after movement.

Cycle 395 closed `dark_december_c2s_len45_field_semantics` with all 8 dominant field positions in len=45 named. So the dominant-class understanding is now thorough; turning to the long tail is the natural next coverage extension.

## Hypothesis
The remaining 2,146 C2S frames split into ≤8 secondary length classes, each ≥10 instances, each with internal byte-position regularity matching the len=45 signature pattern (low-distinct bytes at small offsets, opaque high-entropy bytes in body). Top secondary classes have semantic meaning identifiable by signature: action/interact frames will have a target-entity-id u16 or u32, chat frames will have ASCII string content visible after adjacent-XOR decode.

## Falsification (3 outcomes)
- (a) **≥1500 of 2146 frames classified into ≥4 named secondary classes (≥70% coverage), each with byte-position layout** → SUCCESS. Fact: `dark_december_c2s_minor_classes_<n>_classified_<m>_named`.
- (b) **Lengths bucket but bodies show no internal regularity (random high-entropy)** → minor classes use a different encoding than len=45/10/61. Fact: `dark_december_c2s_minor_classes_no_regularity`.
- (c) **A length bucket appears to contain ASCII text** (chat frames) → success on that subclass; document the chat protocol. Fact: `dark_december_c2s_minor_classes_chat_decoded`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-c2s-minor-classes/analysis/c2s_minor_classes_2026-05-15.md` with:
1. **Length histogram of the 2,146 remaining frames** (filter: not in {45, 10, 61}). Top-10 buckets with counts.
2. **For each bucket with ≥10 frames**: byte-position variance table (similar to cycle-390's len=45 analysis). Identify low-variance positions = type/subtype codes.
3. **ASCII detection sweep**: for each bucket, check if decoded body contains stretches of printable ASCII (looking for chat / item-name / NPC-name content).
4. **Candidate semantic names** per bucket based on byte patterns:
   - target-id present (small u16/u32 in low-variance position) → interact/action
   - ASCII content → chat
   - very short bodies → control / state-change
   - 2 or 4 bytes of zeros + small int → simple command
5. **Output CSV** `analysis/c2s_minor_decoded.csv` with at least 500 decoded records, columns: ord, length, bucket_name, decoded_hex, decoded_ascii.
6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `C2S_MINOR_CLASSES_DONE` on the final line.

## Execution flow

**Step 1 — Load + filter:**
```python
import sys
sys.path.insert(0, '/home/sdancer/orchestrator')
import darkdec_decoder as dd
from pathlib import Path
c2s = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin').read_bytes()
payloads = [('C2S', 0, 0, 0.0, '192.168.1.36', 65261, '158.101.105.58', 10001, c2s)]
frames = [f for f in dd.app_frames(payloads) if f['dir']=='C2S' and len(f['raw']) not in (45, 10, 61)]
print(f'minor-class frames: {len(frames)}')   # expect ~2146
```

**Step 2 — Histogram + filter buckets with ≥10 instances.**

**Step 3 — Per-bucket byte-position variance** (reuse the cycle-390 approach):
```python
from collections import Counter
def variance_table(bucket_frames):
    n = len(bucket_frames)
    body_len = len(bucket_frames[0]['dec'])
    table = []
    for i in range(body_len):
        c = Counter(f['dec'][i] for f in bucket_frames)
        table.append((i, len(c), c.most_common(1)[0]))
    return table
```

**Step 4 — ASCII detection:**
```python
import string
def ascii_runs(body, min_run=4):
    runs = []
    cur = []
    for b in body:
        if 32 <= b < 127:
            cur.append(chr(b))
        else:
            if len(cur) >= min_run: runs.append(''.join(cur))
            cur = []
    if len(cur) >= min_run: runs.append(''.join(cur))
    return runs
```

**Step 5 — Cross-correlate with S2C ACKs**: small C2S frames typically generate small S2C ACKs. Match by stream-ordinal proximity to confirm bucket semantics.

**Step 6 — Write markdown + CSV + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time.**
- The cycle-390 c2s-decoder analysis is the reference framework — replicate the byte-position variance methodology per bucket.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-minor-classes/`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin`
- Decoder (REUSE): `/home/sdancer/orchestrator/darkdec_decoder.py`
- Cycle-390 c2s-decoder analysis (methodology): `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md`
- Cycle-395 len45 field map (for cross-reference where bucket patterns overlap): `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
- success-fact key: `dark_december_c2s_minor_classes_<n>_classified_<m>_named` (a)
- block-fact keys: `dark_december_c2s_minor_classes_no_regularity` (b), `dark_december_c2s_minor_classes_chat_decoded` (c)
