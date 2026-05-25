# unrestriction-flow-drive — Clear pdj8pyp3 workplace restriction via Playwright + mail.tm

## Role & workdir
Browser-automation worker. Workdir: `/home/sdancer/nmss-emu-unrestriction-flow-drive`. **Outbound HTTPS + headless browser + mail.tm polling allowed**. Authorized: `harness facts | grep nmss_clientless_login_goal_refined_2026_05_17` shows user explicit "implement the necessary api call".

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `unrestriction-flow-drive`

## Terminal success criterion
After this turn: `playerStatus.restriction` for pdj8pyp3 changes from `["workplace"]` to `[]`. Verified by re-running cpp-auth/v1/sign-in and confirming the field.

## Task 1 (CHEAP PROBE FIRST — do this before Playwright)
Run a fresh cpp-auth/v1/sign-in for pdj8pyp3 via the existing `get_stoken.py --pid BC054AEE37DB4B8286489A1B282715D8 ...` shortcut (it skips full signup, just refreshes SToken + playerStatus). Save the response to `analysis/probe_2026-05-17.json`.

**If `playerStatus.restriction == []`**: user already cleared it manually. **Set fact `workplace_restriction_cleared_user_manual_2026_05_17`**, write a 3-line note, and STOP without running Playwright. Print `UNRESTRICTION_FLOW_DRIVE_DONE` immediately. The sibling path `adapt-vampir-to-thered` will retry lobby_login on the next orchestrator cycle.

**If `playerStatus.restriction == ["workplace"]` still**: proceed to Task 2 below.

## Task 2 (Playwright pipeline — only if Task 1 says still restricted)

Implementation plan (write as `analysis/clear_workplace.py`):

1. **Setup**: `xvfb-run -a` Playwright (Chromium) with realistic UA + viewport. Re-use any helpers from `/home/sdancer/games/vampir/create_account/get_stoken.py` for mail.tm polling.
2. **Navigate**: `https://profile-auth-view.netmarble.com/restriction?playerId=BC054AEE37DB4B8286489A1B282715D8&gameCode=thered&accessToken=<fresh_stoken>` (the React SPA accepts query params — see `nmss-emu-unrestriction-webview-audit/analysis/webview_audit_2026-05-17.md` for the endpoint inventory).
3. **Wait for app boot** (load `main.31b84824.js`), then wait for the reCAPTCHA Enterprise checkbox to render.
4. **Click the reCAPTCHA checkbox** and observe outcome:
   - If green check → score-based, no image puzzle. Proceed.
   - If image challenge → STOP. Save screenshot to `analysis/recaptcha_challenge.png`. Set fact `unrestriction_flow_drive_blocked_recaptcha_image_challenge`. Surface as a resource ask (paid solver / human required).
5. **Identify the email-verify path** in the page UI (button labeled with "email" semantics). Click it.
6. **Wait for POST `/v2/restriction/workplace/release/email`** to fire, capture the response.
7. **Poll mail.tm** for the email to `thered_pdj8pyp3@wshu.net` — see `/home/sdancer/games/vampir/create_account/get_stoken.py` for mail.tm-polling code. Extract the verification code (usually 6 digits).
8. **Enter the code in the page UI**, click submit.
9. **Wait for POST `/v2/restriction/workplace/release`** to finalize, capture the response.
10. **Re-run cpp-auth probe** (same as Task 1) and confirm `playerStatus.restriction == []`.

Save:
- `analysis/clear_workplace.py` (the driver)
- `analysis/run_log.txt` (stdout/stderr)
- `analysis/probe_after.json` (verifying cleared)
- screenshots of any failure points

## Falsification (3 outcomes — write the closing fact)
- (a) **Restriction cleared end-to-end** → fact `nmss_workplace_restriction_cleared_2026_05_17`. Goal progress 2.7/5 → 3/5.
- (b) **reCAPTCHA image challenge blocks** → fact `unrestriction_flow_drive_blocked_recaptcha_image_challenge`. Resource ask: paid solver or human.
- (c) **Other failure** (mail.tm empty, email-route disabled for this region, etc.) → fact `unrestriction_flow_drive_blocked_<short_classification>`. Document with screenshot.

## Constraints & gotchas
- **NO new account creation**. We're clearing an existing account's flag.
- **Memory budget**: 2 GB (Playwright is heavy). Time budget: 30 min wall.
- **Rate limit**: 1 cpp-auth call per probe, max 3 total in this turn. 2-second pacing between any apis.netmarble.com requests.
- **Korean proxy**: if any of the workplace-release endpoints return 403/region-gated, document and stop — proxy stale.
- **DO NOT click any "delete account" / "withdraw" buttons** in the SPA — operate only on workplace-restriction-clear flow.
- **Honor `[[feedback_inapp_webview_cdp]]`**: off-device CDP fails for device-bound URLs; profile-auth-view appears NOT device-bound per cycle 943 audit (any auth variant served identical shell).
- **Honor `[[feedback_apkpure_skill_fixes]]`**: snap chromium blocks DevTools; headless playwright defeats some captchas. Use nodriver-style if standard Playwright triggers a captcha-detection.

## Progress so far
- **Phase 0** (cycle 931): function-to-stage map written.
- **Phase 1** (cycle 935): live cpp-auth HTTP 200, fresh JWT, pid=BC054AEE37DB4B8286489A1B282715D8 captured.
- **Phase 2** (cycle 938): lobby_login timed out — diagnosed as account-state-scoped workplace restriction.
- **Phase 3** (cycle 941): investigation surfaced SDK GetRestrictionStatus + ShowUnrestriction + unrestrictionWebViewUrl flow.
- **Phase 4** (cycle 943): audit confirmed URL `https://profile-auth-view.netmarble.com/restriction` reachable, gated by reCAPTCHA Enterprise + email/SMS verify.
- **Phase 5** (this turn): attempt the clearing flow.

## Relevant files
- Token JSON: `/home/sdancer/nmss-emu-adapt-vampir-to-thered/analysis/stage_1_3_capture/get_stoken_output.json` (note: JWT expires 2026-05-18T13:05Z — still valid for the next ~22h)
- Account creds: `/home/sdancer/games/autoproto/accounts/netmarble_thered_pdj8pyp3.json`
- get_stoken.py source: `/home/sdancer/games/vampir/create_account/get_stoken.py` (use its `--pid` mode + mail.tm helpers)
- Endpoint inventory: `/home/sdancer/nmss-emu-unrestriction-webview-audit/analysis/webview_audit_2026-05-17.md`
- Harness binary: `/home/sdancer/orchestrator/harness`

Print `UNRESTRICTION_FLOW_DRIVE_DONE` on the final line of whatever artifact wraps up the turn.
