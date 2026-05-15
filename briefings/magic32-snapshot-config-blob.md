# magic32-snapshot-config-blob — Extract auth-URL config record from snapshot shards

## Role & workdir
You are an offline snapshot-mining Codex worker. Workdir: `/home/sdancer/nmss-emu-magic32-snapshot-config-blob`.

## Current goal / sub-goal
- **goal_key**: `nmss_magic32_fresh_capture_enabled` (currently 0.6/1.0 done-with-caveat — this path aims to push toward 0.75)
- **sub_goal_key**: `H-fc1-config-blob-extract`

## Why this path exists
Cycle 210 K=6 planner audit identified this as the **2h cheapest highest-signal** autonomous attempt on the magic32 fresh-capture goal. Predecessor `magic32-api-client-turn4` (cycle 207-210) cleanly falsified that URL literals appear in the libUnreal.so request chain (outcome b: `magic32_issue_endpoint_data_driven_blocked`); concrete auth URLs were observed in **snapshot shards** populated earlier by some config bootstrap. This turn recovers the **structured key→value record** that holds those URLs.

## Hypothesis
The snapshot shards `78882ef000.bin` and `7898b3b000.bin` contain a structured config record (JSON, protobuf, or std::unordered_map / std::map blob) within ±4 KB of the recovered `apis.netmarble.com/...` URL literals, mapping config-key names (`authUrl`, `authTokenUrl`, `authCallbackUrl`, `authWebViewUrl`, `identityUrl`, `exchangeIdentityUrl`, `exchangeIdentityWebViewUrl`, `deviceWebViewUrl`) to their concrete URL values. Recovering the record format makes the binary's full auth-route inventory resolvable purely offline.

## Falsification (3 clean outcomes)
- (a) **JSON-shaped blob** found: bytes near URL literals contain `"key":"value"` pairs OR a Java HashMap dump (e.g. `LjavaUtilHashMap...`). → SUCCESS, fact `magic32_config_blob_json_recovered_<format>`.
- (b) **Protobuf-shaped or binary record**: structured-but-non-JSON blob (length-prefixed strings, tag-value bytes). → PARTIAL SUCCESS, characterize shape; fact `magic32_config_blob_binary_recovered_<format>`.
- (c) **No adjacent structure** — URLs are isolated cstrings with surrounding bytes that don't look like a record table → FALSIFIED, fact `magic32_config_blob_isolated_cstrings_no_record`.

## Side-task (cheap, parallel within budget)
Grep `/data/app/.../base.apk` (or the local APK extract at `/home/sdancer/tmp/nmss_apk/`) for `*.json` / `*.cfg` / `*.xml` files containing `apis.netmarble.com`. If a hit, capture the full file content + filename and report as "alternative-success" — the baked-in config file is even cheaper to consume than the heap record.

## Success criteria
**Primary**: write `/home/sdancer/nmss-emu-magic32-snapshot-config-blob/analysis/config_blob_extract_2026-05-15.md` documenting:
1. Exact byte offsets of all 8 config-key→URL pairs (if found)
2. The record format (JSON / protobuf / HashMap dump / cstring-table / other)
3. A reproducer script `analysis/extract_config_blob.py` that takes a shard path and dumps the recovered record
4. (If side-task hit) the full content of any baked-in APK config file

**Closing facts** (set the appropriate one):
- (a) `magic32_config_blob_json_recovered_<format>`
- (b) `magic32_config_blob_binary_recovered_<format>`
- (c) `magic32_config_blob_isolated_cstrings_no_record`
- side-task: `magic32_config_blob_apk_baked_in_<filename>` (if found)

Print `CONFIG_BLOB_DONE` on the final line of your reply.

## Substrate / pivots already pinned
- **Primary shards** (from Turn 4 / Task 1 inventory): `78882ef000.bin`, `7898b3b000.bin` — these contain the auth-URL literals.
- **Donor lane**: `78a2b0b000.bin` carries `Host: terms-api.netmarble.com` + JWT artifacts.
- **Other auth-adjacent shards**: `78b733f000.bin`, `78a7210000.bin` (mentioned in Task 1 artifact).
- **Memdump root**: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`
- **Config keys to anchor on** (in this priority order):
  ```
  authUrl                = https://apis.netmarble.com/cpp-auth/
  authAccountUrl         = https://apis.netmarble.com/accounts
  authTokenUrl           = https://apis.netmarble.com/oauth2
  authCallbackUrl        = https://apis.netmarble.com/auth/callback
  authWebViewUrl         = https://members.netmarble.com/auth
  identityUrl            = https://apis.netmarble.com/identity
  exchangeIdentityUrl    = https://apis.netmarble.com/identity
  exchangeIdentityWebViewUrl = https://profile-auth-view.netmarble.com/exchange
  deviceWebViewUrl       = https://profile-auth-view.netmarble.com/secure/device
  ```

## Execution flow — DO NOT EXIT BETWEEN STEPS (atomic single turn)

**Step 1** — Use `strings -a -t x` on both primary shards. Filter for the URL substrings to locate offsets:
```bash
for s in 78882ef000.bin 7898b3b000.bin; do
    strings -a -t x /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/$s | \
      rg -i 'apis\.netmarble\.com|members\.netmarble\.com|profile-auth-view\.netmarble\.com'
done
```
Build a sorted list of (shard, offset, url) triples.

**Step 2** — For each URL hit, dump the **±4096 bytes** around it using `dd` + `xxd`:
```bash
dd if=<shard> bs=1 skip=$((offset - 4096)) count=8192 2>/dev/null | xxd | head -512
```
Look for, in this order:
- JSON braces / quotes (`{`, `}`, `"key":"value"`)
- Java HashMap serialization markers (`LjavaUtilHashMap`, `Entry`)
- length-prefixed string tables (1B or 2B length followed by ASCII)
- C++ `std::pair` / `std::string` SBO layouts (size at +0x10, ptr at +0x18, cap at +0x20 — same shape as the MAGIC32 device_info+0x210 layout we already know)
- protobuf wire-format tag bytes (varints near printable strings)

**Step 3** — If a record shape is identified in any of the windows, write a Python script `extract_config_blob.py` that:
- Takes a shard path
- Locates the record by anchor pattern (e.g., the first `authUrl` cstring offset)
- Walks forward/backward through the record structure
- Emits a `{key: value, ...}` dict as JSON

**Step 4** — Cross-validate: the same record should appear (or have a structurally compatible variant) in BOTH primary shards. If shards diverge, document why.

**Step 5** — Side-task: scan APK for baked-in config:
```bash
ls /home/sdancer/tmp/nmss_apk/extract/ 2>&1  # check what's present
# If assets/ exists:
find /home/sdancer/tmp/nmss_apk/extract -type f \( -name '*.json' -o -name '*.cfg' -o -name '*.xml' \) | \
  xargs -I{} grep -l 'apis.netmarble.com' {} 2>/dev/null
# Else try the live APK:
adb shell 'ls /data/app/*/com.netmarble.thered*/base.apk' 2>/dev/null
# Or extract a fresh copy if needed
```

**Step 6** — Write the artifact + set the appropriate fact via:
```bash
/home/sdancer/orchestrator/harness fact-set <key> "<value>"
```
Print `CONFIG_BLOB_DONE`.

## Constraints & gotchas
- **HARD memory budget per Python step: 1 GB.** Use streaming reads (`dd | xxd | head`), not `open().read()` of full shards. Cycle 209 had a runaway python3 enumerator that ate 22 GB RSS — DO NOT DO THAT.
- **Bounded address ranges**: never `open(shard).read()` a multi-GB blob. Read at most 8 KB at a time via `dd bs=1 skip=X count=8192`.
- **NO live network calls this turn.** Pure offline.
- **NO Frida / device interaction.** Pure offline.
- **One Codex turn budget**: ≤2h wall time, single end-to-end script generation + execution.
- Honor memory rule `[[bulk-enumeration-needs-explicit-memory-budget]]` — declare your read sizes up front, don't let them grow.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-snapshot-config-blob/`
- predecessor artifact (Task 1 + Task 4): `/home/sdancer/nmss-emu-magic32-api-client/analysis/api_client_impl_2026-05-14.md`
- snapshot root: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`
- success-fact keys: `magic32_config_blob_json_recovered_<format>`, `magic32_config_blob_binary_recovered_<format>`
- block-fact key: `magic32_config_blob_isolated_cstrings_no_record`
