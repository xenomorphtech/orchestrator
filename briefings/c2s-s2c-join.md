# c2s-s2c-join — Bind C2S request classes to S2C response classes by stream-order correlation

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-c2s-s2c-join` (worktree of `/home/sdancer/darkdecember/`, branch `c2s-s2c-join`).

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_s2c_request_response_pairs` (new — planner Rank 2 from cycle 404)
- **sub_goal_key**: `pair-cl_gs-rq-with-gs_cl-rp-by-stream-order`

## Why this turn exists
Both directions of the protocol are now individually bucketed:
- **C2S**: 3 dominant classes (len=45 movement / len=10 ack / len=61 composite) + 21 minor-class buckets, 97.4% coverage. Len=45 has 8/8 field positions named. Len=61 is len=45-family with 16-byte insertion at dec[17:33].
- **S2C**: 15 named families covering 89.9% of 35,260 frames; smoke test reproduces 6013 player + 17555 entity updates exactly.

The remaining gap is **cross-direction binding**: which `CL_GS::F*Rq` produces which `GS_CL::F*Rp`? Without this, we have two independent decoders that don't form a single causal protocol. With it, we can name the entire request/response API surface.

## Hypothesis
Pairing C2S frames with S2C frames in **stream-byte-offset order** (using the existing TCP-stream byte offsets as a monotonic clock proxy) reveals fixed C2S-class → S2C-class mappings for ≥5 request/response pairs. Specifically:
- C2S len=10 ack → S2C len=41 broadcast (movement echo / world-state delta)
- C2S len=45 movement → S2C len=41 player_position_update_family (own-position echo)
- C2S len=61 composite → S2C len=39 movement_event_reply_mixed (skill/projectile response)
- C2S len≈37 minor-class actions → S2C len=37 coord_event_short or len=86 world_state_bundle_a (action result)

## Falsification (3 outcomes)
- (a) **≥5 C2S→S2C class pairs have conditional probability P(S2C-class | preceding C2S-class within ±20 frame ordinals) ≥ 0.5**, significantly above the marginal baseline → SUCCESS. Fact: `dark_december_c2s_s2c_pairs_<n>_bound`.
- (b) **No C2S→S2C class pair clears P ≥ 0.30 above marginal baseline** → S2C is server-pushed and not request-correlated; the pairing assumption is wrong. Fact: `dark_december_c2s_s2c_no_correlation`.
- (c) **Mixed outcome: 1-4 pairs bind, rest unbound** → partial success — name the bound pairs, document why the rest stay free. Fact: `dark_december_c2s_s2c_partial_<n>_bound`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-c2s-s2c-join/analysis/c2s_s2c_pairs_2026-05-15.md` with:

1. **Stream-byte timeline reconstruction**: each C2S frame gets a `stream_byte_offset_c2s`, each S2C gets `stream_byte_offset_s2c`. The TCP capture is one direction-per-stream, so use the **frame-ordinal interleaving** based on byte position within each stream. (This is approximate, but better than no clock.)
2. **Conditional probability matrix**: for each C2S length-class × S2C length-class, compute `P(S2C class j appears within ±20 frame ordinals of C2S class i) / P(S2C class j marginal)`. Report a heatmap-like table of the top 40 cells (sorted by lift = conditional/marginal).
3. **Per-pair detail**: for the top 5-10 pairs by lift, show:
   - C2S class signature (length + dominant prefix)
   - S2C class signature
   - lift value
   - 3 stream-order examples showing the pairing
   - candidate Rz typename binding (from cycle-406's 822-symbol catalog at `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`)
4. **Cross-reference with c2s-len45 opaque slots (@13, @21, @29)**: if any S2C field echoes one of these values back, that disambiguates the opaque slot from the C2S side. Specifically: for each c2s-len45 frame, check if its opaque slot values appear in the paired S2C frame's decoded body (any offset, any width). Report hit count for each slot.
5. **Output CSV** `analysis/c2s_s2c_pairs.csv` with at least 1000 paired records: `c2s_ord, c2s_len, c2s_prefix, s2c_ord, s2c_len, s2c_prefix, ordinal_gap, lift`.
6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `C2S_S2C_JOIN_DONE` on the final line.

## Execution flow

**Step 1 — Load both streams + bucketize:**
```python
import sys
sys.path.insert(0, '/home/sdancer/orchestrator')
import darkdec_decoder as dd
from pathlib import Path
from collections import Counter, defaultdict

c2s = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin').read_bytes()
s2c = Path('/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin').read_bytes()

payloads = [
    ('C2S', 0, 0, 0.0, '192.168.1.36', 65261, '158.101.105.58', 10001, c2s),
    ('S2C', 0, 0, 0.0, '158.101.105.58', 10001, '192.168.1.36', 65261, s2c),
]
all_frames = list(dd.app_frames(payloads))
# Critical: dd.app_frames merges directions; each frame has 'dir', 'ord' (within direction), and 'off' (stream byte offset)
```

**Step 2 — Build an interleaved timeline:**
Because the two streams are independent, you cannot perfectly interleave them by time. But you can interleave by **byte fraction**:
```python
c2s_total = len(c2s)
s2c_total = len(s2c)
# Normalize each frame's stream byte offset to [0, 1], then merge-sort.
timeline = []
for f in all_frames:
    total = c2s_total if f['dir'] == 'C2S' else s2c_total
    timeline.append((f['off'] / total, f))
timeline.sort(key=lambda t: t[0])
# Now timeline is the merged "wall-clock proxy" ordering of all 47,987 frames.
```

**Step 3 — Conditional probability scan:**
```python
WINDOW = 20  # ordinals in either direction
# For each (c2s_class, s2c_class) pair, count co-occurrences within window
# Marginal: count of s2c_class in full timeline / N
# Conditional: count of (s2c_class within ±WINDOW of c2s_class) / (count of c2s_class)
# Lift = conditional / marginal
```

**Step 4 — Top-pair detail report.**

**Step 5 — c2s-len45 opaque-slot echo scan**: for each paired (c2s-len45, s2c-len=41) pair, check whether `c2s_dec[13]`, `c2s_dec[21]`, `c2s_dec[29]` (and 4-byte little-endian u32 reads at those offsets) appear *anywhere* in the s2c frame's decoded body. Report hit count.

**Step 6 — Write markdown + CSV + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** 48K frames is small; the join is the work.
- **NO new disasm. NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time.**
- The interleaving is **approximate** (no real timestamps). State this explicitly in the artifact; report the assumption's potential bias (e.g. if S2C is bursty server-push, the byte-fraction interleave will under-correlate request-response pairs).
- Cross-pollination: the 822 RZ typenames are at `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`. Use them to label suggested bindings.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-s2c-join/` (branch `c2s-s2c-join`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- Decoder (REUSE): `/home/sdancer/orchestrator/darkdec_decoder.py`
- C2S decoder analysis: `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md`
- C2S len=45 field map: `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
- C2S len=61 decoded: `/home/sdancer/dark-december-c2s-len61-decode/analysis/c2s_len61_decode_2026-05-15.md`
- C2S minor classes: `/home/sdancer/dark-december-c2s-minor-classes/analysis/c2s_minor_classes_2026-05-15.md`
- S2C inventory: `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
- RZ symbol catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt` (822 typenames)
- RZ wire format: `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md`
- success-fact key: `dark_december_c2s_s2c_pairs_<n>_bound` (a)
- block-fact keys: `dark_december_c2s_s2c_no_correlation` (b), `dark_december_c2s_s2c_partial_<n>_bound` (c)
