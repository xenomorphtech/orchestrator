# albion-captcha-vision-solve

## Role & workdir

You attached to the sibling `albion-accountportal-cdp` driver's chromium via CDP and solved the Google reCAPTCHA v2 image challenge. The original briefing was scoped to L3.5 (captcha solve). The cascade has since advanced AND broken — your scope is **expanded** to drive the post-submit cascade through L4 (real token capture).

Workdir: `/home/sdancer/albion-captcha-vision-solve` (git worktree, branch `albion-captcha-vision-solve`).

## Achievement state (CRITICAL — read before acting)

| Level | Artifact | Status | Notes |
|---|---|---|---|
| L1 — browser surface | `/home/sdancer/albion-accountportal-cdp/analysis/accountportal_flow_trace.json` | ✅ DONE | nodriver+xvfb under CF clears, real Albion login form rendered |
| L2 — form fill | Same trace, `form_filled` events | ✅ DONE | credentials reached form via CDP |
| L3 — captcha challenge presented | `recaptcha_present` event in trace | ✅ DONE | bframe iframe at (757,355) size 304x78 |
| **L3.5 — captcha SOLVED** | `challenge_crop.png` + `after_round1_submit.png` (your artifacts) | ✅ DONE | Tiles 1,6,7 = fire hydrant; token materialized in `g-recaptcha-response` |
| L3.6 — form submit | Captcha log `[+] login form found; submitting credentials` | ✅ DONE | Sibling driver consumed your token and posted credentials |
| **L4 — token capture** | `/home/albion/accountportal-headed/refresh_token.json` exists BUT contains only `{"kind":"tokenish","source":"document.cookie","value":"_gcl_au=...","metadata":{"fallback":true}}` | ❌ **FAILED — NO REAL TOKEN** | Driver logged `[-] no refresh/access/exchange candidate available for prefs persistence`; flow_trace.json shows `prefs_persist_skipped: no_pref_candidates`. Driver exited 17:41:58Z. |
| L5 — Unity prefs persist | `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` | ❌ blocked by L4 |
| L6 — Albion restart | proc `Albion-Online` PID 549987 still alive but at LoginScreen | ❌ blocked by L4 |
| L7 — zone non-null | dashboard `albion.orch.run/state` zone=null | ❌ blocked by L4 |

**The cascade BROKE at L4 even though `refresh_token.json` exists on disk. Do not log L3.5 success and exit — the goal is L7 (zone non-null), not L3.5.**

## Current goal / sub-goal

- `goal_key`: `albion_action_loop`
- `sub_goal_key`: `l4_real_token_capture_post_submit`
- Success: Albion `refresh_token.json` (on remote) contains a real session token (NOT a Google Analytics cookie), OR the chromium page reaches a state from which Unity can pick up auth, OR the dashboard `zone` becomes non-null after a subsequent Albion restart.

## What the sibling driver did

Sibling driver `accountportal_login.py` (now exited):
1. Polled `g-recaptcha-response` every 5s — you populated it, driver continued.
2. Submitted login form via `form_filled` event.
3. Looked for refresh/access/exchange tokens in the post-submit page — found NONE.
4. Fell back to capturing any cookie that looked tokenish — picked up `_gcl_au` (Google Analytics, not auth).
5. Wrote that to `/home/albion/accountportal-headed/refresh_token.json` (NOT useful).
6. Tried to persist to Unity prefs — skipped because no actual token candidates.
7. Exited.

## Next 3 concrete tasks (post-L3.5, scope-expanded)

1. **Diagnose post-submit page state.** Your chromium is still attached via CDP at port 39223 (SSH tunnel to remote :38185). Inspect:
   - Current page URL (`Runtime.evaluate: window.location.href`)
   - Page title (`document.title`)
   - Visible body text (`document.body.innerText` first 2000 chars)
   - Network history (any `https://*albion*` or `*.albiononline.com/*` requests since submit)
   - Cookies (`document.cookie` and `Network.getAllCookies`)
   - Local storage / session storage
   - Any `callback://` redirect attempts the page tried (look for `Page.frameNavigated` history)
   
   You already captured `live_page.png` at 19:37 — re-examine it AND capture a fresh one (`live_page_post_submit_t2.png`). Identify whether the page is at: (a) 2FA "verify your email" challenge, (b) login-success/redirect page, (c) wrong-credentials error, (d) account-locked, (e) something else.

2. **Drive the cascade based on state.**
   - **If 2FA page**: locate the email-code input field; trigger `inbox_poll.py` (already on remote at `/home/albion/accountportal-headed/inbox_poll.py` — check what creds it has; mailbox is configured per `mailbox.json`); wait for code arrival (~30-90s); type code via CDP `Input.insertText` + click submit. Then re-check for token issuance.
   - **If success/redirect page**: capture the real token from the URL/cookies/storage. The token may live in: `localStorage.getItem('refresh_token')`, `document.cookie` for `*.albiononline.com`, a hidden form input, or a URL hash fragment. Search liberally; the driver's narrow heuristics missed it.
   - **If error page**: report what error message is shown; if "bad credentials" check `/home/albion/accountportal-headed/accountportal.env` for the password the driver actually used (must match the briefing's expected creds: email `5fswkv6zf4@wshu.net`, but DO NOT echo password to chat/logs; just verify it's set non-empty).

3. **Persist + restart.** Once you have a real token (refresh_token / access_token / exchange_code), write it into `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` using the same key Unity expects (read sibling `accountportal_login.py` lines that build the prefs entry for the format; expected key: `Albion_Online.RefreshToken` or similar — confirm in source). Then SIGINT the Albion process (PID 549987 or current) and trust the supervisor to relaunch it. Watch `https://albion.orch.run/state` for `zone != null`.

## Constraints & gotchas

- **DO NOT** spawn a new chromium. Use the existing one attached via CDP port 39223.
- **DO NOT** kill chromium PID 602905. The sibling driver is gone but its browser is still alive — that's your handle to the post-submit DOM state.
- **DO NOT** echo Albion credentials (email/password) into stdout, logs, chat, or git commits. Treat as secret.
- The Albion supervisor `albion-supervise` (PID 443749) will respawn Albion automatically when killed. Don't manually re-exec.
- Unity prefs file IS already loaded by the running Albion at startup — writing it now requires Albion to be RESTARTED (kill+supervisor-respawn) before changes take effect.
- 2FA codes from `mail.tm` expire in ~5 min — get the code AND submit it within that window.
- The mailbox creds for `inbox_poll.py` are likely in `mailbox.json` (you saw it in the dir listing). Don't paste creds; just chain the script.

## Falsification (path-level)

If you cannot identify the post-submit page state OR drive the cascade in 3 more solve-loop iterations, write a verdict file at `/home/sdancer/albion-captcha-vision-solve/analysis/post_submit_diagnosis.md` documenting:
- Exact page URL + visible text
- All `document.cookie` entries (redact value bytes, keep keys + length)
- All network requests since 17:41 (sibling driver's submit time)
- localStorage/sessionStorage key inventory

Then exit — orchestrator will spawn the next sibling.

## Relevant files / references

- Sibling driver source: `/home/sdancer/albion-accountportal-cdp/scripts/accountportal_login.py` (remote copy at `/home/albion/accountportal-headed/accountportal_login.py`)
- Sibling flow trace: `/home/albion/accountportal-headed/accountportal_flow_trace.json` (193KB, contains all sibling-side events through driver exit)
- Captcha log: `/var/log/albion-headed-captcha.log` (on remote)
- Albion Player.log: `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/Player.log`
- Mailbox config: `/home/albion/accountportal-headed/mailbox.json`
- Inbox poller: `/home/albion/accountportal-headed/inbox_poll.py`
- Your existing artifacts: `/home/sdancer/albion-captcha-vision-solve/analysis/{challenge_crop,after_round1_submit,after_anchor_click,live_page}.png`
- Dashboard: `https://albion.orch.run/state` (poll for `zone != null` as proof of L7)
