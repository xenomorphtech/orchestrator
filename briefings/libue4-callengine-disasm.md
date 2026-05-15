# libue4-callengine-disasm — Recover dispatch inside hive::HiveCppPlugin::callEngine

## Role & workdir
Static-disasm worker. Workdir: `/home/sdancer/dark-december-libue4-callengine-disasm`.

## Current goal / sub-goal
- **goal_key**: `dark_december_hive_jni_dispatch_recovered` (continuing — prior path closed at outcome b)
- **sub_goal_key**: `callengine-internal-dispatch`

## Why this turn exists — prior cycle finding
Prior path `libue4-jni-disasm` (cycle 314) closed at outcome (b). Finding:
- `Java_com_hive_plugin_HivePluginUnreal_jniCallEngine` at `0x6ce4c314b8` is a 424-byte string marshaller — no inline dispatch.
- It calls **one** internal target: `hive::HiveCppPlugin::callEngine(std::string)` at **`0x6ce4c30720`** (size `0x868` = 2152 bytes, resolved via dynsym).
- Adjacent dynstr cluster includes `_Z15GetChatInstancev`, `_Z17GetAuthV4Instancev`, `_Z18HiveCPP_CallNativeNSt6__ndk112basic_string...`, `_Z21GetIAnalyticsInstancev` — these are likely route targets.
- **Critical methodology**: prior rodata-mine VAs were dynstr (string) addresses, NOT function entries. Always resolve via dynsym for true function VAs.

## Hypothesis
The function `hive::HiveCppPlugin::callEngine(std::string)` at `0x6ce4c30720` parses the std::string argument (likely format `"<method_name>:<payload>"` or JSON), then dispatches to one of N internal handlers (auth, chat, matchmaking, analytics, etc.). The dispatch is either:
- A string-compare chain against method-name literals, OR
- A hash → switch / map lookup, OR
- A vtable / function-pointer table indexed by parsed method-id.

Recovering this map gives us the per-method routing — the goal's success criterion.

## Falsification (3 clean outcomes)
- (a) **Dispatch fully recovered**: ≥6 method-names → handler symbol/VA mappings → SUCCESS. Fact: `dark_december_hive_jni_dispatch_mapped_<n>_methods`.
- (b) **Function reads/writes JSON or a serialized protocol blob without per-method routing** (e.g., forwards everything to a single HiveCPP_CallNative): document the forwarding pattern. Fact: `dark_december_hive_callengine_passthrough_to_<target>`.
- (c) **Function is too large/obfuscated to enumerate dispatch** (e.g., function-pointer table not resolvable statically): record what was learned, recommend dynamic trace. Fact: `dark_december_hive_callengine_static_disasm_insufficient`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-callengine-disasm/analysis/callengine_dispatch_2026-05-15.md` documenting:
1. Function bounds: confirm start `0x6ce4c30720`, end, size `0x868`.
2. Full disassembly (capped ≤800 lines in artifact — use `objdump` or `aeon` with narrow window).
3. **Method-name string literals** loaded inside the function (via `adrp+add` pairs) with their rodata content.
4. **Call targets** out of this function (every `bl` instruction's destination, resolved to dynsym symbol if possible).
5. The dispatch table or compare-chain if found: `<method_name>  <handler_va>  <handler_symbol>` rows.
6. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c) options above.

Print `CALLENGINE_DISASM_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Compute file offset, dump function bytes.**
```bash
mkdir -p analysis
SHARD=/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin
# .text shard base VA: 0x6ce4bd4000
# Function VA: 0x6ce4c30720
# File offset: 0x6ce4c30720 - 0x6ce4bd4000 = 0x5c720
FUNC_FILE_OFFSET=$((0x5c720))
FUNC_SIZE=$((0x868))
dd if=$SHARD bs=1 skip=$FUNC_FILE_OFFSET count=$FUNC_SIZE 2>/dev/null > analysis/callengine.bin
ls -lah analysis/callengine.bin   # expect 0x868 = 2152 bytes
```

**Step 2 — Disassemble.**
```bash
aarch64-linux-gnu-objdump -D -b binary -m aarch64 \
  --adjust-vma=0x6ce4c30720 analysis/callengine.bin > analysis/callengine.disasm
wc -l analysis/callengine.disasm
```

**Step 3 — Extract every adrp+add pair to identify rodata loads.**
```bash
grep -B0 -A1 -E 'adrp\s+x[0-9]+' analysis/callengine.disasm | head -80
# For each adrp+add target, resolve to the rodata shard content
```

For each `adrp xN, #imm; add xN, xN, #imm2` pair:
- Target VA = (current_pc & ~0xfff) + adrp_imm + add_imm
- If target ∈ [0x6cdd243000, 0x6cdd243000+0x7990a9c]: look up in rodata shard via `dd ... bs=1 skip=$((target - 0x6cdd243000)) count=64` then `strings`

**Step 4 — Extract every `bl <addr>` call target.**
```bash
grep -oE 'bl\s+0x[0-9a-f]+' analysis/callengine.disasm | sort -u
```
For each unique `bl` target, look up its dynsym entry (if `libUE4_reconstructed.elf` from prior path is reusable, or rebuild a minimal symbol-only sparse ELF).

Useful dynsym lookup commands:
```bash
# The prior path may have produced an ELF; if not, use the rodata mine's jni_symbols.txt.va etc to find symbols whose VA is closest to each bl target
PRIOR_ELF=/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf
nm -D $PRIOR_ELF 2>/dev/null | head -5
# Or scan rodata shard for nearest symbol
```

**Step 5 — Identify dispatch pattern.**
Look for the typical patterns:
- String compares: sequence of `bl strcmp` / `bl strncmp` / `bl __cxx_compare` against multiple literals.
- Hash dispatch: hash computation (CRC32, FNV, etc.) then table lookup.
- std::map lookup: red-black tree traversal.
- std::unordered_map: bucket array + linked-list compares.
- Vtable: `ldr xN, [obj]; ldr xM, [xN, #offset]; blr xM`.

**Step 6 — Build mapping table + fact-set + DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set <fact_key> "<summary>"
echo CALLENGINE_DISASM_DONE
```

## Constraints & gotchas
- **HARD memory budget: 1 GB.** 2.1 KB function — trivial, but stay disciplined.
- **HARD output cap**: ≤800 lines of disasm in artifact (the full 2152-byte function is ~538 instructions).
- **No Frida / no device interaction.** Static disasm only.
- **No call-graph traversal beyond direct `bl` targets.** Resolve targets to symbols but don't recursively disassemble.
- **VA resolution rule** (carried from prior path): Always use dynsym for function entries. Symbol-name strings in rodata are NOT function addresses.
- **One Codex turn budget: ≤1 hour wall time.**
- The prior path's reconstructed-ELF (232 MB at `/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf`) — failed in aeon but may work with `nm -D` / `objdump --dynamic-syms` for symbol lookups. Try, but don't depend on it.
- Honor `[[bulk-enumeration-needs-explicit-memory-budget]]`.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-callengine-disasm/`
- .text shard (read-only): `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (97 MB, base VA 0x6ce4bd4000)
- Rodata shard (read-only): `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin` (122 MB, base VA 0x6cdd243000)
- Prior closure: `/home/sdancer/dark-december-libue4-jni-disasm/analysis/jni_dispatch_2026-05-15.md` (READ THIS FIRST for context)
- Prior reconstructed ELF (may be reusable for symbol lookups): `/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf`
- Target function: `hive::HiveCppPlugin::callEngine(std::string)` at **VA 0x6ce4c30720**, size 0x868
- success-fact key: `dark_december_hive_jni_dispatch_mapped_<n>_methods` (a)
- partial-fact: `dark_december_hive_callengine_passthrough_to_<target>` (b)
- block-fact: `dark_december_hive_callengine_static_disasm_insufficient` (c)
