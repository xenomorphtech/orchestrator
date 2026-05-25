# unrestriction-webview-audit — Audit the restriction-clearing webview for pdj8pyp3

## Role & workdir
Network-investigation worker. Workdir: `/home/sdancer/nmss-emu-unrestriction-webview-audit`. **Outbound HTTPS allowed BUT bounded**: ≤10 GET requests to `*.netmarble.com`, no POSTs that would mutate account state.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `unrestriction-webview-audit` (sibling of `adapt-vampir-to-thered` blocked + `workplace-restriction-investigation` done)

## Why this turn exists
Investigation (cycle 941) found that the Netmarble SDK has a `GetRestrictionStatus` / `ShowUnrestriction` flow and an `unrestrictionWebViewUrl` config key, with the likely concrete URL being `https://profile-auth-view.netmarble.com/restriction`. This turn AUDITS that URL with valid pdj8pyp3 session credentials to determine: (1) is the page reachable, (2) what does the page ask the user to do, (3) can the steps be automated, or do they need human-in-the-loop.

## Hypothesis
Driving `https://profile-auth-view.netmarble.com/restriction` (or sibling URLs) with the valid pdj8pyp3 NetmarbleSToken yields a page that names the workplace restriction and offers a clearing action. The action may be self-completable (e.g. button click) or require external steps (phone/email verify).

## Falsification (3 outcomes)
- (a) **Page renders + names a self-completable clearing action** (e.g., "I agree" button, terms acceptance) → SUCCESS. Fact: `unrestriction_webview_self_completable_<short_description>`. Next path attempts the click.
- (b) **Page renders + requires out-of-band step** (phone/email/captcha/customer support) → falsifies self-serve. Fact: `unrestriction_webview_requires_<step_type>`. Decision: either user takes the step manually, or fall back to new-account path.
- (c) **Page does not render with our session** (401, 403, redirect to login, blank) → token scope insufficient. Fact: `unrestriction_webview_session_insufficient`. May need different access-token scope, or members.netmarble.com auth flow first.

## Success criteria
**Primary deliverable**: `/home/sdancer/nmss-emu-unrestriction-webview-audit/analysis/webview_audit_2026-05-17.md` with:
1. **Pre-audit token verification**: confirm the saved `NetmarbleSToken` from `/home/sdancer/nmss-emu-adapt-vampir-to-thered/analysis/stage_1_3_capture/get_stoken_output.json` is still valid (JWT exp not past) — if past, re-issue via `get_stoken.py --pid BC054AEE37DB4B8286489A1B282715D8` (don't restart full signup).
2. **Probe sequence** (≤10 GETs total):
   - GET `https://profile-auth-view.netmarble.com/restriction` with no auth headers → record status + redirect chain + body first 1024 chars
   - GET same URL with `Authorization: Bearer <stoken>` → same record
   - GET same URL with `?playerId=...&gameCode=thered&nid=...` query params → same record
   - GET `https://profile-auth-view.netmarble.com/restriction?accessToken=<stoken>` → same record
   - One more variant if any of the above returned a hint about expected params
   - Also probe sibling URLs `https://members.netmarble.com/auth` and `https://members.netmarble.com/auth/restriction` (informative, not destructive)
3. **Per-probe row**: status, response headers (esp. `Location`, `Set-Cookie`), body excerpt (first 1024 chars), any `Content-Security-Policy` or framing notes.
4. **Verdict** matched to (a)/(b)/(c) above + closing fact via `harness fact-set`.

Print `UNRESTRICTION_WEBVIEW_AUDIT_DONE` on the final line.

## Constraints
- **GET only**. No POSTs, no clicks, no state mutation.
- **≤10 requests total**, 2-second pacing between them.
- **Memory budget**: 256 MB.
- **Time budget**: 15 min wall.
- **NO Playwright** this turn (heavy); use `requests` + headers manipulation.
- **Korean proxy may be needed** — start without proxy; if responses look geo-gated (region mismatch, `403`), document and stop.
- **Honor `[[feedback_inapp_webview_cdp]]`**: off-device CDP fails for device-bound URLs. The `profile-auth-view` URL MIGHT or might NOT be device-bound — the audit will tell us.

## Progress so far
- **Phase 0**: vampir clientless pipeline maps to thered (cycle 935 cpp-auth live HTTP 200).
- **Phase 1**: lobby_login fails because pdj8pyp3 account has `playerStatus.restriction=["workplace"]` (cycle 938).
- **Phase 2 (workplace-restriction-investigation)**: identified `GetRestrictionStatus` / `ShowUnrestriction` SDK flow and likely clearing URL `https://profile-auth-view.netmarble.com/restriction` (cycle 941).
- **This turn (unrestriction-webview-audit)**: confirm that URL is reachable and see what it asks.

## Next 1 concrete task
Produce `analysis/webview_audit_2026-05-17.md` per the success criteria above.

## Relevant files
- Token JSON: `/home/sdancer/nmss-emu-adapt-vampir-to-thered/analysis/stage_1_3_capture/get_stoken_output.json`
- Account creds: `/home/sdancer/games/autoproto/accounts/netmarble_thered_pdj8pyp3.json`
- Investigation report: `/home/sdancer/nmss-emu-workplace-restriction-investigation/analysis/workplace_restriction_2026-05-17.md`
- Existing strings inventory: `/home/sdancer/nmss-emu-magic32-api-client/analysis/task1_issue_endpoint_inventory_2026-05-14.md`
- Memory: `[[feedback_impossibility_caution]]`, `[[feedback_inapp_webview_cdp]]`
- Harness binary: `/home/sdancer/orchestrator/harness`
