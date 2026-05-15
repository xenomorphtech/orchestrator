# magic32-config-fetch-endpoint — Find the bootstrap code path that LOADS NMServiceSettingsSave

## Role & workdir
Offline static-disasm Codex worker. Workdir: `/home/sdancer/nmss-emu-magic32-config-fetch-endpoint`.

## Current goal / sub-goal
- **goal_key**: `nmss_magic32_fresh_capture_enabled` (currently 0.75/1.0)
- **sub_goal_key**: `H-fc2-config-fetch-endpoint`

## Why this turn exists
Sister-path `magic32-snapshot-config-blob` (cycle 211-212) recovered the full `NMService.NMServiceSettingsSave SDKConstants.Constants` UE MapProperty<StrProperty,StrProperty> with 80 entries. Then Turn 5 (cycle 213) and Turn 6 (cycle 214) live-probed every URL in that inventory — **all returned post-issue verification responses or generic 404s**. No URL in the recovered config is the MAGIC32 issuer.

The config blob therefore has to be **populated from somewhere** — that source is either a server-fetched bootstrap or a baked-in asset we missed. The cycle-210 K=6 planner's H-fc2 hypothesis: **find the code in libUnreal.so that WRITES to the config-key strings** (`authUrl`, `exchangeIdentityUrl`, etc.). Those write sites are the bootstrap code. If the bootstrap fetches NMServiceSettingsSave from a server, the FETCH URL is the new probe target — and its response may contain the real issuer URL.

## Hypothesis
libUnreal.so contains an early-startup code path that:
1. Issues an HTTPS request to a hardcoded bootstrap URL
2. Receives a response containing the SDKConstants.Constants key-value pairs (or a serialized save file)
3. Writes those values into the in-memory MapProperty
4. The bootstrap URL is recoverable via xref analysis around the config-key strings AND/OR the SDKConstants/NMServiceSettingsSave class strings.

## Falsification (3 clean outcomes)
- (a) **URL recoverable**: a hardcoded HTTPS URL is found in code paths that reference the config keys → SUCCESS. Fact `magic32_bootstrap_fetch_url_recovered_<url>`.
- (b) **Java-side bootstrap**: config keys are only referenced by getters/empty-check helpers in the native side; the writes happen in Java/smali via SharedPreferences → BLOCKED for offline-native path. Fact `magic32_bootstrap_is_java_side_native_only_reads`.
- (c) **Save-file-only**: the config is loaded from a `.sav` file on disk (UE save serialization) and the save was downloaded at install-time or shipped with the APK → escalate to APK-asset re-grep. Fact `magic32_bootstrap_is_save_file_load`.

## Success criteria
**Primary**: write `/home/sdancer/nmss-emu-magic32-config-fetch-endpoint/analysis/config_fetch_endpoint_2026-05-15.md` documenting:
- 5-10 representative xref sites around the strings `authUrl`, `exchangeIdentityUrl`, `NMServiceSettingsSave`, `SDKConstants`
- For each: whether it's a read (load to register, compare to "" empty), or a write (store from register), or a class-loader site
- The bootstrap-fetch URL, if found, with byte offset + adjacent disassembly
- Verdict matched to (a)/(b)/(c)

**Closing fact**: see list above.

Print `H_FC2_DONE` on the final line.

## Execution flow — DO NOT EXIT BETWEEN STEPS (atomic, single Codex turn)

**Step 1** — Locate the 4-6 anchor strings in libUnreal.so:
```bash
strings -a -t x /home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so | \
  rg -i 'authUrl|exchangeIdentityUrl|authTokenUrl|NMServiceSettingsSave|SDKConstants|NMConstantSave' | head -20
```
Note the byte offsets. Convert to VAs using `readelf -lW` to find the `.rodata` PHDR (VA offset = file offset - PHDR file offset + PHDR vaddr).

**Step 2** — For EACH anchor string offset, find xrefs via bounded `objdump`:
```bash
# Bounded — disassemble ONLY a 128 KB window of .text and grep for adrp+add referencing the string's page
aarch64-linux-gnu-objdump -d --start-address=<text-base+N> --stop-address=<text-base+N+0x20000> \
  /home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so | rg 'adrp.*0x<page>'
```
**Cap memory**: never disassemble the full `.text` section. Operate in 128 KB chunks; abort enumeration after first 10 hits per anchor string.

**Step 3** — For the top 5 xref hits per string, dump ±256 bytes of disasm around each:
```bash
aarch64-linux-gnu-objdump -d --start-address=<hit-256> --stop-address=<hit+512> \
  /home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so
```
For each hit, classify it as:
- **READ**: pattern is `adrp + add → ldr` (load string for compare/getter)
- **WRITE**: pattern is `adrp + add → str` (store value to memory) OR adjacent to another `adrp + add` that loads a URL-shaped string
- **CLASS-LOADER**: pattern is `adrp + add → bl SomeLoader::Load` (UObject deserialize site)

**Step 4** — For any WRITE site, follow the source register backward (≤32 instructions) to find what's being written. If it's another `adrp + add` to an HTTPS URL string, you've found the bootstrap fetch URL.

**Step 5** — For any CLASS-LOADER site, note the function being called. If it's an HTTP fetch (curl wrapper, HttpsRequest, etc.), the function's caller might have the URL.

**Step 6** — Cross-validate by scanning the snapshot shards for HTTPS URLs adjacent to "NMServiceSettingsSave" or "SDKConstants" markers:
```bash
strings -a -t x /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/7898b3b000.bin | \
  rg -B 3 -A 3 'NMServiceSettingsSave|SDKConstants' | head -40
```

**Step 7** — Write the artifact + set the appropriate fact:
```bash
/home/sdancer/orchestrator/harness fact-set <key> "<one-line summary>"
```
Print `H_FC2_DONE`.

## Constraints & gotchas
- **HARD memory budget: 1 GB per step.** NEVER `aarch64-linux-gnu-objdump -d` the whole libUnreal.so — only bounded `--start-address/--stop-address` windows of ≤128 KB. Cycle 209 lost 22 GB RSS to a runaway unbounded enumerator on this same binary — DON'T repeat.
- **HARD enumeration bound: 10 xref hits per anchor string.** Don't try to enumerate all hits.
- **NO live network calls.** Pure offline.
- **NO Frida/device interaction.**
- **NO python-heredoc enumerators** that load the full ELF into Python memory. If you write a python script, declare the read-window size in advance.
- **One Codex turn budget**: ≤3 hours wall time.
- Honor memory rule `[[bulk-enumeration-needs-explicit-memory-budget]]`.
- Honor user memory rule `[[impossibility-caution]]` — this IS the implementation of "the necesary api call" by finding its true endpoint.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-config-fetch-endpoint/`
- libUnreal.so ELF: `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so` (158 MB, aarch64)
- prior config-blob recovery: `/home/sdancer/nmss-emu-magic32-snapshot-config-blob/analysis/config_blob_extract_2026-05-15.md`
- prior callback disasm: `/home/sdancer/nmss-emu-magic32-disasm/analysis/task2_callback_disasm_2026-05-13.md`
- Turn 4 disasm starting points (route-construction chain — already shown to NOT contain the URL): payload-ctor 0x57f7250, queue-helper 0x4af4c10, tail-call 0x57f74e0
- success-fact key: `magic32_bootstrap_fetch_url_recovered_<url>` (a)
- block-fact keys: `magic32_bootstrap_is_java_side_native_only_reads` (b), `magic32_bootstrap_is_save_file_load` (c)
