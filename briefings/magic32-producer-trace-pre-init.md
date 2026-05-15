# magic32-producer-trace-pre-init — Turn 2: classify origin from existing enumeration

## Role & workdir
Offline forensics analyst, turn 2. Workdir: `/home/sdancer/nmss-emu-magic32-producer-trace-pre-init`. **No device interaction. No fresh captures. No aeon MCP. No Frida.** Pure analysis of existing artifact + snapshot.

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro` (stalled-meta retracted).
- Sub-goal: **classify the origin** of MAGIC32 = `2FCF997702C244969BFEAF7F0D6AAA1C`. Tasks 2-5 only — Task 1 enumeration is closed.

## Prior turn status
Turn 1 closed cleanly with `analysis/task1_magic32_ascii_offsets_2026-05-14.md` (18.9 KB) — 52 ASCII occurrences across 12 snapshot shards. **The data already contains the answer.** Specifically:

- **`userKey=2FCF99…` URL parameter** appears in 5 occurrences across 3 shards (`78882ef000.bin` x2, `78a2b0b000.bin` x2 = donor lane, `78b733f000.bin` x1).
- Each preceded by `uRVm3j0Q8fTCcjh4ySDBHdA&` (looks like another URL parameter — session-token/signature shape).
- Donor lane (`78a2b0b000.bin@0x541bbb`) continues into `…&userKey=2FCF99…\x007IdSsgI0PskKtBXqSYmfA\x00…AyQzI0N…` — multiple null-separated URL fragments.
- Token also appears as `"playerId":` / `"PlayerID":` / `"I_PID":` / `"appUserId":` / `"AppUserId":` JSON-key contexts (Netmarble-internal + analytics SDKs like AppsFlyer).

This is **strong network-origin evidence**. Turn 2 must confirm or refute, then close the path.

## Success criteria
Append a section "## Task 2-5 Closure" to `analysis/task1_magic32_ascii_offsets_2026-05-14.md` (DO NOT rewrite; APPEND). Sections:

1. **URL context recovery** — for each of the 5 `userKey=` hits, dump ±512 B around the offset to recover the full URL: scheme + host + path + all query parameters. The donor lane `78a2b0b000.bin@0x541bbb` is the best candidate.
2. **Request vs response classification** — does the surrounding bytes contain `HTTP/1.`, `Host:`, `\r\n\r\n`, `Content-Type:`, or any HTTP-header markers? If yes, is the marker BEFORE the URL (= request being built) or AFTER (= response being parsed)?
3. **Cross-check with libUnreal.so producer chain** — earlier `nmss_magic32_origin_recovered_16_of_16` claimed AES encryption of PGS playerId. Is that consistent with network-origin? Hypothesis: the AES is for **encrypting payloads sent TO Netmarble** using a userKey that was server-issued in a prior request. NOT for deriving the userKey locally.
4. **Verdict + fact-set** — set ONE of:
   - `harness fact-set magic32_producer_origin_classified "network_fetch_from_netmarble_userkey_endpoint_<host_if_recovered>"`
   - `harness fact-set magic32_producer_origin_classified "local_computation_from_<inputs_observed>"` (only if Task 2 finds an HTTP-response context is absent AND a clean computation evidence exists)
   - `harness fact-set magic32_producer_origin_classified "indeterminate_<reason>"`
5. **Next-path recommendation** — one paragraph. If network-origin: "first-launch pcap with TLS interception" or "kernel-module library-injection to hook the SDK's HTTP client before TLS wraps the request" (the user's standing kernel-aggressive directive applies). If local: "recover the AES key from the snapshot via call-site disasm of `OnGetPGSPlayerIdWithAuthCode`".

## Tasks (single-pass, do not loop)

### TASK 2 — full URL recovery
Read `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78a2b0b000.bin` at offset `0x541bbb` ±512 B and `78882ef000.bin` at `0x379d09` ±512 B. Look for:
- `https://` prefix preceding the URL fragment
- `Host:` header
- `?` query-string start
- Path components like `/auth/`, `/login/`, `/userKey`, `/installation`, `/register`
- Recognizable hostnames: `netmarble.com`, `m.netmarble`, `mw.nmgame`, `nmssecure`, `netmarbleslog`

Extract the full URL (or as much as is recoverable).

### TASK 3 — request/response classification
For each of the 5 URL hits, dump ±1024 B and check:
- Presence of `HTTP/1.` or `HTTP/2` (response status line)
- `Host: <host>\r\n` (request)
- `Content-Length:`, `Set-Cookie:`, `Server:`, `Date:` (response markers)
- `User-Agent:`, `Accept:`, `Cookie:` (request markers)

Classify each as: request-building / response-parsing / detached-fragment.

### TASK 4 — libUnreal.so consistency check
The previously-recovered producer chain says AES encrypt happens at `OnGetPGSPlayerIdWithAuthCode` (JNI in `libUnreal.so`). 
- If MAGIC32 is server-issued (network fetch), AES at that PC is for **encrypting payloads sent TO Netmarble** (e.g., the request body containing PGS auth_code) using a pre-existing userKey.
- If MAGIC32 is locally derived, AES at that PC produces it.

Look at the AES call site disassembly in `/home/sdancer/aeon/libUnreal.so` near `0x195b9f8` (the earlier-identified AES PC) — what bytes are in x0/x1/x2 at the call? The plaintext source matters: if x1 (input) is a PGS player ID string, it's local derivation; if x1 is a JSON body and x2 (key) is the existing MAGIC32 token, it's encryption-of-payload-WITH-the-token.

Optional, only if static disasm is quickly feasible.

### TASK 5 — close + fact-set + exit
- Append Task 2-5 Closure section to the existing artifact.
- Set `magic32_producer_origin_classified` fact.
- Append a 1-line entry to `/home/sdancer/orchestrator/analysis/falsified.md` for portfolio bookkeeping if the path closes (done-success), or note as path-progressing if more work needed.

## Constraints & gotchas
- **No re-enumeration.** Task 1 is closed; don't re-grep the snapshot.
- **No huge file reads.** Use `dd bs=1 skip=… count=2048` or `python3 -c 'open(...).seek(N); print(f.read(2048).hex())'` to slice byte ranges.
- **Memory budget**: stay under 1 GB. Avoid `numpy` for pcap-style reading; use plain `open(..., "rb").read()` with seek/slice.
- Single-pass turn: tasks 2-5 in order, append, set fact, exit. Don't loop.

## Falsification (acceptable outcomes)
- URL fragments do NOT contain HTTP markers, host, or path → mark `magic32_origin_indeterminate_no_url_context` and propose the kernel-injection live capture path.
- URL clearly says request-building (Host: header before the URL): the token is being SENT by the client, which means it was previously OBTAINED → server-issued, confirms network origin.
- URL clearly says response-parsing (HTTP/1. + Set-Cookie / Content-Type before the URL): the token is being RECEIVED → first-time fetch from server, confirms network origin.

## Relevant files / references
- snapshot: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`
- existing artifact: `analysis/task1_magic32_ascii_offsets_2026-05-14.md` (APPEND, don't overwrite)
- key facts: `magic32_origin_url_userKey_evidence_2026_05_14` (just set), `nmss_magic32_origin_recovered_16_of_16` (older, may be misread), `ctx_210_is_device_PID_not_per_challenge_2026_04_26`
- libUnreal.so: `/home/sdancer/aeon/libUnreal.so` (for optional Task 4 disasm)
- AES PC from prior work: `0x195b9f8` in libUnreal.so
- This is a **single targeted turn** — keep it under 8 minutes. Tight scope.
