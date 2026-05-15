# wire-state-source — Find what writes to [x0+0xb8] (cipher state) before each XOR call

## Role & workdir
Static disasm + xref worker. Workdir: `/home/sdancer/dark-december-wire-state-source`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `locate-per-packet-state-source`

## Why this turn exists
Cycle 361's `xor-recurrence-verify` did the definitive disasm of 0x6ce57c1904. **Verdict**: cycle-335 spec is correct exactly. Function is a chained XOR with state at `[x0+0xb8]` and post-loop `state_u64 += payload_len`.

**Critical contradiction**: 9 server-to-client c0d2 broadcast frames (spanning packet_frame_number 35..63) have IDENTICAL ciphertext bytes 0..9 — requiring identical state at time-of-XOR. Under the verified `+= payload_len` advance, state SHOULD differ by 35×N bytes between frames. Conclusion: **something writes state at [x0+0xb8] BEFORE each XOR call**, overriding the monotonic advance.

Additionally observed: all 11 client frames have `body[2] == body[3]` (e.g. `e17f 1b1b`, `0896 f2f2`, `33ad c9c9`) — probability ≈ 10^-26 at random → structural property of the plaintext or cipher.

## Hypothesis
There's a CALLER of 0x6ce57c1904 (or sibling code path) that initializes/reloads the 8-byte state at `[x0+0xb8]` from a per-packet source: maybe the frame's own length/header bytes, maybe a per-channel handshake value, maybe a connection-level counter that doesn't monotonically increment. Finding what writes there reveals the missing state-init mechanism.

## Falsification (3 outcomes)
- (a) **Caller writes state[0..7] from a per-packet source identifiable in our shards** (e.g. state = `LE64(channel_seq, packet_kind, ...)` derived from frame header) → SUCCESS. Decode all 22 long frames under the recovered state model. Fact: `dark_december_wire_xor_state_source_<short_hash>_decoded_<n>`.
- (b) **Caller exists but state is set from runtime-only data not in our shards** (e.g. handshake-negotiated key from server) → escalate: need live-capture of state value via kernel uprobe. Fact: `dark_december_wire_xor_state_runtime_only`.
- (c) **No clean caller writes [x0+0xb8] besides 0x6ce57c1904 itself** → 0x6ce57c1904 is genuinely NOT the wire cipher (cycle-361's outcome b confirmed) and there's a different cipher path. Fact: `dark_december_wire_xor_wrong_function_confirmed_no_state_caller`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-wire-state-source/analysis/state_source_2026-05-15.md` with:
1. **Grep for `str * [x?, #0xb8]` or `str * [obj, #184]` patterns in the .text shard.** List every callsite that writes 8 bytes at offset +0xb8 of any object.
2. **For each candidate writer**, disasm a 40-instruction context window and identify:
   - The source register/expression for the stored value
   - When this code runs (called once at connection-init? per packet? per direction?)
3. **Identify the call chain that ends at 0x6ce57c1904**. Look for callers via `bl 0x6ce57c1904` or `br x?` to that address. There's a `br x3` at 0x6ce57c19b0 — that's an OUTBOUND call (to the packet handler) — find what's INBOUND.
4. **Sibling cipher check**: search `.text` shard for OTHER functions exhibiting the chained-XOR loop pattern (ldrb + ldrb + eor + eor + strb + b.ne). The cycle-361 worker found only one inside `xor.disasm`'s 43 KB range, but the full .text shard is 97 MB — there may be others.
5. **Header-derived state test**: for the 9 c0d2 frames, compute candidate state from the frame header bytes `[length:4B little-endian | gate:1 | channel:1]` = `290000000100` for server frames. If state_u64 = `0x0000010000000029` (8-byte interpretation of frame header), test whether cycle-335 decoder with this state produces plausible plaintext.
6. Verdict matched to (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `WIRE_STATE_SOURCE_DONE` on the final line.

## Execution flow

**Step 1 — Find all writers to offset +0xb8 in the .text shard (97 MB):**
```bash
SHARD=/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin
# Use capstone or single-call objdump — DO NOT spawn per-function disasm
# Look for instruction encodings: STR (immediate) with offset 0xb8 = #184
# Pattern: 0xf9 0x5c (str x?, [x?, #0xb8])
# OR linear scan with capstone, filter by mnemonic+offset
```

Use capstone Python module (faster than objdump):
```python
import capstone
md = capstone.Cs(capstone.CS_ARCH_ARM64, capstone.CS_MODE_LITTLE_ENDIAN)
md.detail = True
SHARD_BASE = 0x6ce4bd4000
data = open(SHARD, 'rb').read()
hits = []
for insn in md.disasm(data, SHARD_BASE):
    if insn.mnemonic in ('str','stp') and '0xb8' in insn.op_str:
        hits.append((hex(insn.address), insn.mnemonic, insn.op_str))
    if len(hits) > 200: break
```

If capstone isn't available, fall back to a streaming objdump but bounded to ≤200 MB output:
```bash
aarch64-linux-gnu-objdump -D -b binary -m aarch64 --adjust-vma=0x6ce4bd4000 $SHARD 2>/dev/null | grep -n -E 'str\s+[xw][0-9]+,\s*\[[^]]+#0xb8' | head -200
```

**Step 2 — Find inbound calls to 0x6ce57c1904:**
```bash
# Look for `bl 0x6ce57c1904` or constant 0x6ce57c1904 in the .text
grep -E '0x?6ce57c1904' analysis/disasm_index.txt  # if we made one
# OR: scan for bytes `04 19 7c ce 6c 00 00 00` near `bl`/branch instructions
```

**Step 3 — For each unique writer of [x?+0xb8]**, capture the source. Write a table:
| writer VA | source register | source meaning | calling context |

**Step 4 — Header-derived state test:**
```python
import json, struct
frames = [json.loads(l) for l in open('/home/sdancer/dark-december-body-decode/analysis/frames.jsonl') if l.strip()]
server_35 = [f for f in frames if f['direction']=='server_to_client' and f['body_len']==35]
# Header bytes 0..7: length (4B LE) + gate (1) + channel (1) + ??(2)
# For each frame, take first 8 bytes of frame_hex as state_u64 candidate
# Run cycle-335 decoder
def decode(c_bytes, state_u64):
    key = state_u64.to_bytes(8, 'little')
    out = bytearray(len(c_bytes))
    out[0] = c_bytes[0] ^ key[0]
    for i in range(1, len(c_bytes)):
        out[i] = c_bytes[i] ^ out[i-1] ^ key[i & 7]
    return bytes(out)
# Try multiple state candidates: header bytes, header XOR'd, channel-as-state, etc.
```

**Step 5 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 1 GB.** Capstone on a 97 MB shard is fast and small (~150 MB).
- **NO pyelftools full-load.** Use capstone or raw byte scan.
- **NO Frida. NO live device. NO MCP aeon.**
- **NO new memdump.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤30 min wall time.**
- **Honor `[[bulk-enumeration-needs-explicit-memory-budget]]`** — declared 1 GB cap, bounded scan.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-wire-state-source/`
- **.text shard** (97 MB): `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (base VA `0x6ce4bd4000`, image base `0x6cdd243000`)
- Verified disasm of 0x6ce57c1904: `/home/sdancer/dark-december-xor-recurrence-verify/analysis/recurrence_verify_2026-05-15.md`
- Frame corpus: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`
- Move/Stand layout: `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`
- Prior dynsym dump (grep-only): `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_symbols.txt` (7642 Rz symbols)
- success-fact key: `dark_december_wire_xor_state_source_<sha>_decoded_<n>` (a)
- block-fact keys: `dark_december_wire_xor_state_runtime_only` (b), `dark_december_wire_xor_wrong_function_confirmed_no_state_caller` (c)
