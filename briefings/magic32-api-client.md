# magic32-api-client — Turn 9 (H-fc3): probe members.netmarble.com host

## Role & workdir
Live HTTPS probe Codex worker. Workdir: `/home/sdancer/nmss-emu-magic32-api-client`.

## Current goal / sub-goal
- **goal_key**: `nmss_magic32_fresh_capture_enabled` (currently 0.80/1.0)
- **sub_goal_key**: `turn9-members-host-probe`

## Why this turn exists
Four consecutive falsifications on the apis.netmarble.com host:
- Turn 5 (cycle 213): 3 new identity routes → all post-issue verifiers
- Turn 6 (cycle 215): protobuf+octet-stream → same /cpp-auth 404
- Turn 7 (cycle 219): bootstrap gmc2/v4/constants → "wrong path" 404 with server=gmc2 (host LIVE, path stale)
- Turn 8 (cycle 220): 6 alternate gmc2 sub-paths → identical structured 404s with only echo-path differing

The cycle-216 K=6 planner's prescribed fallback for an exhausted apis.netmarble.com is **H-fc3 members.netmarble.com** — a host tagged `authWebViewUrl=https://members.netmarble.com/auth` in the recovered config blob (cycle 212 H-fc1 artifact) that has NEVER been probed.

## Hypothesis
The `members.netmarble.com` host exposes a JSON/REST API under `/api/*`, `/v1/*`, `/sso/*`, or similar sub-paths that accepts the Google-schema body and either (a) returns a MAGIC32 directly OR (b) reveals a missing-parameter / route-hint error contract that narrows future probing.

## Falsification (3 clean outcomes)
- (a) Any route returns 200/4xx with `googleX`-field validation OR a JSON response containing `userKey`/`I_PID`/`playerId` → SUCCESS. Fact `magic32_members_endpoint_<path>_<status>`.
- (b) Different webview-shaped HTML response across routes → host is webview-only, no REST contract. Fact `magic32_members_webview_only_no_rest`.
- (c) All routes return generic 404 → members has no probable issuer route. Fact `magic32_members_all_404_no_route`.

## Success criteria
**Primary**: append `## Task 9 — members.netmarble.com host probe (H-fc3)` section to `analysis/api_client_impl_2026-05-14.md` documenting:
- per-probe table: method, URL, HTTP status, response body (first 300 chars; HTML responses → just the `<title>` and first 100 chars of body)
- final verdict matched to (a)/(b)/(c)

**Closing fact**: see list above.

Print `TURN9_DONE` on the final line.

## Execution flow — DO NOT EXIT BETWEEN STEPS (atomic)

**Step 1** — Reuse the existing Rust probe client. Add `--turn9-members` mode with canonical headers (User-Agent: X-UnrealEngine-Agent, Accept: application/json, gameCode: thered, buildCode: A, NMTimeZone: +08:00).

**Step 2** — 7 probes max (1.5s spacing). Body for POSTs = same Google-schema JSON as Turn 5:
```
{"googleAuthCode":"","googleClientId":"","googleClientSecret":"","googleAppId":"","googleUserId":"a_1408633172786630918"}
```

Probes:
```
1. GET  https://members.netmarble.com/api/                  (api root)
2. POST https://members.netmarble.com/api/login             (api login)
3. POST https://members.netmarble.com/v1/sso                (sso root)
4. POST https://members.netmarble.com/login                 (bare login)
5. POST https://members.netmarble.com/token                 (oauth-style token)
6. POST https://members.netmarble.com/issue                 (issue verb)
7. GET  https://members.netmarble.com/auth                  (the config-blob's actual authWebViewUrl)
```

Capture full response body (first 300 chars) per probe.

**Step 3** — Classify each response:
- 200/4xx with structured JSON naming a `google*` field → outcome (a). The route is the issuer.
- 200 with HTML `<title>...</title>` (login page) → outcome (b). Webview-only.
- 404 with backend-shaped JSON OR generic 404 → outcome (c) for that probe.
- Mix → record per-probe verdict.

**Step 4** — Append section + verdict + fact-set:
```bash
/home/sdancer/orchestrator/harness fact-set <key> "<one-line summary>"
```
Print `TURN9_DONE`.

## Constraints & gotchas
- **HARD probe budget: 7 requests max. 1.5 s spacing → 10.5 s wall time HTTP.**
- **HARD memory budget: 500 MB.** Rust probe client only.
- **NO bulk-enumeration python.** Memory rule `[[bulk-enumeration-needs-explicit-memory-budget]]`.
- **HTML responses are OK** — they confirm outcome (b). Capture `<title>` and first 100 chars.
- **One Codex turn budget**: ≤30 min wall time.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-api-client/`
- Rust probe client: `src/main.rs` (--turn5-sweep, --turn6-protobuf, --turn7-bootstrap-gmc2, --turn8-gmc2-subpaths exist; add --turn9-members)
- prior turns artifact: `analysis/api_client_impl_2026-05-14.md` (Turn 1-8 — APPEND)
- recovered config blob (cycle 212): `/home/sdancer/nmss-emu-magic32-snapshot-config-blob/analysis/config_blob_extract_2026-05-15.md` — confirms `authWebViewUrl = https://members.netmarble.com/auth`
- success-fact key: `magic32_members_endpoint_<path>_<status>` (a)
- block-fact keys: `magic32_members_webview_only_no_rest` (b), `magic32_members_all_404_no_route` (c)
