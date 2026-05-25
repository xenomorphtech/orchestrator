# adapt-vampir-to-thered — End-to-end clientless login + cert + GS security-packet success

## Role & workdir
Integration worker. Workdir: `/home/sdancer/nmss-emu-adapt-vampir-to-thered`. **You will make REAL outbound HTTPS to `apis.netmarble.com` and a REAL TCP connection to the thered Game Server.** This is authorized by the user (goal `nmss_clientless_fresh_login`, refinement 2026-05-17 — read fact `nmss_clientless_login_goal_refined_2026_05_17`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `adapt-vampir-to-thered` (path 2; first attacking path)

## Terminal success criterion
**GS accepts a security packet** (opcode 901 `PktLobbyNetmarbleSSecurityVerify`) — i.e. GS responds with opcode 902 (or any positive post-security-verify packet) — on a TCP connection initiated by this clientless pipeline.

## Hypothesis (the key insight from stage-gap analysis)
The vampir.cpp clientless pipeline already targets **thered** (`GAME_CODE = "thered"` hardcoded in `/home/sdancer/games/vampir/create_account/get_stoken.py`). The on-disk validation_result.json proves stages 1–2 (account/token + GS TCP login) work end-to-end with `success: true, login_result.Result: 0`. The cpp-auth/v1/sign-in response field `pid` (32-char hex, e.g. `BC054AEE37DB4B8286489A1B282715D8`) is structurally identical to **MAGIC32** (same length, same role, same JWT `iss`). So `pid` IS MAGIC32 in the fresh-login flow. The missing piece is just composing opcode 901 with a cert computed from that `pid` + the challenge bytes the GS sends.

## Falsification (3 outcomes — name the closing fact)
- (a) **GS responds positively to a security packet built from the cpp-auth `pid` + a cert from cert-rust-repro** → SUCCESS. Fact: `nmss_fresh_account_clientless_login_complete` (the goal's success-fact-key). Metric goes 0/5 → 5/5.
- (b) **Stages 1–4 all work but opcode 901 → GS responds with error/disconnect** → partial success; the security packet shape or input is wrong. Fact: `gs_security_packet_rejected_<error_classification>`. Investigate the specific error.
- (c) **The vampir pipeline no longer works on the current backend** (token endpoints changed, GS endpoint moved, signup blocked) → reopens stage-1 work. Fact: `vampir_pipeline_regressed_<which_stage>`.

## Success criteria — what "done" looks like concretely
**Primary deliverable**: `/home/sdancer/nmss-emu-adapt-vampir-to-thered/analysis/run_2026-05-17.md` with:
1. **Pipeline run log**: end-to-end from account-reuse (use existing `thered_pdj8pyp3@wshu.net` from `/home/sdancer/games/autoproto/accounts/netmarble_thered_pdj8pyp3.json` — DO NOT create a new account this turn) through cpp-auth/v1/sign-in → fresh `pid` + `NetmarbleSToken` + GS TCP open + version-check + login → record opcode trace of all received packets.
2. **`pid`-is-MAGIC32 verification**: take the fresh `pid`, feed it as MAGIC32 into cert-rust-repro (`/home/sdancer/nmss-emu/cert-rust-repro/`) with a captured-from-GS challenge string, produce a cert. Either verify cert against a known good pair, OR send live to GS and observe acceptance.
3. **Opcode 901 send**: extend `validate_gameserver.py` (or write a sibling) to send `PktLobbyNetmarbleSSecurityVerify` containing the cert. Receive whatever the GS sends back.
4. **Verdict** matched to (a)/(b)/(c) above with closing fact via `harness fact-set`.

Print `ADAPT_VAMPIR_TO_THERED_DONE` on the final line.

## Constraints & gotchas
- **Reuse the existing account** `thered_pdj8pyp3@wshu.net` (credentials in `/home/sdancer/games/autoproto/accounts/netmarble_thered_pdj8pyp3.json`). DO NOT create new accounts this turn — minimize spam risk and keep the run reproducible.
- **Korean proxy may be needed** for `apis.netmarble.com` reachability (per `vampir.full_pipeline_working` fact). If `validate_login.py` fails on TLS/region grounds, document and stop — don't spam retries.
- **Rate limit**: ≤1 HTTPS request per second to apis.netmarble.com. ≤1 TCP connection attempt per minute to GS. If you hit rate-limit / ban responses, STOP and document.
- **Memory budget**: 1 GB RSS hard cap. Per `[[feedback_bulk_enumeration_memory_budget]]`.
- **Time budget**: aim for 90 min total work. If you can't make stage 5 within that window, write up what you have and stop with a clear next-turn handoff.
- **No Frida / no thered process touches**: this is purely clientless. The com.netmarble.thered process on adb-:5558 stays untouched.
- **GS opcode 901 payload format is unknown** — you may need to look at how the legitimate client builds it. Sources to grep:
  - `/home/sdancer/games/vampir/protocol/` (docs)
  - `/home/sdancer/games/vampir/create_account/validate_gameserver.py` (current GS client)
  - The thered libUnreal.so (if needed — `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so`)
  - The captured GS pcaps under `/home/sdancer/dark-december-*/` (different game family but same Netmarble GS framing genus)
- **`feedback_impossibility_caution`**: user said "implement the necessary API call". You're authorized.
- **`feedback_check_existing_decoder_before_re`**: before writing new packet parsers, sweep existing repos for the same opcode work.

## Progress so far
- **Stage-gap analysis** (cycle 928–929): ranks this path #1.
- **Phase 0 (Survey) DONE** (cycle 931, `analysis/run_2026-05-17.md`): function-to-stage map; opcode 901 shape = `Token` + `Log` (both fstring); 902 returns `Result` (int32).
- **Phase 1 (Live cpp-auth) DONE** (cycle 935, `analysis/stage_1_3_capture/get_stoken_output.json`): cpp-auth/v1/sign-in HTTP 200 `errorCode 0` for pdj8pyp3 account. **Fact `nmss_clientless_pid_equals_magic32_2026_05_17` set**: `playerId = pid = JWT iss = I_PID = MAGIC32 = BC054AEE37DB4B8286489A1B282715D8` (32-char hex, single value, no separate server-issued field). Player has `restriction: ["workplace"]` flag — possibly limited access.
- **Phase 2 (Lobby login) FAILED** (cycle 936, `analysis/stage_1_3_capture/validate_gameserver_failure.json`): lobby TCP connect to **183.110.205.25:12000** succeeded; `PktLobbyVersion` received OK; `PktLobbyLogin` sent; **timed out waiting for response (5s)**. The hardcoded `LOBBY_HOST=183.110.205.25` / `LOBBY_PORT=12000` and `GAME_HOST=183.110.40.34:12000` in `validate_gameserver.py` lines 28-32 are **from the 2026-03-29 sibling pipeline run — possibly stale by 7+ weeks**. GS TCP not reached.
- Fact `clientless_login_stage_gap_complete_adapt_vampir_to_thered_proposed` set 2026-05-17T12:40Z.
- **All prior phases are COMPLETE or recorded as FAILED. Do not re-run them. The task below is NEW DIAGNOSIS work.**

## Task 1 (THIS turn): diagnose & fix the lobby_login timeout

The lobby is reachable (TCP connects, PktLobbyVersion returns) but PktLobbyLogin gets no response. Possibilities, in roughly increasing cost:

1. **Endpoint rotation**: lobby IP 183.110.205.25 no longer maps to the thered lobby (only the version probe survives because version-handshake is often the same wire format across Netmarble games). Test: try other Netmarble-game lobby IPs from any cached cpp-auth response, or check whether `apis.netmarble.com` returns a router/dispatcher URL for thered.
2. **Stale auth token format**: PktLobbyLogin shape changed in 7 weeks; the SToken or NID field is rejected and lobby silently drops.
3. **Account gating**: `restriction: ["workplace"]` on the pdj8pyp3 account locks lobby login until a verification step. Note that cpp-auth still returned `errorCode 0` — the gate is on the lobby side.
4. **Korean proxy required**: the current run had `proxied_url_probe: "407_proxy_authentication_required"` — the proxy is stale; if the lobby is geo-gated, this fails silently.

### Concrete steps

1. **Endpoint freshness check** (15 min):
   - Search the captured `cpp-auth_response` (`stage_1_3_capture/get_stoken_output.json`) for any `lobbyHost`, `gameHost`, `dispatcher`, `router` keys. None were visible in the saved cut-down JSON — re-run get_stoken with raw HTTP body capture and dump headers, full body.
   - Grep `/home/sdancer/games/vampir/protocol/` and `/home/sdancer/games/vampir/captures/` for `183.110` to see if there's a more recent IP recorded.
   - Grep the libUnreal.so extract `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so` (a stripped 158 MB ELF) for `183.110.` and any `*.netmarble.com` hostnames that look like lobby/GS routers. (Memory-bound: `strings` with `--bytes=15` is fine — bound to 5 minutes wall time.)
   - If a router endpoint is found, hit it with curl + saved access token to ask for the current thered lobby/GS addresses.
2. **If endpoint is current** (15 min):
   - Diff `PktLobbyLogin` payload (`_nme_json` + `nid` + `pid` + `gameCode`) against what the cpp-auth flow now expects. Test by replaying captures: if `vampir/captures/` has a pcap of a working PktLobbyLogin, compare byte-for-byte. Document the diff.
3. **If lobby still silently drops** (15 min):
   - Try with one of the older Tokens from `vampir/captures/netmarble_stoken.json` (different player) — if THAT lobby login works, the issue is account-scoped (workplace restriction); if not, lobby protocol/endpoint is the issue.
   - Try the alternate Netmarble lobby family (Korean proxy) if there's a hint of geo-gating.

### Output
Update `analysis/stage_1_3_capture/lobby_diagnosis.md` with:
- Endpoint freshness verdict (live / rotated / unknown)
- PktLobbyLogin field diff against any working capture
- Account-scope vs protocol-scope conclusion
- Concrete next-step recommendation: either "lobby login fixable in <X> hours" or "blocked by user resource ask: <what>"

Then either retry lobby login with the fix and proceed to GS TCP + opcode 901, OR stop and record the precise blocker. Set fact `gs_security_packet_rejected_lobby_login_<verdict_token>` or `nmss_fresh_account_clientless_login_complete` as appropriate. Print `ADAPT_VAMPIR_TO_THERED_DONE` on the final line.

## Relevant files / references
- Upstream pipeline: `/home/sdancer/games/vampir/create_account/`
- Account credentials: `/home/sdancer/games/autoproto/accounts/netmarble_thered_pdj8pyp3.json`
- Cert pipeline: `/home/sdancer/nmss-emu/cert-rust-repro/`
- Cert tests: `/home/sdancer/nmss-emu/cert-rust-repro/tests/captured_sp968_5vec_verify.rs`
- Refined-goal fact: run `harness fact-get nmss_clientless_login_goal_refined_2026_05_17`
- Stage gap doc: `/home/sdancer/nmss-emu-clientless-login-stage-gap/analysis/stage_gap_2026-05-17.md`
- Memory rules: `[[feedback_impossibility_caution]]`, `[[feedback_inapp_webview_cdp]]`, `[[feedback_bulk_enumeration_memory_budget]]`, `[[feedback_check_existing_decoder_before_re]]`
- Harness binary: `/home/sdancer/orchestrator/harness`
