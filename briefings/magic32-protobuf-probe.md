# magic32-protobuf-probe — Probe apis.netmarble.com auth with gzip protobuf encoding

## Role & workdir
Network/RE probe worker. Workdir: `/home/sdancer/nmss-emu-magic32-protobuf-probe`.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `magic32-protobuf-probe` (first path under the new goal)

## Hypothesis
All 31 prior `apis.netmarble.com` probes (turns 1–5, JSON-encoded) were rejected at the **content-type / protocol-format** layer, not at the field-validation or credential layer. The sibling-game `vampir.cpp` (Hercules-obfuscated DD) production auth POSTs use **gzip-compressed protobuf** + `Content-Type: application/octet-stream` + `User-Agent: ProjectRED/UE5-CL-0`. Re-issuing the same routes with protobuf encoding should surface a *different* error class (missing protobuf fields rather than malformed JSON), unlocking layer-2 of the auth handshake.

## Falsification (3 outcomes)
- (a) **Probe returns a non-4xx HTTP code (2xx/3xx/5xx) OR a 4xx with a distinct error code not seen in any of the 31 JSON probes** → content-type was the gate; body shape now matters. Fact: `magic32_protobuf_probe_unlocks_layer2`. Metric stage 3 → 1.
- (b) **Probe returns the same 4xx/error code as JSON probes** (`405`, `errorCode 2000 INVALID_TOKEN_VERIFY_PARAMETER`, or equivalent) → content-type was not the gate; rejection is upstream (host header, TLS fingerprint, route prefix). Fact: `magic32_protobuf_probe_same_response_as_json`. Hypothesis falsified.
- (c) **Probe returns a "device key missing" / "nmDeviceKey required" error** → request reaches the device-binding layer; the next blocker is a per-device header (a different shape of OAuth resource ask). Fact: `magic32_protobuf_probe_reaches_device_key_layer`. Distinct from JSON probes' earlier rejection.

## Success criteria
**Primary deliverable**: `/home/sdancer/nmss-emu-magic32-protobuf-probe/analysis/protobuf_probe_2026-05-17.md` containing:
1. **Test matrix** — minimum 8 probes across 3 candidate routes × {empty body, dummy-protobuf body, gzip-empty body} × {with/without `ProjectRED/UE5-CL-0` UA}. Routes:
   - `https://apis.netmarble.com/identity/secure`
   - `https://apis.netmarble.com/identity/api/verification`
   - `https://apis.netmarble.com/identity/api/v2/verification`
2. **Per-probe row**: HTTP status, response headers (esp. `X-Error-Code`, `Set-Cookie`, `Content-Type`), response body (first 256 bytes hex + ASCII), TLS handshake notes.
3. **Diff table** vs cycle-213 JSON probe results (the 31-row corpus), highlighting which fields changed under protobuf encoding.
4. **Verdict** matched to (a)/(b)/(c) + closing fact via `harness fact-set`.
5. Print `MAGIC32_PROTOBUF_PROBE_DONE` on the final line.

## Constraints & gotchas
- **Memory budget**: 512 MB max RSS, hard. Per `[[feedback_bulk_enumeration_memory_budget]]`.
- **Probe budget**: **HARD ≤20 HTTP requests total**, **1 request per 2 seconds**. If you hit 429, stop and document. No retries on 4xx.
- **TLS**: use `requests` + system openssl. Document `requests.adapters.HTTPAdapter` cipher list. Don't go raw-socket.
- **No real credentials**: dummy fields only. Don't include real PGS tokens, GMS auth, or device IDs. The goal is layer-2 unlocking, not authenticated login.
- **User authorization**: `[[feedback_impossibility_caution]]` — user explicitly authorized: "implement the necessary api call".
- **Adb device**: irrelevant for this probe (clientless by design — no thered process touched).
- **Do NOT extend probe scope** beyond the 3 routes listed. Bound the work.

## Progress so far (this is the first turn under the new goal)
- Goal `nmss_clientless_fresh_login` opened 2026-05-17.
- Cycle 213 baseline: 31 JSON probes against 10 routes returned 405 / `errorCode 2000 INVALID_TOKEN_VERIFY_PARAMETER`. Closed at outcome (b) `magic32_api_client_3_new_routes_also_blocked`.
- Cycle 211–212 config-blob recovery surfaced full URL inventory at `/home/sdancer/nmss-emu-magic32-strings/extract_config_blob.py`.
- Sibling-game format reference: harness fact `vampir.cpp_auth_binary_format` (gzip protobuf + `application/octet-stream` + `ProjectRED/UE5-CL-0`).
- Cached working MAGIC32 (for shape-verification only — DO NOT SEND): `2FCF997702C244969BFEAF7F0D6AAA1C`.

## Next 3 concrete tasks
1. `git status` to confirm worktree state. Write `probe.py` (≤150 lines): one function per probe, results streamed to `results.jsonl`. Use `time.sleep(2)` between requests.
2. Execute the 8-probe matrix. Stream results to disk. Inspect each row.
3. Write `analysis/protobuf_probe_2026-05-17.md` per the success criteria above. Set the closing fact via `harness fact-set`.

## Relevant files / references
- Config blob extractor: `/home/sdancer/nmss-emu-magic32-strings/extract_config_blob.py`
- Sibling format fact: `vampir.cpp_auth_binary_format` (run `harness fact-get vampir.cpp_auth_binary_format` for full content)
- Memory: `[[feedback_impossibility_caution]]`, `[[feedback_bulk_enumeration_memory_budget]]`
- Harness binary: `/home/sdancer/orchestrator/harness`
