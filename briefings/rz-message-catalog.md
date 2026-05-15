# rz-message-catalog — Catalog every CL_GS / GS_CL message type with wire-payload size

## Role & workdir
Bulk-disasm catalog worker. Workdir: `/home/sdancer/dark-december-rz-message-catalog`.

## Current goal / sub-goal
- **goal_key**: `dark_december_rz_message_catalog_complete` (new)
- **sub_goal_key**: `cl-gs-rq-and-gs-cl-rp-wire-size-catalog`

## Why this turn exists
The `decoder-validate` path (cycle 338) found that the static decoder partially validates (first 2 bytes XOR cleanly to `RQAppGuard` packet-id `0x001a`) but the 41/45-byte frames have 33/37-byte message-specific payloads that don't match any of the 5 disassembled `FRz*Info` types (sizes 0/4/10/192). The `rz_symbols.txt` dump has **1433 `Serialize` methods** including CL_GS/GS_CL packet types we haven't characterized (HeartBeat, CharacterStatus, Chatting, Skill, WorldMove, AddRuneSlot, Achievement, …). This path builds the full catalog so the next decode pass can identify what the 33/37-byte frames are.

## Hypothesis
Most CL_GS::FRz*Rq::Serialize and GS_CL::FRz*{Rp,Br,Noti}::Serialize functions are short (typically <0x200 bytes of code) and follow the same IRzBuffer pattern: a sequence of `mov w2, #N; blr [vt+0x60]` for raw scalar writes + vt[0x50]/vt[0x68] for FString/nested. Wire-payload size = sum of all `w2` immediates within the function. Catalog all 1433 with this method.

## Falsification (3 clean outcomes)
- (a) **Catalog completes with ≥1 message type matching 33-byte and ≥1 matching 37-byte payload** → SUCCESS. Fact: `dark_december_rz_message_catalog_<n_types>_size_match_<33and37>`.
- (b) **Catalog completes but no 33/37-byte match** → message types use variable-length (FString/TArray) bodies; static size estimation insufficient. Fact: `dark_december_rz_message_catalog_variable_length_dominant`.
- (c) **Catalog incomplete due to bulk-enumeration memory budget** → document what % was covered. Fact: `dark_december_rz_message_catalog_partial_<pct>`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-rz-message-catalog/analysis/rz_catalog_2026-05-15.md` with:
1. Top-level table: `<message_name> | <direction CL_GS or GS_CL> | <type Rq/Rp/Br/Noti> | <wire_size_bytes> | <function_va> | <function_size_bytes>` for all 1433 entries.
2. **Filtered subtable**: all messages whose wire_size ∈ {0, 2, 33, 37} (matches observed frame payloads minus 2-byte packet-id where applicable).
3. **Top-3 candidates** for 33-byte and 37-byte payloads.
4. The Python script that does the size estimation (≤100 lines), reusable.
5. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c).

Print `RZ_CATALOG_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Read the symbol list.**
```bash
mkdir -p analysis
INPUT=/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_symbols.txt
wc -l $INPUT
# Filter to CL_GS and GS_CL Serialize only
grep -E 'CL_GS.*Serialize|GS_CL.*Serialize' $INPUT > analysis/serialize_symbols.txt
wc -l analysis/serialize_symbols.txt
```

**Step 2 — For each, disassemble + count `w2` immediate writes.**

For each symbol (line format `0x<st_value>  size=0x<func_size>  idx=<n>  <symbol>`):
- file_offset_in_text = st_value - 0x7991000  (per .text shard base relative to image base 0x6cdd243000; .text shard base VA = 0x6ce4bd4000 = image+0x7991000)
- Dump exactly `func_size` bytes from `.text` shard at that offset, disasm via `aarch64-linux-gnu-objdump -D -b binary -m aarch64 --adjust-vma=<va>`
- Extract every `mov w2, #0xNN` immediate as bytes-written-per-call

```python
# Pseudo:
import re, subprocess
SHARD = '/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin'
TEXT_BASE = 0x6ce4bd4000
IMAGE_BASE = 0x6cdd243000

def count_writes(st_value, func_size):
    file_off = st_value - 0x7991000  # st_value is image-relative; .text segment starts at +0x7991000
    if file_off < 0 or file_off + func_size > <text_shard_size>: return None
    # Read function bytes
    with open(SHARD, 'rb') as f:
        f.seek(file_off)
        code = f.read(func_size)
    # Disassemble in-process to avoid spawning objdump 1433 times
    # Use capstone (if available) OR a single batched objdump call
    ...
    # Sum the w2 immediates
    return sum_of_immediates
```

**IMPORTANT**: do NOT call `objdump` 1433 times (too slow). Either:
- Use `capstone` python module if available (`pip show capstone` to check).
- OR write a single big disasm of the .text shard once, parse line-by-line, attribute each `w2,#N` to the enclosing function by address.

**Step 3 — Build size table.**
```python
# Bucket by wire_size; flag the 33-byte and 37-byte rows specifically
```

**Step 4 — Filter to matches.** Look up which message types have wire_size in {0, 2, 33, 37} (the 33 and 37 because frame_payload = packet_id(2) + msg_body, and observed payloads are 2/35/39 → msg_body = 0/33/37).

**Step 5 — Cross-check known.** Earlier rodata-mine found:
- `_ZN8FRzStatInfo9SerializeER9IRzBuffer` total 10 bytes (2-byte stat + 8-byte value). Should appear as wire_size=10.
- `FRzDamageInfo::Serialize` ~ 19 KB of disasm code with many writes.
Confirm those match. If not, the size-counter is buggy.

**Step 6 — Write artifact + fact-set + DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set <fact_key> "<summary>"
echo RZ_CATALOG_DONE
```

## Constraints & gotchas
- **HARD memory budget: 1 GB.** 1433 functions × 256 bytes avg = ~366 KB total bytes to disasm. Keep in memory, don't slurp the whole .text shard (97 MB).
- **HARD output cap**: ≤2000 catalog rows in artifact (group by direction; sort by size).
- **NO pyelftools full-load.** The symbol dump is already a plain text file — use grep/awk/python on text. **No reconstructed ELF load** (this is what blew memory in prior cycles).
- **NO MCP aeon calls.**
- **NO new memdumps.**
- **One Codex turn budget: ≤45 minutes wall time.**
- Capstone python module is fastest if available; else use `aarch64-linux-gnu-objdump` once on a concatenated input.
- Honor `[[bulk-enumeration-needs-explicit-memory-budget]]` — DECLARED 1 GB; cycle 332 hit 21 GB by ignoring this.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-rz-message-catalog/`
- Symbol input: `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_symbols.txt` (652 KB, 7642 Rz symbols, 1433 Serialize)
- .text shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (97 MB)
- .text shard base VA: `0x6ce4bd4000`; image base: `0x6cdd243000`; .text seg file-offset within image = `0x7991000`
- Prior partial validation: `/home/sdancer/dark-december-decoder-validate/analysis/decoder_validate_2026-05-15.md`
- Recovery target: 33-byte and 37-byte message body match for the observed 41/45-byte frames in `frames.jsonl`
- success-fact key: `dark_december_rz_message_catalog_<n>_size_match_<sizes>` (a)
- block-fact keys: `dark_december_rz_message_catalog_variable_length_dominant` (b), `dark_december_rz_message_catalog_partial_<pct>` (c)
