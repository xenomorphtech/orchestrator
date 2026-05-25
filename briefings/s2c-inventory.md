# s2c-inventory — Catalogue S2C frame classes (the protocol's other half)

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-s2c-inventory` (worktree of `/home/sdancer/darkdecember/`, branch `s2c-inventory`).

## Current goal / sub-goal
- **goal_key**: `dark_december_s2c_inventory` (new — planner-proposed Rank 1)
- **sub_goal_key**: `bucket-and-name-s2c-frame-classes`

## Why this turn exists
C2S protocol is ~97% decoded (3 dominant classes named + 21 minor buckets / 97.4% coverage + len=45 fully field-mapped). S2C currently has only ONE class named: len=41 player/entity position update (6013 player + 17555 entity updates extracted). The remaining **~11,700 S2C frames** (of 23,570 total) are unclassified — almost certainly the bulk of the in-game protocol: world-state, NPC dialog, chat, inventory deltas, combat events, quest progress, login/lobby messages.

Closing this path unlocks downstream work on C2S↔S2C correlation, request/response semantics, and any real-time decoder embedding.

## Hypothesis
The S2C stream (23,570 frames, 1.67 MB after reassembly) partitions cleanly into ~15-25 length-bucketed opcode classes, with the **4-byte decoded body prefix** (e.g. the known `12 02 60 6d` for the len=41 position frame) acting as a stable opcode tag across all classes. Top-10 length buckets cover ≥80% of frames; each ≥10-instance bucket has internal byte-position regularity matching the C2S signature pattern (low-distinct bytes at small offsets, opaque high-entropy bytes in body).

## Falsification (3 outcomes)
- (a) **≥80% of 23,570 S2C frames bucketed AND ≥10 opcode prefixes (4-byte) named by occurrence + correlation against C2S** → SUCCESS. Fact: `dark_december_s2c_inventory_<n>_classified_<m>_named`.
- (b) **Length histogram is heavy-tailed with no peak holding >2% of frames AND leading 4 bytes vary uniformly within each length bucket** → S2C is multiplexed sub-frames OR uses a different header convention. Fact: `dark_december_s2c_no_length_buckets`.
- (c) **A length bucket appears to contain ASCII text** (chat / NPC dialog) → success on that subclass; document the protocol. Fact: `dark_december_s2c_text_decoded`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md` with:

1. **Length histogram of all 23,570 S2C frames** (filter: `dir=='S2C'`). Top-15 buckets with counts and % of total.
2. **For each bucket with ≥10 frames**: 4-byte decoded prefix histogram (treat as opcode); pick the dominant prefix; report frequency + variance.
3. **Per dominant-prefix bucket**: byte-position variance table (reuse the cycle-390 + cycle-395 c2s-decoder methodology). Identify low-variance positions = type/subtype codes.
4. **ASCII detection sweep**: for each bucket, check decoded body for stretches of printable ASCII ≥4 chars (looking for chat / NPC text / item names).
5. **Cross-reference len=41 player frames** (the known class): confirm decoded prefix `12 02 60 6d` and decoded shape `12 <id> 86 01 00 00 00 46 11` for entity frames bucket cleanly. This is the **smoke test** for the methodology — if these don't reproduce, the buckets are wrong.
6. **Candidate semantic names** per bucket: entity-spawn (large+ASCII), entity-despawn (small+id), chat (ASCII), inventory-delta (id+count tuples), world-tick (periodic+small), quest (variable, ASCII-tagged).
7. **Output CSV** `analysis/s2c_decoded.csv` with at least 5,000 decoded records, columns: `ord, length, opcode_4b_hex, decoded_hex, decoded_ascii, bucket_name`.
8. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `S2C_INVENTORY_DONE` on the final line.

## Execution flow

**Step 1 — Load + filter:**
```python
import sys
sys.path.insert(0, '/home/sdancer/orchestrator')
import darkdec_decoder as dd
from pathlib import Path
s2c = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin').read_bytes()
payloads = [('S2C', 0, 0, 0.0, '158.101.105.58', 10001, '192.168.1.36', 65261, s2c)]
frames = [f for f in dd.app_frames(payloads) if f['dir']=='S2C']
print(f'S2C frames: {len(frames)}')  # expect 23,570
```

**Step 2 — Length + opcode histogram.**
```python
from collections import Counter
lens = Counter(len(f['raw']) for f in frames)
opcodes = Counter(bytes(f['dec'][:4]) for f in frames if len(f['dec']) >= 4)
```

**Step 3 — Per-bucket byte-position variance** (reuse cycle-390 approach).

**Step 4 — Smoke test:** verify len=41 frames containing prefix `12 02 60 6d` count == 6013 (matches Python decoder's player_track row count).

**Step 5 — ASCII detection** (reuse from c2s-minor-classes briefing).

**Step 6 — Cross-correlate with C2S**: for each S2C bucket, check whether it follows a specific C2S length within ±5 frame ordinals. This hints at request/response pairs.

**Step 7 — Write markdown + CSV + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Use streaming where possible; the full S2C decoded buffer is ~1.5 MB so this is comfortable.
- **NO new disasm. NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time.**
- The cycle-390/395/397 C2S work is the reference framework — replicate the byte-position variance + ASCII-sweep methodology, but on the S2C direction.
- The decoded body is 1 byte SHORTER than the raw body (adjacent-XOR collapse).
- The 4-byte LE length prefix counts the WHOLE frame including the 6-byte header (4B length + 2B channel).

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-s2c-inventory/` (branch `s2c-inventory`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin`
- Decoder (REUSE): `/home/sdancer/orchestrator/darkdec_decoder.py`
- Rust decoder (REFERENCE, may also use): `/home/sdancer/dark-december-rust-decoder/src/lib.rs`
- Cycle-390 c2s-decoder analysis (methodology): `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md`
- Cycle-395 c2s-len45 field analysis (cross-reference): `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
- Cycle-397 c2s-minor-classes (methodology): `/home/sdancer/dark-december-c2s-minor-classes/analysis/c2s_minor_classes_2026-05-15.md`
- success-fact key: `dark_december_s2c_inventory_<n>_classified_<m>_named` (a)
- block-fact keys: `dark_december_s2c_no_length_buckets` (b), `dark_december_s2c_text_decoded` (c)
