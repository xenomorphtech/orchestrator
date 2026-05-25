# fresh-thered-account-signup — Create a fresh thered account and complete lobby+GS login

## Role & workdir
Browser-automation + integration worker. Workdir: `/home/sdancer/nmss-emu-fresh-thered-account-signup`. Outbound HTTPS + headless browser allowed.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `fresh-thered-account-signup` (pivot after pdj8pyp3 declared unrecoverable cycle 947)

## Terminal success criterion
A **freshly-created** thered account completes:
1. cpp-auth/v1/sign-in → HTTP 200 errorCode 0, `playerStatus.restriction == []`
2. lobby_login → `login_result.Result == 0`
3. game_server_login → `Result == 0`
4. opcode 901 `PktLobbyNetmarbleSSecurityVerify` sent with cert
5. GS responds (902 or any post-security packet)

If 1–3 work but 4–5 fail, that's a separate sub-issue we'll attack — but stages 1–3 succeeding *with a fresh account* would by itself confirm the workplace-restriction theory and unblock the campaign.

## Hypothesis
A thered account created today via the vampir email-signup pipeline (`get_stoken.py:create_account`) will start with `player.status == 0`, `restriction == []`, and complete lobby+GS login the same way the historical 2026-03-29 captures show.

## Falsification (4 outcomes)
- (a) **Fresh account passes all 5 stages** → `nmss_fresh_account_clientless_login_complete` ← goal success fact. **Metric 5/5**.
- (b) **Fresh account passes stages 1–4 but opcode 901 rejected** → `gs_security_packet_rejected_<short_code>`. Goal at 4/5. Surface for next-path attack.
- (c) **Fresh account also gets `restriction: ["workplace"]`** → Netmarble's workplace flag is geo-based (DE IP triggers it on signup), not account-history-based. Fact: `workplace_flag_triggered_by_de_ip_on_fresh_signup`. Resource ask: Korean/HK proxy required.
- (d) **Signup fails earlier** (reCAPTCHA / mail.tm / region blocked) → fact: `fresh_account_signup_failed_<stage>`.

## Constraints
- **Memory**: 2 GB. **Time**: 60 min wall.
- **One account only.** Do NOT loop signup attempts. If first attempt fails outcome (c) or (d), stop and document.
- **mail.tm** for email. The existing `get_stoken.py create_account` already handles this.
- **No new dependencies.** Use existing tools.
- **Account credentials** must be saved to `/home/sdancer/games/autoproto/accounts/netmarble_thered_<short_token>.json` per the existing autoproto convention.
- **Rate limit**: same as previous turns (1 apis.netmarble.com / 2s, max 30 requests over the turn).

## **USE THE KOREAN PROXY** (resurrected cycle 951)
The cycle-949 signup failure was geo-gating. Cycle 951 confirmed live proxies. Use one for ALL apis.netmarble.com / members.netmarble.com / profile-auth-view.netmarble.com / GS TCP connections in this turn:

- Primary: `http://14a5fdfb7aaa7:0cf801b22d829540@88.223.47.170:12323` (vampir codebase config; returned HTTP 200 for members/auth?countryCode=HK probe; check `/home/sdancer/games/vampir/proxy_ex/test_lobby.exs:30-32` for the exact credentials)
- Set `HTTPS_PROXY` / `HTTP_PROXY` env vars for Python requests + curl
- For Playwright: pass `proxy={"server": "http://88.223.47.170:12323", "username": "14a5fdfb7aaa7", "password": "<from vampir code>"}`
- For raw TCP to GS (`183.110.205.25:12000`): may need a SOCKS5 wrapper — fall back to candidate B/C/D/E (all SOCKS5) if the HTTP proxy can't tunnel TCP. Or attempt direct TCP from local IP first since GS may be geo-gated differently than HTTP endpoints.
- Verify the proxy's exit IP looks Korean/HK via httpbin.org/ip BEFORE attempting Netmarble flows.

## Plan
1. Run `xvfb-run -a python3 /home/sdancer/games/vampir/create_account/get_stoken.py` in full pipeline mode (no `--pid`, no `--email/--password`). Capture: account email, password, pid, JWT, full cpp-auth response.
2. Save the credentials to autoproto accounts/.
3. Check `player.status` and `playerStatus.restriction` on the fresh response. If `restriction != []`, jump to outcome (c) documentation and stop.
4. If clean, run `validate_gameserver.py` (or a patched copy that keeps the GS socket open per cycle-932 briefing) with the fresh tokens.
5. Confirm lobby_login `Result: 0`.
6. Confirm GS version + login `Result: 0`.
7. Send opcode 901 with `Token = <cert from cert-rust-repro using fresh pid as MAGIC32>` and `Log = ""`.
8. Read up to 5 seconds for response; classify per (a)/(b).

## Output deliverables
- `analysis/signup_2026-05-17.md` — narrative with stage-by-stage results
- `analysis/signup_artifacts/` — raw JSON captures of cpp-auth, lobby, GS opcodes
- `analysis/lobby_gs_trace.txt` — sequenced opcode trace
- closing fact via `harness fact-set` per (a)/(b)/(c)/(d)
- final line: `FRESH_THERED_ACCOUNT_SIGNUP_DONE`

## Progress so far
- pdj8pyp3 has `workplace` restriction (cycle 938). Cleared-path attempts (cycle 941–947) found: clearing requires HK/TW/MO phone we don't have. **Pivot to fresh account**.
- vampir pipeline already targets thered; signup via mail.tm proven on 2026-03-29 (fact `clientless_login_complete`).

## Relevant files
- Signup pipeline: `/home/sdancer/games/vampir/create_account/get_stoken.py` (full mode: no flags)
- Account convention: `/home/sdancer/games/autoproto/accounts/netmarble_thered_pdj8pyp3.json` (template)
- GS client: `/home/sdancer/games/vampir/create_account/validate_gameserver.py` (use with patched receive loop per `adapt-vampir-to-thered` task spec)
- Cert pipeline: `/home/sdancer/nmss-emu/cert-rust-repro/` + remote oracle `root@162.244.80.97:9876`
- Opcode 901 shape: `Token (fstring) + Log (fstring)` from `/home/sdancer/games/vampir/protocol_base_report.yaml`
- Harness: `/home/sdancer/orchestrator/harness`
