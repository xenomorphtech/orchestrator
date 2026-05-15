# move-decode — Disassemble FRzMove/FRzStand Serialize + decode player trajectory from capture

## Role & workdir
Targeted disasm + decode worker. Workdir: `/home/sdancer/dark-december-move-decode`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (new)
- **sub_goal_key**: `frz-move-rq-br-field-layout-and-trajectory`

## Why this turn exists
Catalog path (cycle 341) identified the 47-frame capture corpus as player-movement gameplay (33-byte server broadcasts = `FRzMoveBr`/`FRzStandBr`/`FRzForceMoveBr`, 37-byte client requests = `FRzMoveRq`/`FRzStandRq`). With:
- XOR algorithm (cycle 335)
- 6-byte packet header (cycle 330)
- IRzBuffer per-field wire format (cycle 324)
- **THIS PATH**: exact field layout of FRzMove/FRzStand messages from disasm

…we can decode the captured trajectory to (x, y, z, heading, …) per packet. This satisfies the prior open goal `dark_december_minimap_decode` (semantic decode to entity-position level).

## Hypothesis
`FRzMoveRq::Serialize` (37 bytes payload) emits a fixed sequence of integer/float fields covering: player ID, current position (x, y, z as float32 — UE4 standard), target/destination position, heading angle, optional speed. Disassembling the function and tallying field offsets + sizes yields the exact byte-layout.

## Falsification (3 clean outcomes)
- (a) **Field layout recovered for ≥2 message types + 1 frame decoded to plausible coords** → SUCCESS. Fact: `dark_december_move_field_layout_decoded_<msg_count>_traj_<frame_count>`.
- (b) **Field layout recovered but decoded coords don't form sane trajectory** (e.g. huge values, NaN) → wire-format gap or XOR state-init issue. Fact: `dark_december_move_decoded_implausible`.
- (c) **Disasm can't be cleanly parsed** (variable-length or unrecognized pattern) → fallback. Fact: `dark_december_move_disasm_unparsable`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md` with:
1. **Field layout** for `FRzMoveRq` (37 B), `FRzMoveBr` (33 B), `FRzStandRq`, `FRzStandBr` from disasm. Format: `offset | size | inferred_type | inferred_name`.
2. **Python decoder** for each message type (≤150 lines combined).
3. **Decoded trajectory** from the capture corpus: list of (frame_idx, direction, msg_type, x, y, z, heading) tuples for at least 5 frames.
4. Sanity check: do consecutive Move frames show smooth position increments? Plot-ready CSV.
5. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c).

Print `MOVE_DECODE_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Disassemble 4 small target functions.**
Symbol VAs (st_value image-relative; subtract `0x7991000` for file offset in .text shard):
- `_ZNK5CL_GS9FRzMoveRq9SerializeER9IRzBuffer` @ `0x8b66c64` size `0x168` (360 bytes)
- `_ZNK5GS_CL9FRzMoveBr9SerializeER9IRzBuffer` @ `0x8b66f0c` size `0x140` (320 bytes)
- `_ZNK5CL_GS10FRzStandRq9SerializeER9IRzBuffer` @ `0x8b66754` size `0x168`
- `_ZNK5GS_CL10FRzStandBr9SerializeER9IRzBuffer` @ `0x8b669fc` size `0x140`

For each:
```bash
SHARD=/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin
TEXT_BASE_OFFSET=0x7991000   # file offset of .text within image (image base 0x6cdd243000)
# For st_value 0x8b66c64: file_offset = st_value - 0x7991000 = 0x11d5c64
# Then dump and disasm with --adjust-vma at live VA = 0x6cdd243000 + st_value
```

Actually the cleanest approach:
- The .text shard's first byte corresponds to live VA `0x6ce4bd4000` (the shard base from memdump artifact).
- st_value `0x8b66c64` is image-relative; live VA = `0x6cdd243000 + 0x8b66c64 = 0x6ce5da9c64`.
- File offset in shard = live VA - shard base = `0x6ce5da9c64 - 0x6ce4bd4000 = 0x11d5c64`.

```bash
mkdir -p analysis
for sym in 'CL_GS_FRzMoveRq:0x8b66c64:0x168' \
           'GS_CL_FRzMoveBr:0x8b66f0c:0x140' \
           'CL_GS_FRzStandRq:0x8b66754:0x168' \
           'GS_CL_FRzStandBr:0x8b669fc:0x140'; do
  name=$(echo $sym | cut -d: -f1)
  st=$(echo $sym | cut -d: -f2)
  sz=$(echo $sym | cut -d: -f3)
  live_va=$((0x6cdd243000 + $st))
  file_off=$(($live_va - 0x6ce4bd4000))
  dd if=$SHARD bs=1 skip=$file_off count=$(($sz)) 2>/dev/null > analysis/${name}.bin
  aarch64-linux-gnu-objdump -D -b binary -m aarch64 --adjust-vma=$live_va analysis/${name}.bin > analysis/${name}.disasm
done
ls -lah analysis/
```

**Step 2 — Decode each disasm into a field-write sequence.**
Pattern (from prior cycles): repeated
```
add x1, x0, #FIELDOFF
mov w2, #N            ; N bytes
ldr x8, [x19]         ; vtable
ldr x8, [x8, #0x60]   ; raw write
blr x8
```
For each call: record (field_offset, byte_width). Infer type from `mov w2`:
- w2=1 → int8/bool
- w2=2 → int16
- w2=4 → int32 OR float (look for `fmov` nearby = float)
- w2=8 → int64 OR double

Also: `ldr x8, [x8, #0x50]` = FString write; `ldr x8, [x8, #0x68]` = nested serializer.

**Step 3 — Validate against known 37-byte / 33-byte body sizes.** Sum the widths; should equal 37 (Rq) or 33 (Br).

**Step 4 — Build Python decoder per message type.** Define each as a `struct.unpack` format string and named-tuple.

**Step 5 — Apply decoder to frames.jsonl** (client→server 37-byte = MoveRq/StandRq; server→client 33-byte = MoveBr/StandBr) after XOR-decoding the payload using the algorithm from cycle 335. The first 2 bytes of decoded payload are the packet-id; remaining is the message body.

**Step 6 — Output trajectory CSV** + write artifact + fact-set + DONE.

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Pure Python decode + 4 tiny disasms. Trivial.
- **HARD output cap**: ≤500 lines disasm in artifact, ≤200 lines pseudocode.
- **NO pyelftools full-load.** All symbol VAs already known from this briefing.
- **NO MCP aeon calls.**
- **NO new memdumps.**
- **VA arithmetic**: live VA = image_base (0x6cdd243000) + st_value; file_offset = live_va - .text_shard_base (0x6ce4bd4000).
- **One Codex turn budget: ≤30 minutes wall time.**
- Honor `[[bulk-enumeration-needs-explicit-memory-budget]]`.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-move-decode/`
- .text shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin`
- Frame data: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` (47 frames)
- XOR algorithm: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Header layout: `/home/sdancer/dark-december-libue4-pktsystem-disasm/analysis/pktsystem_framing_2026-05-15.md`
- IRzBuffer vtable model: `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md`
- Catalog: `/home/sdancer/dark-december-rz-message-catalog/analysis/rz_catalog_2026-05-15.md`
- success-fact key: `dark_december_move_field_layout_decoded_<msg>_traj_<frames>` (a)
- block-fact keys: `dark_december_move_decoded_implausible` (b), `dark_december_move_disasm_unparsable` (c)
