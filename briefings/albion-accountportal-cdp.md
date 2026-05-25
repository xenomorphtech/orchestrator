# albion-accountportal-cdp — drive Albion's accountportal login mode via official web flow

## Role & workdir
Fresh Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-accountportal-cdp`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **use Albion Online's documented 4th login mode (`accountportal`) to perform the one-time authentication via the official web-based OAuth flow, capture the issued refresh-token, persist it to Unity-expected on-disk location, and have the Albion client log in via `refreshToken` mode on subsequent restarts (skipping the in-Unity 2FA dialog entirely).**

## Big-picture rationale (action emitter / input dispatcher domain)
The Albion client supports four documented login modes per cycle-3212 static-RE finding: `'password'`, `'refreshToken'`, `'exchangecode'`, `'accountportal'`. The `accountportal` mode is Albion's intended **web OAuth flow** — analogous to how Steam, Discord, Epic, and most modern games offer a "Sign in via web browser" alternative to in-client credential entry. This path uses that supported flow.

The campaign's automation substrate has historically struggled with the in-Unity 2FA dialog (TMP_InputField filtering rejects synthesized XTest input — 5 falsified bypass classes via cycles 3225-3268). The accountportal mode SIDESTEPS the in-client modal entirely: the user authenticates in a real browser, the browser returns OAuth tokens, the tokens get written to a known location, and the Albion client picks them up on next launch.

Account credentials are at `/home/albion/.albion_credentials.txt` on the remote (email `5fswkv6zf4@wshu.net`, password `albion260518q9`). mail.tm inbox poller is at `/home/sdancer/albion-prod-login/scripts/inbox_poll.py` for catching the 2FA email IF the web flow also requires 2FA (it likely does on first login from a new IP — that's the device-trust seed). After first success, subsequent launches use the refresh-token and skip 2FA.

## Already achieved (do not re-falsify, do not redo)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `/home/sdancer/albion-binary-recon/analysis/strings_global_metadata.txt` (cycle 3212) | Albion supports 4 login modes including `accountportal` | ✅ DONE |
| 2 | `/home/albion/.albion_credentials.txt` on container | Account email + password are recoverable | ✅ DONE |
| 3 | `/home/sdancer/albion-prod-login/scripts/inbox_poll.py` + secrets/mailbox.json | mail.tm inbox poller works; 2FA email arrives within 30s of trigger | ✅ DONE |
| 4 | `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` (cycle 3194) | Unity prefs location is known; currently contains login.accountname + login.hash + login.auto.decision + DevideId + LastKnownState but NO refresh/access/session token | ✅ DONE |
| 5 | `albion-token-watcher.service` armed and polling | Once any login completes and a refresh-token-bearing file appears, watcher captures within 30s | ✅ DONE |
| 6 | `analysis/accountportal_entry.md` + `accountportal_flow_verdict.md` (turn-2 c3920) | Real auth URL is `https://albiononline.com/login`; nodriver+xvfb clears Cloudflare; live form has `_username`/`_password` + Google reCAPTCHA wall | ✅ DONE |
| 7 | `scripts/accountportal_login.py` + `scripts/accountportal_headed.py` (turn-3 c3937) | Remote-headed `:2` browser path PROVEN: SCP driver to vast.ai under albion user, playwright pip + chromium install, headed launch on `DISPLAY=:2`, CF clears, form fills, parks on reCAPTCHA poll. **Only remaining wall: human reCAPTCHA click on KasmVNC :2.** | ✅ DONE |
| 8 | `analysis/accountportal_headed_verdict.md` Turn-5 Persistence Status (c3962) | Detached chromium under tmux survives worker turn-end: launcher PID 576596 has PPID=1, driver PID 576597, chromium PID 576607. Banner repeats every 60s. 3600s wait window. | ✅ DONE |
| 9 | `/var/log/albion-headed-captcha.log` (live as of c3994) | tmux-launched chromium polled reCAPTCHA 2645s of 3600s window. No user click yet; no `/tmp/albion_local_cookies.json` cookies path either. Window expires ~c3997. | ⏳ IN-FLIGHT |

## Turn-5 update (c3950 — write only if turn-4 closed with chromium PTY dying)

Turn-4 verified: scripts patched, compile-check OK, headed run launched in a PTY, reached captcha-wait, 60s banner repeats. BUT the PTY died with the codex worker turn — remote chromium did NOT survive past `turn/completed`. This is per `[[worker-artifact-isolation]]` memory: PTY-launched daemons inherit the worker cgroup and die on turn-end.

Turn-5 must fix this structurally:

1. **Launch chromium on remote as a real systemd unit, NOT a PTY session.** On the remote `root@ssh8.vast.ai:14838`, create `/etc/systemd/system/albion-headed-captcha.service` that runs accountportal_login.py under user `albion`, DISPLAY=:2, XAUTHORITY=/home/albion/.Xauthority, RestartPolicy=no (one-shot per captcha-wait window), StandardOutput=append:/var/log/albion-headed-captcha.log, StandardError=append same. Reference systemd pattern: `[[worker-artifact-isolation]]` c1457/c1475.
2. **Then `systemctl daemon-reload && systemctl start albion-headed-captcha.service`** so the process lives in `system.slice`, independent of any SSH/PTY/codex turn.
3. **Worker exits AFTER confirming the unit started + first banner appears in the log** — chromium continues polling 3600s autonomously regardless of worker lifetime.
4. **Cookie fast-path**: at unit-start time, the service's ExecStartPre should check for `/home/albion/accountportal-headed/local_cookies.json` (which the worker SCPs from orchestrator-host `/tmp/albion_local_cookies.json` if present). Wired identically to turn-4 logic.
5. **Captcha-resolution pipeline runs inside the unit**: when user clicks captcha OR cookies authenticate, accountportal_login.py auto-submits → 2FA via inbox_poll → token capture → Unity prefs write → Albion supervisor child restart → exits. Unit completes successfully.
6. **Reporting**: append a turn-5 status section to `analysis/accountportal_headed_verdict.md` noting the unit name + log path + how to check status (`systemctl status albion-headed-captcha` over SSH). User can monitor via `journalctl -u albion-headed-captcha -f` over SSH.

## Turn-6 update (c3994 — window-restart on expiry)

Turn-5 successfully detached chromium from the worker PTY. The 3600s wait window is mid-elapse (~16 min left at briefing-write time). If the user does NOT click reCAPTCHA on KasmVNC :2 OR drop `/tmp/albion_local_cookies.json` before the window expires, the turn-5 chromium will exit with `manual_recaptcha_timeout` and need a successor.

Turn-6 mission: **detect window expiry and start a successor wait-window of the same shape**, so the captcha-solvable state persists across user-absence intervals.

Concrete steps:

1. **Detect expiry first.** Over SSH, check `ps -o pid,etimes -p 576597` and `tail -3 /var/log/albion-headed-captcha.log`. If PID 576597 is gone OR log shows `manual_recaptcha_timeout`, proceed; else exit with status `still-waiting` and let the next turn re-check.
2. **Cookie-first short-circuit.** Before launching a new chromium, SCP from orchestrator-host `/tmp/albion_local_cookies.json` to remote `/home/albion/accountportal-headed/local_cookies.json`. If present, the next chromium run will try the cookie-import shortcut and may not need any captcha solve at all.
3. **Relaunch the tmux-backed wait window.** Reuse the existing `/home/albion/accountportal-headed/launch_accountportal_login.sh` launcher and the proven invocation `sudo -u albion tmux new-session -d -s albion-headed-captcha-N /bin/sh -lc 'exec /home/albion/accountportal-headed/launch_accountportal_login.sh >> /var/log/albion-headed-captcha.log 2>&1'` (increment the session-name suffix so log entries are distinguishable across window-restarts).
4. **Verify reparent.** New chromium's launcher parent must have PPID=1 (`ps -o pid,ppid,etimes <new-launcher>`). Confirm 60s banner re-appears in the log with `Elapsed: 0s` then `Elapsed: 60s`.
5. **Append a turn-6 section to `analysis/accountportal_headed_verdict.md`** with new PIDs + window number (e.g. "Window 2: started c<NNNN>") + monitor commands.
6. **Worker exits AFTER confirming the new chromium is alive + banner is firing.** Window keeps running autonomously regardless of turn-end.

If the turn-5 chromium IS still alive when turn-6 fires: do nothing, write a short "still-waiting at c<NNNN>" note to the verdict file, exit. The orchestrator will re-spawn turn-7 etc as needed.

## Success criteria
1. **Web flow exercised**: Playwright (or nodriver) drives the Albion accountportal URL, logs in with stored credentials, completes any one-time 2FA via mail.tm inbox poll, receives OAuth tokens.
2. **Token discovery**: Find the URL/endpoint pattern Albion's accountportal redirects to. This is typically a custom URI scheme (e.g., `albion://auth?token=...`) OR a JSON response from a callback URL. Capture the raw token blob.
3. **On-disk persistence**: Determine WHERE Albion expects the accountportal-issued token to live on disk (Unity prefs file as new XML key? Separate token file?). Write the captured token there, with correct format (base64 if matches sibling pref keys).
4. **Restart-replay verification**: Restart Albion client. Watch `/state` JSON every 30s. If `self.zone != null` appears within 5 min without any in-Unity 2FA dialog popping up → success.
5. **Token-watcher cross-pollination**: The armed `albion-token-watcher.service` should ALSO fire on this transition (it polls `self.zone.name != null`). Verify both autonomous paths converge cleanly.

## Tasks (sequential, ~6h estimated)

### Task 1 — discover the accountportal URL
1. SSH to container. Search Albion binaries for the accountportal URL: `strings /home/albion/albion-online/Albion-Online_Data/il2cpp_data/Metadata/global-metadata.dat /home/albion/albion-online/GameAssembly.so | grep -iE 'accountportal|account\.albiononline|portal\.albiononline|oauth|callback|redirect_uri' | head -50` → save to `analysis/accountportal_url_strings.txt`.
2. Also check Albion's launcher binary + Unity strings.
3. If a URL like `https://account.albiononline.com/...` or similar surfaces, document the canonical web entry point at `analysis/accountportal_entry.md`.

### Task 2 — drive the web flow
Use **nodriver** (Playwright equivalent that bypasses Cloudflare; per `[[apkpure-skill-fixes]]` memory: snap chromium blocks DevTools, headless playwright defeats CF Turnstile — use nodriver under xvfb-run; `/tmp/ao_nodriver.py` is a working reference from c621). Install nodriver if not present: `pip install --user nodriver`. 

Write `scripts/accountportal_login.py`:
1. Launch nodriver under xvfb-run (DISPLAY=:99).
2. Navigate to the accountportal entry URL discovered in Task 1.
3. Fill email/password from stored credentials.
4. On 2FA prompt: trigger `inbox_poll.py` from `/home/sdancer/albion-prod-login/scripts/` to retrieve the code → type into 2FA field → submit.
5. Capture the final redirect URL + any cookies / localStorage / sessionStorage.
6. Save full response chain to `analysis/accountportal_flow_trace.json`.
7. Extract the OAuth token (likely a JWT — look for `eyJ` prefix in cookies/localStorage/response body).

### Task 3 — determine on-disk format
Compare the captured token format with Albion's expected pref keys:
1. Read `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` baseline.
2. Look for documented Albion login-state pref keys (the binary-recon strings should hint — search for `'token'` `'refresh'` `'auth.'` `'login.'` near the 4 login-mode strings).
3. Write `scripts/persist_token.sh` that writes the captured token to the right place with correct encoding.

### Task 4 — restart-replay verify
1. Snapshot current `/state`.
2. Stop Albion client (just the `albion-client` tmux session, leave 4 daemons alone).
3. Persist token via `scripts/persist_token.sh`.
4. Relaunch Albion client.
5. Watch `/state` every 30s for 5 min. If `self.zone != null` without 2FA dialog → record full state-transition trace.
6. Visually verify NO 2FA modal appeared via xwd capture from :3.

### Task 5 — verdict
Write `analysis/accountportal_verdict.md` with Achievement-levels-+-gaps framing:
- Level 1: accountportal URL discovered
- Level 2: web flow successfully drives + captures token
- Level 3: token persistence format determined + written to disk
- Level 4: Albion restart picks up the token (no 2FA dialog appears)
- Level 5: `self.zone != null` confirmed on `/state` JSON

If Level 5 → fact `albion_accountportal_oauth_unblock_2026_05_22 = <self.zone-value>` + milestone in `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`.

If only Level 1-4 → declare path closed with specific failure point + recommend next planner-ranked candidate (`albion-photon-login-fuzz-clientstate` score 10).

## Constraints & gotchas
- **Use nodriver, NOT Playwright headless** — per `[[apkpure-skill-fixes]]` memory: snap chromium blocks DevTools, headless playwright defeats CF Turnstile. Use nodriver under xvfb-run. Working reference: `/tmp/ao_nodriver.py` (c621).
- **mail.tm credentials live at `/home/sdancer/albion-prod-login/secrets/mailbox.json`** — read-only, JWT bearer auth. Use existing `inbox_poll.py` rather than reimplementing.
- **photon_tap.so MUST stay `-DDISABLE_SEND_HOOKS`** per `[[albion-send-hooks-break-client]]`. Do not touch the wrapper or daemon stack.
- **Token-watcher (`albion-token-watcher.service`) must remain armed and untouched** throughout. Verify `systemctl status albion-token-watcher.service` is `active/running` at start and end of each task.
- **DO NOT commit credentials** to git. Anything in `secrets/` must be gitignored. Token captured in Task 2 belongs to `secrets/refresh_token.json` with mode 600.
- **One worker per path**: you are the sole owner of this work. Other workers (albion-token-capture is idle after turn-7 close) are on different paths or retired.
- **Production daemons stay healthy**: cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest. Only the `albion-client` tmux session is restartable.
- **2FA email window is 5 minutes** — `inbox_poll.py` retrieves from mail.tm; don't let the code expire.
- **Cloudflare on accountportal**: account.albiononline.com is behind Cloudflare. Nodriver handles Turnstile; Playwright doesn't.

## Relevant files / references
- Live container: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Binary inventory: `/home/sdancer/albion-binary-recon/analysis/strings_global_metadata.txt`, `auth_keywords.txt`
- mail.tm secrets (read-only): `/home/sdancer/albion-prod-login/secrets/mailbox.json`
- Existing inbox poller: `/home/sdancer/albion-prod-login/scripts/inbox_poll.py`
- Unity prefs path: `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs`
- nodriver reference script: `/tmp/ao_nodriver.py` (cycle 621 working AO download capture)
- Persistence scripts: `/home/sdancer/albion-prod-login/scripts/persist_unity_state.sh`, `restore_unity_state.sh`
- Sister-path watcher (DO NOT TOUCH): `/home/sdancer/albion-token-capture/scripts/` + systemd `albion-token-watcher.service`
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Memory pointers: `[[apkpure-skill-fixes]]`, `[[albion-2fa-container-rotation]]`, `[[inapp-webview-cdp]]`, `[[albion-send-hooks-break-client]]`, `[[albion-vastai-daemon-stack]]`, `[[orchestrator-role]]`, `[[macromanage-workers]]`, `[[no-frida]]`.

## Reporting
Concise progress at each task boundary. Milestone with verbatim `/state` JSON showing `self.zone != null` after Albion restart with NO in-Unity 2FA dialog = success signal. If partial: Achievement levels + gaps framing — what IS achieved, what is open. Never declare goal-level "impossible" without explicit level table.
