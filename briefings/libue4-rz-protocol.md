# libue4-rz-protocol — Recover the ingame Rz* wire format (IRzBuffer serialize/deserialize)

## Role & workdir
Static-disasm worker focused on ingame protocol. Workdir: `/home/sdancer/dark-december-libue4-rz-protocol`.

## Current goal / sub-goal
- **goal_key**: `dark_december_ingame_rz_protocol_decoded` (new — USER-PRIORITY)
- **sub_goal_key**: `rz-protocol-buffer-and-message-types`

## Why this turn exists — USER-PRIORITY PIVOT
The user explicitly raised that **ingame protocol is the highest priority** on dark december. Prior paths recovered the Hive **platform** SDK (auth/match/chat/IAP — `callEngine` → 14 class routes). That's NOT the ingame protocol. The ingame protocol is the **`Rz*` family** — already surfaced in the rodata mine but not yet disassembled. The naming convention `CL_GS::FRz...Rq` (Client→GameServer request) + `GS_CL::FRz...` (GameServer→Client response) + paired `Serialize`/`Deserialize` methods on a custom `IRzBuffer` abstraction tells us this is a **typed-message binary protocol over a custom transport** — the actual gameplay packet stream.

## Hypothesis
`IRzBuffer` is a custom write-cursor wire buffer (likely TLV / length-prefixed primitives + endianness handling). Each `FRz*::Serialize(IRzBuffer&) const` and `FRz*::Deserialize(IRzBuffer&)` pair encodes/decodes one message-type by issuing a fixed sequence of `IRzBuffer::append<T>(...)` and corresponding reader calls. Recovering the primitive-write helpers in `IRzBuffer` and one or two complete message-type Serializers gives us the wire encoding rules + at least one fully-decoded packet type.

## Falsification (3 clean outcomes)
- (a) **`IRzBuffer` primitives + ≥3 message-type Serialize/Deserialize pairs fully decomposed** → SUCCESS. Fact: `dark_december_rz_protocol_wire_format_decoded_<n>_msgtypes`. Output includes byte-level wire layout for at least 3 of `FRzStatInfo`, `FRzDamageInfo`, `FRzDamageCalcInfo`, `FRzDamageAttributeCalcInfo`, `FRzAppGuardRq`.
- (b) **`IRzBuffer` primitives recovered but message types use opaque calls or polymorphic dispatch** that can't be statically decomposed → document the primitives, mark messages as opaque. Fact: `dark_december_rz_buffer_primitives_only`.
- (c) **`Serialize`/`Deserialize` pairs delegate to nested templated helpers** beyond static reach (e.g., visitor pattern with runtime-resolved virtual dispatch) → reach the wall, document. Fact: `dark_december_rz_protocol_static_disasm_insufficient`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md` with:
1. **dynsym-resolved VAs** for `IRzBuffer::append<T>(...)` template instantiations and any non-template `IRzBuffer::WriteXxx` methods.
2. **Primitive write functions** — for each `IRzBuffer::append<T>` (`uint8`, `uint16`, `uint32`, `uint64`, `int*`, `float`, `double`, `bool`, `FString`, `TArray<T>`, `TMap<K,V>`): the function VA + a one-line description of byte-layout (endian, length prefix, padding).
3. **At least 3 message-type Serialize/Deserialize pairs** fully decomposed: byte-level field layout, named fields where mangled symbols allow inference.
4. **Concrete wire example** for one message type: hex byte sequence with field-by-field annotation.
5. **Transport hint** — any string literals near these functions referencing TCP port, socket setup, header magic, packet length encoding (this informs `tcpdump` / mitmproxy capture work).
6. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c) above.

Print `RZ_PROTOCOL_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Reuse prior reconstructed ELF symbols if available + resolve target VAs via dynsym.**
```bash
mkdir -p analysis
PRIOR_ELF=/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf
# Or rebuild sparse ELF if needed. Use python-elftools.
python3 << 'PY'
from elftools.elf.elffile import ELFFile
with open('/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf','rb') as f:
    elf = ELFFile(f)
    ds = elf.get_section_by_name('.dynsym')
    if ds:
        for sym in ds.iter_symbols():
            n = sym.name
            if not n: continue
            if ('IRzBuffer' in n or 'FRzStat' in n or 'FRzDamage' in n
                or 'RzPktSystem' in n or 'FRzAppGuard' in n or 'CL_GS' in n or 'GS_CL' in n):
                print(f'0x{sym.entry.st_value:x}  size=0x{sym.entry.st_size:x}  {n}')
PY > analysis/rz_symbols.txt
wc -l analysis/rz_symbols.txt
```

**Step 2 — For each unique function VA in `rz_symbols.txt`, compute file offset in .text shard.**
Base VA of .text shard: `0x6ce4bd4000`. File offset = VA - 0x6ce4bd4000 (if VA ≥ 0x6ce4bd4000). Use ELF entry `st_value` directly when reading via the reconstructed-ELF base 0x6cdd243000.

**Step 3 — Disasm priority targets first (small functions, primitives):**
Target priority order:
1. **`IRzBuffer::append` template instantiations** (typically <0x100 bytes each — these are the byte-layout primitives).
2. **`FRzStatInfo::Serialize`** + `Deserialize` (likely smallest message; great training example).
3. **`FRzDamageInfo::Serialize`** + `Deserialize`.
4. **`FRzDamageAttributeCalcInfo::Serialize`** + `Deserialize` (uses TMap — exercises template machinery).
5. **One CL_GS or GS_CL request/response pair**.

For each: extract bytes via `dd`, disasm via `aarch64-linux-gnu-objdump -D -b binary -m aarch64 --adjust-vma=<va>`, store as `analysis/<symbol>.disasm`.

**Step 4 — Decompose each disasm by hand into field-write sequence.**
Pattern to look for: `ldr/ldrb wN, [x0, #fieldoff]` followed by `bl <primitive_writer>`. The succession of these calls + field offsets gives the wire byte order.

**Step 5 — Build wire-layout table for each message; build one concrete example by reading from the .data shard if a sample instance happens to be there.**

**Step 6 — Synthesize artifact + fact-set + print DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set <fact_key> "<summary>"
echo RZ_PROTOCOL_DONE
```

## Constraints & gotchas
- **HARD memory budget: 2 GB.** Multiple small functions disassembled, but never slurp >100 MB into python.
- **HARD output cap**: ≤1200 lines of disasm in artifact (use carving + per-function files).
- **No Frida / no device interaction.** Pure offline static.
- **VA resolution rule**: Always use dynsym for function entries (carried from prior paths). Rodata-string VAs are NOT function entries.
- **Memory rule `[[bulk-enumeration-needs-explicit-memory-budget]]`** — declared.
- **Memory rule `[[orchestrator-role]]`** — yes, this IS the code-changing dispatch, fine to spawn me.
- **One Codex turn budget: ≤2 hours wall time.**
- The function naming convention `CL_GS` = "Client → GameServer" and `GS_CL` = "GameServer → Client" — this is a strong protocol-direction hint. Use it.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-rz-protocol/`
- .text shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (97 MB, base VA 0x6ce4bd4000)
- Rodata shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin` (122 MB, base VA 0x6cdd243000)
- Data shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ceac63000.bin` (14 MB, base VA 0x6ceac63000) — may contain live message instances
- Prior reconstructed ELF (use for dynsym lookups): `/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf`
- Rodata mine inventory: `/home/sdancer/dark-december-libue4-rodata-mine/analysis/rodata_inventory_2026-05-15.md`
- Rodata mine Rz cluster: `/home/sdancer/dark-december-libue4-rodata-mine/analysis/protocol_rz_surface.txt` + `protocol_rz.txt`
- Prior callEngine closure (for context, NOT this path's target): `/home/sdancer/dark-december-libue4-callengine-disasm/analysis/callengine_dispatch_2026-05-15.md`
- success-fact key: `dark_december_rz_protocol_wire_format_decoded_<n>_msgtypes` (a)
- block-fact keys: `dark_december_rz_buffer_primitives_only` (b), `dark_december_rz_protocol_static_disasm_insufficient` (c)
