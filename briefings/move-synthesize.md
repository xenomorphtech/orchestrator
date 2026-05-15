# move-synthesize — Synthesize FRzMove/FRzStand field layout + trajectory from existing disasms

## Role & workdir
Pure-synthesis worker. Workdir: `/home/sdancer/dark-december-move-decode`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing — prior worker dead at memory cap)
- **sub_goal_key**: `synthesize-from-existing-disasms`

## Why this turn exists — RECOVERY
Prior worker `move-decode` (cycles 342-345) extracted 38 disasm files (Move/Stand Rq+Br, GetTypes, RzPktBase Start/Push/PopRequest) but never wrote the closure markdown. Hit cgroup MemoryHigh=20G (likely via aeon MCP). Your job: **synthesize the field layout + decoder + trajectory from the existing files**. NO new disasm. NO aeon MCP. NO pyelftools.

## Hypothesis
The 4 primary Serialize disasms (FRzMoveRq, FRzMoveBr, FRzStandRq, FRzStandBr) contain enough pattern info to recover field offsets and widths. Pattern (from prior recovery): `add x1, x0, #FIELDOFF; mov w2, #WIDTH; ldr x8, [x19]; ldr x8, [x8, #0x60]; blr x8` per field. Parse these triples sequentially.

## Falsification (3 outcomes)
- (a) **Field layout recovered + ≥1 frame decoded to plausible coords** → SUCCESS. Fact: `dark_december_move_field_layout_decoded_<msgs>_traj_<frames>`.
- (b) **Field layout recovered but coords implausible** (NaN, huge values) → fact `dark_december_move_decoded_implausible`.
- (c) **Disasm parse fails to extract clean pattern** → fact `dark_december_move_disasm_unparsable`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`:
1. **Field layout** for each of 4 target message types: `offset | width | inferred_type | inferred_name`. Total sums must match 37 (Rq) and 33 (Br).
2. **Python decoder** for each, ≤150 lines combined.
3. **Trajectory output** from frames.jsonl — apply XOR + decode for each Rq/Br frame, output (frame_idx, direction, msg_type, decoded_fields) for at least 5 frames.
4. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c). Set the fact via `harness fact-set`.

Print `MOVE_SYNTHESIZE_DONE` on the final line.

## Execution flow

**Step 1 — Read existing disasms (already on disk).**
```bash
ls analysis/*.disasm
wc -l analysis/CL_GS_FRzMoveRq.disasm analysis/GS_CL_FRzMoveBr.disasm analysis/CL_GS_FRzStandRq.disasm analysis/GS_CL_FRzStandBr.disasm
```

**Step 2 — Read the recovered XOR algorithm spec (already on disk).**
```bash
sed -n '1,80p' /home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md
```

**Step 3 — Parse field-write triples per disasm.**
```python
# Walk disasm file line-by-line; identify the pattern:
#   add  x1, x0, #FIELDOFF
#   mov  w2, #WIDTH
#   ldr  x8, [x19]
#   ldr  x8, [x8, #0x60]   ; raw-write vtable slot
#   blr  x8
# Output (FIELDOFF, WIDTH) sequence.
import re
def parse_writes(path):
    lines = open(path).read().splitlines()
    fields = []
    i = 0
    while i < len(lines)-3:
        m = re.search(r'add\s+x1,\s+x0,\s+#(0x[0-9a-f]+|\d+)', lines[i])
        if m:
            for j in range(i+1, min(i+6, len(lines))):
                w = re.search(r'mov\s+w2,\s+#(0x[0-9a-f]+|\d+)', lines[j])
                if w:
                    fields.append((int(m.group(1),0), int(w.group(1),0)))
                    break
        i += 1
    return fields
```

**Step 4 — Build Python decoder per message.**
Use the parsed (offset, width) sequence to define a struct format. Map width 1/2/4/8 → `B/H/I/Q` (LE). Type inference: floats are 4-byte; coordinates are usually 3 floats (x, y, z); int8 fields are often flags/status.

**Step 5 — XOR-decode payload + apply decoder.**
The XOR algorithm: `p0 = c0 ^ k0; pi = ci ^ p(i-1) ^ k[i & 7]`. The state-init key for the first frame: use the partial-validation insight from cycle 338 — known-plaintext on the AppGuard frame gave key bytes; carry forward. Or attempt zero key as fallback.

**Step 6 — Output trajectory + fact-set + DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set <fact_key> "<summary>"
echo MOVE_SYNTHESIZE_DONE
```

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Pure text + small frame data. Trivial.
- **NO aeon MCP, NO pyelftools, NO new disasm, NO new memdump, NO live device.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤30 min wall time.**
- All input files are on disk in this worktree's `analysis/`. Read them. Do NOT re-disassemble.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-move-decode/`
- Inputs (existing on disk):
  - `analysis/CL_GS_FRzMoveRq.disasm` (3.8 KB, 360-byte function)
  - `analysis/GS_CL_FRzMoveBr.disasm` (3.4 KB, 320-byte function)
  - `analysis/CL_GS_FRzStandRq.disasm` (3.8 KB)
  - `analysis/GS_CL_FRzStandBr.disasm` (3.4 KB)
  - `analysis/RzPktBase_*.disasm` (3 additional helpers, for context)
- Frame data: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`
- XOR spec: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Prior partial validation: `/home/sdancer/dark-december-decoder-validate/analysis/decoder_validate_2026-05-15.md`
- success-fact key: `dark_december_move_field_layout_decoded_<msgs>_traj_<frames>` (a)
