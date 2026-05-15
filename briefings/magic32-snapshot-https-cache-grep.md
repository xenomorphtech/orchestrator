# magic32-snapshot-https-cache-grep — H-fc7: re-mine snapshot shards for gmc2 response body

## Role & workdir
Offline snapshot-mining Codex worker. Workdir: `/home/sdancer/nmss-emu-magic32-snapshot-https-cache-grep`.

## Current goal / sub-goal
- **goal_key**: `nmss_magic32_fresh_capture_enabled` (currently 0.80/1.0 CLOSED done-with-caveat-advanced — this path can reopen toward 0.90)
- **sub_goal_key**: `H-fc7-snapshot-https-cache-grep`

## Why this turn exists — NOVEL SUBSTRATE
Cycle-222 K=6 planner added this path after 5-in-a-row falsifications of route/host-probing. The previous paths attacked the LIVE backend — this one attacks the **SNAPSHOT HEAP** that already contains the artifact of a SUCCESSFUL gmc2 call.

The 80-entry `NMServiceSettingsSave.SDKConstants.Constants` MapProperty was recovered cycle 212 from snapshot shard `7898b3b000.bin` — a serialized form of the constants map. But that map was **populated from a server response**. The response itself — JSON or protobuf body, including the actual URL with suffix that thered hit and the HTTP response framing — may still be in heap on a different shard.

If we find that response body in the snapshot, we have:
1. The **actual successful URL** with whatever suffix was appended (the cycle-219 "wrong path" hint suggested a suffix exists)
2. The **response shape** (headers + body) to reproduce
3. Any **session token** or **signed nonce** that gated the call

## Hypothesis
The HTTPS response body that delivered the SDKConstants map is preserved in one or more of the 2966 snapshot shards at `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`. Distinct from H-fc1 (which found the destination structure) and from `magic32-snapshot-cached-https-mine` backlog (which targeted userKey reuse). Anchor: the URL prefix `apis.netmarble.com/gmc2/v4/` is now KNOWN — grep for it across all shards and look at adjacent bytes for JSON/protobuf response structure.

## Falsification (3 clean outcomes)
- (a) **gmc2 URL found in heap with adjacent JSON/protobuf body**: SUCCESS. Fact `magic32_snapshot_gmc2_response_recovered_<shard>_<offset>`. Metric 0.80 → 0.90.
- (b) **gmc2 URL appears only in rodata copies** (the literal bytes from libUnreal.so itself mapped into heap): no live response captured. Fact `magic32_snapshot_gmc2_only_in_rodata`.
- (c) **No `apis.netmarble.com/gmc2/v4/` hits anywhere in the 2966 shards** — the response was never on heap, fully deallocated. Fact `magic32_snapshot_gmc2_no_response_in_heap`.

## Success criteria
**Primary**: write `/home/sdancer/nmss-emu-magic32-snapshot-https-cache-grep/analysis/snapshot_https_cache_grep_2026-05-15.md` documenting:
1. Shard-by-shard hit count for `apis.netmarble.com/gmc2/v4/`
2. For each hit cluster: ±2 KB hex dump showing JSON/protobuf structure adjacent OR rodata-mapping confirmation
3. If a response body is recovered: the actual URL with suffix + the response shape
4. Verdict matched to (a)/(b)/(c)

**Closing fact**: see list above.

Print `H_FC7_DONE` on the final line.

## Execution flow — DO NOT EXIT BETWEEN STEPS (atomic, single Codex turn)

**Step 1** — Inventory shards and bulk-grep for the anchor:
```bash
ls /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/*.bin | wc -l   # expect ~2966
grep -rlE 'apis\.netmarble\.com/gmc2/v4/' /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/ 2>/dev/null
```
**MEMORY BOUND**: use `grep -rl` (filename-only) NOT `grep -r` (content) on first pass. That's O(file count) memory, not O(total bytes).

**Step 2** — For each hit shard, find exact offsets:
```bash
grep -boE 'apis\.netmarble\.com/gmc2/v4/[A-Za-z0-9_/.-]*' /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/<shard>.bin
```
The `-b` flag prints byte offsets; `-o` only the matches. Bounded output.

**Step 3** — For each (shard, offset) hit, dump ±2 KB:
```bash
dd if=<shard> bs=1 skip=$((offset - 2048)) count=4096 2>/dev/null | xxd | head -256
```
NEVER `open().read()` the multi-MB shards entirely. Use `dd` for windowed reads.

**Step 4** — Classify each window:
- **JSON-shaped**: contains `{` `}` `"` `:` near the URL → outcome (a). Look for `"version":`, `"data":`, `"constants":`, `"authUrl":` etc.
- **Protobuf-shaped**: contains length-prefixed cstring patterns or tag-value bytes adjacent → outcome (a).
- **rodata-mapping**: surrounded by other libUnreal.so constants (look for adjacent `dev-apis.netmarble.com`, `alpha-apis.netmarble.com` strings — those are the literals from ctor `0x4b458b0`) → outcome (b).
- **Isolated cstring with NUL padding**: no structure → outcome (b) or (c).

**Step 5** — If outcome (a), extract the response body. Look for HTTP response markers (`HTTP/1.1 200 OK`, `Content-Type:`, `Content-Length:`) before the body. Note the suffix part of the URL (everything after `gmc2/v4/`).

**Step 6** — Write the artifact + set the appropriate fact:
```bash
/home/sdancer/orchestrator/harness fact-set <key> "<one-line summary; if outcome (a), include the recovered suffix>"
```
Print `H_FC7_DONE`.

## Constraints & gotchas
- **HARD memory budget per step: 1 GB.** No bulk `open().read()`. Use `grep -l`, `dd bs=1 skip=X count=4096`, streaming pipelines.
- **HARD enumeration cap: 50 shard hits.** If more than 50 shards match, sample the first 50 by alphabetical order and document the remainder count.
- **HARD per-hit cap: top 20 byte-offsets per shard.** Don't try to dump every match.
- **NO live network calls.** Pure offline.
- **NO Frida / device interaction.**
- **One Codex turn budget**: ≤3 hours wall time.
- Honor memory rule `[[bulk-enumeration-needs-explicit-memory-budget]]`. Cycle-209's 22-GB python heredoc is the negative example — DON'T REPEAT.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-snapshot-https-cache-grep/`
- snapshot root: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`
- predecessor artifacts:
  - cycle 212 H-fc1: `/home/sdancer/nmss-emu-magic32-snapshot-config-blob/analysis/config_blob_extract_2026-05-15.md` (canonical shard `7898b3b000.bin` confirmed contains the SDKConstants MapProperty)
  - cycle 218 H-fc2: `/home/sdancer/nmss-emu-magic32-config-fetch-endpoint/analysis/config_fetch_endpoint_2026-05-15.md` (URL prefix confirmed)
- success-fact key: `magic32_snapshot_gmc2_response_recovered_<shard>_<offset>` (a)
- block-fact keys: `magic32_snapshot_gmc2_only_in_rodata` (b), `magic32_snapshot_gmc2_no_response_in_heap` (c)
