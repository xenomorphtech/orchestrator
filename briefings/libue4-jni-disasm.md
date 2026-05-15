# libue4-jni-disasm — Recover the HivePluginUnreal jniCallEngine dispatch table

## Role & workdir
Static-disasm worker. Workdir: `/home/sdancer/dark-december-libue4-jni-disasm`.

## Current goal / sub-goal
- **goal_key**: `dark_december_hive_jni_dispatch_recovered` (new)
- **sub_goal_key**: `libue4-jni-disasm-bridge-recovery`

## Why this turn exists
The libUE4 rodata mine (cycle 311) recovered the symbol `Java_com_hive_plugin_HivePluginUnreal_jniCallEngine` at VA `0x6cde7b48a1` inside the decrypted .text shard. The rodata cluster around it includes:
- `FHiveAuthV4::GetHiveTalkPlusLoginToken` (login token retrieval)
- `FHiveMatchMaking::RequestMatchMaking`
- `FHiveChannelMessage::FHiveChannelMessage` (chat/channel constructor)
- AppGuard onS2AuthTryCallback (anticheat auth)

`jniCallEngine` is the **Java↔native multiplex point** for the Hive SDK. Recovering its dispatch (switch/case OR vtable lookup) tells us **what method-IDs route to which native handlers** — the exact map needed to understand the auth/protocol flow without disassembling all 97 MB of .text.

## Hypothesis
The function `Java_com_hive_plugin_HivePluginUnreal_jniCallEngine` at VA `0x6cde7b48a1` (file offset `0x07b18a1` inside `memdump/6ce4bd4000.bin`, after subtracting .text base `0x6ce4bd4000`) reads a method-id argument from JNI and dispatches via switch/table to a fixed set of handlers — typically one per Hive SDK call (login, token, send-message, request-matchmaking, etc.).

## Falsification (3 clean outcomes)
- (a) **Dispatch recovered**: structured table mapping ≥6 method-IDs to handler symbols/addresses → SUCCESS. Fact: `dark_december_hive_jni_dispatch_mapped_<n>_methods`.
- (b) **Function disassembles but no obvious dispatch** (e.g. just one big block calling into a single helper) → partial. Document call target. Fact: `dark_december_hive_jni_dispatch_single_callee`.
- (c) **Disasm fails or VA points to wrong content** → falsified anchor mapping. Fact: `dark_december_hive_jni_disasm_failed`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-jni-disasm/analysis/jni_dispatch_2026-05-15.md` documenting:
1. The function bounds (start VA, end VA, size in bytes).
2. The disassembly of the entry prologue + dispatch (switch table or function-pointer table or vtable lookup).
3. The method-id → handler mapping table: `<method_id>  <handler_va>  <handler_symbol_if_known>` for each branch arm.
4. Cross-references: which rodata strings are loaded inside this function (URLs, error messages, JNI signatures).
5. Verdict matched to (a)/(b)/(c).

**Closing fact**: `dark_december_hive_jni_dispatch_mapped_<n>_methods` (a).

Print `JNI_DISASM_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Compute file offset from VA.**
```bash
# VA of function: 0x6cde7b48a1
# Base VA of .text shard: 0x6ce4bd4000 (from rodata mine artifact)
# File offset = 0x6cde7b48a1 - 0x6ce4bd4000 = 0x91e08a1
python3 -c "print(hex(0x6cde7b48a1 - 0x6ce4bd4000))"
```

**Step 2 — Disasm a generous window (16 KB) around entry; locate function bounds.**
```bash
SHARD=/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin
mkdir -p analysis
# Dump 16 KB from file offset 0x91e08a1
dd if=$SHARD bs=1 skip=$((0x91e08a1)) count=16384 2>/dev/null > analysis/jni_entry_16k.bin
# Disasm with binbase aligned to file-offset within the shard
aarch64-linux-gnu-objdump -D -b binary -m aarch64 \
  --adjust-vma=0x6cde7b48a1 analysis/jni_entry_16k.bin > analysis/jni_entry_disasm.txt
wc -l analysis/jni_entry_disasm.txt
```

**Step 3 — Locate function bounds.**
Look for the prologue (`stp x29, x30, [sp,#-NN]!` or `sub sp, sp, #NN`) at the very start, and either RET or unconditional branch to a known epilogue/PLT after the dispatch. Cap function size at 8 KB; if larger, sample first 8 KB and document continuation.

**Step 4 — Identify the dispatch pattern.**
Common AArch64 dispatch patterns:
- **Direct switch**: `cmp w0, #N; b.eq .label_N` chain → simple ID compares.
- **Indirect via table**: `adrp x1, .Lrodata; add x1, x1, #imm; ldr x2, [x1, w0, lsl #3]; br x2` → indirect jump table.
- **Vtable**: `ldr x1, [x0]; ldr x2, [x1, #offset]; blr x2` → C++ vtable lookup.
- **Hash dispatch**: a series of hash computations before branching.

**Step 5 — For each branch arm, resolve handler symbol if possible.**
```bash
# For each handler VA seen in the dispatch, find nearest preceding symbol from the rodata inventory
RODATA_VA_FILE=/home/sdancer/dark-december-libue4-rodata-mine/analysis/all_strings.txt
# Use jni_symbols.txt.va etc. to look up nearby names
```

**Step 6 — Sanity check: cross-reference rodata loads.**
List every `adrp + add` pair in the disasm and resolve the target rodata string. Confirms function actually does what we think.

**Step 7 — Write artifact + fact-set + print DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set dark_december_hive_jni_dispatch_mapped_<N>_methods "<summary>"
echo JNI_DISASM_DONE
```

## Constraints & gotchas
- **HARD memory budget: 1 GB.** Single 16 KB function window; do not slurp the 97 MB .text shard.
- **HARD output cap**: ≤500 lines of disasm in artifact (use head/grep to summarize the rest).
- **No Frida / no device interaction.** Pure offline static disasm.
- **Don't blow up to full call graph** — that's a future path. This path is JUST the dispatch table.
- **VA base check**: the rodata mine recorded base VA `0x6cdd243000` for the rodata shard (122 MB at offset 0x198000 in the APK), and `0x6ce4bd4000` for the .text shard (101 MB at offset 0x7b29000). Function VAs from the rodata inventory are absolute. To find them in the .text shard, subtract base `0x6ce4bd4000`.
- **One Codex turn budget: ≤1 hour wall time.**
- Honor memory rule `[[bulk-enumeration-needs-explicit-memory-budget]]`.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-jni-disasm/`
- .text shard (read-only): `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (97 MB, base VA 0x6ce4bd4000)
- Rodata shard for cross-ref: `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin` (122 MB, base VA 0x6cdd243000)
- Prior closure: `/home/sdancer/dark-december-libue4-rodata-mine/analysis/rodata_inventory_2026-05-15.md` (11 disasm anchors)
- Target VA: `0x6cde7b48a1` symbol `Java_com_hive_plugin_HivePluginUnreal_jniCallEngine`
- success-fact key: `dark_december_hive_jni_dispatch_mapped_<n>_methods` (a)
- block-fact keys: `dark_december_hive_jni_dispatch_single_callee` (b), `dark_december_hive_jni_disasm_failed` (c)
