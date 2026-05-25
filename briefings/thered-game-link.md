# thered-game-link — Link fresh Netmarble account to thered game-player + complete chain

## Role & workdir
Network-investigation worker. Workdir: `/home/sdancer/nmss-emu-thered-game-link`. Outbound HTTPS allowed via Korean proxy `http://14a5fdfb7aaa7:0cf8669540@88.223.47.170:12323` (or any of the 4 alternates from fact `vampir_korean_proxy_88_223_47_170_live`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `thered-game-link`

## Terminal success criterion
**GS accepts opcode 901** for a freshly-created Netmarble account that this turn links to thered, then runs cert → security packet → 902.

## Hypothesis
The vampir corpus identifies two cpp-auth endpoints — `/cpp-auth/v1/fetch-player` and `/cpp-auth/v1/sign-in` (fact `vampir.token_exchange_endpoints`). The signup pipeline previously skipped `fetch-player` because pdj8pyp3 was already game-linked. For a brand-new Members account, **`/cpp-auth/v1/fetch-player` is the call that auto-creates the thered game-player record**, returning a fresh thered-scoped playerId; afterwards `/cpp-auth/v1/sign-in` will succeed and `playerStatus.restriction` will be empty.

## Falsification (3 outcomes)
- (a) **`/cpp-auth/v1/fetch-player` returns a fresh thered playerId, sign-in succeeds, lobby+GS+opcode 901 all complete** → fact `nmss_fresh_account_clientless_login_complete` (the goal's success-fact-key). Metric 5/5.
- (b) **`/cpp-auth/v1/fetch-player` returns OK but downstream lobby/GS/901 fail** → progress to ~4/5; new sub-blocker identified.
- (c) **`/cpp-auth/v1/fetch-player` itself fails** (404, errorCode, wrong shape) → fact `cpp_auth_fetch_player_<status_or_errorcode>`. Investigate alternate link endpoints.

## Constraints
- **Use Korean proxy for ALL Netmarble HTTPS**. Verify exit IP via httpbin first.
- **Use the fresh account** at `/home/sdancer/games/autoproto/accounts/netmarble_thered_rplovhfdkm.json`. DO NOT create another account.
- **Memory**: 1 GB. **Time**: 45 min wall. **Rate limit**: 1 apis.netmarble.com req / 2s.
- Reuse the patched `get_stoken.py` from `/home/sdancer/nmss-emu-fresh-thered-account-signup/` (it has the stale-PID fallback disabled).

## Task 1 (THIS turn — PROBE-FIRST DISCIPLINE)

**The prior worker (cycle 956) OOM'd at 20 GB peak doing libUnreal.so disasm. DO NOT REPEAT.** Order is mandatory:

### Phase A: Local-corpus grep ONLY (≤2 min, ≤256 MB RSS):
- `rg -n -i 'fetch.?player|fetchPlayer|cpp.?auth/v1' /home/sdancer/games/vampir/ /home/sdancer/games/autoproto/ 2>/dev/null`
- That's it. No libUnreal.so this phase.

### Phase B: HTTP probe (≤10 requests via Korean proxy, 2-second pacing):
- Try `POST https://apis.netmarble.com/cpp-auth/v1/fetch-player` with the fresh access_token from `api_account_probe.json` (in nmss-emu-fresh-thered-account-signup worktree's analysis/signup_artifacts/).
- Bodies to try, in order — observe response, adjust based on error:
  1. `{"gameCode": "thered"}` (Authorization: Bearer)
  2. `{"gameCode": "thered", "channelKey": "<netmarbleId>", "channelCode": "20"}`
  3. With Members API headers (clientId, nidV, buildCode) like the existing get_stoken.py uses
- Per probe: record status, headers, response body.

### Phase C: If both A and B fail to identify the right shape:
- ONLY THEN run `strings -a -n 12 /home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so | rg -i 'fetch.?player|cpp-auth/v1/' | head -50` — single streaming pass, NO objdump.
- ABSOLUTELY NO `objdump -s --start-address` on libUnreal.so. That was the OOM cause.

### Phase D: Once fetch-player returns a fresh thered playerId:
1. Call `/cpp-auth/v1/sign-in` with the fresh playerId.
2. Run `validate_gameserver.py` with the fresh SToken — confirm lobby `Result: 0`, GS `Result: 0`.
3. Send opcode 901 with cert computed from fresh playerId-as-MAGIC32. Observe 902.
4. Save all responses, full opcode trace, and verdict to `analysis/thered_link_2026-05-17.md`.

**Hard memory cap**: RSS ≤512 MB. If anything you're about to run might allocate >100 MB, replace it with a streaming alternative.

## Output deliverables
- `analysis/thered_link_2026-05-17.md` — stage-by-stage narrative with final verdict
- `analysis/artifacts/` — raw response JSONs at every step
- closing fact via `harness fact-set` per (a)/(b)/(c)
- final line: `THERED_GAME_LINK_DONE`

## Progress so far
- Phase 0-3: cpp-auth verified, lobby endpoint live, opcode 901 shape known, audit done, restriction-clearing requires HK SMS (cycle 947).
- Phase 4 (cycle 951): Korean proxy `88.223.47.170:12323` is live and unblocks geo gate.
- Phase 5 (cycle 954): fresh account `rplovhfdkm@wshu.net` (`N82eef…`) created via proxy, status="N", clean. BUT `/accounts/v2/user/game/thered` returns `2063 Not linked game` → fresh playerId for thered doesn't exist yet.

## Relevant files
- Fresh account creds: `/home/sdancer/games/autoproto/accounts/netmarble_thered_rplovhfdkm.json`
- Patched signup pipeline: `/home/sdancer/nmss-emu-fresh-thered-account-signup/` (has the working proxy integration)
- Probe results: `/home/sdancer/nmss-emu-fresh-thered-account-signup/analysis/signup_artifacts/api_account_probe.json`
- Vampir source: `/home/sdancer/games/vampir/create_account/get_stoken.py`, `/home/sdancer/games/vampir/protocol/`
- GS client: `/home/sdancer/games/vampir/create_account/validate_gameserver.py` (use with patched receive loop — keep socket open past PktLoginResult)
- Cert pipeline: `cert-rust-repro` + remote oracle `root@162.244.80.97:9876`
- Opcode 901 shape: `Token (fstring) + Log (fstring)` per `/home/sdancer/games/vampir/protocol_base_report.yaml`
- Harness: `/home/sdancer/orchestrator/harness`

Print `THERED_GAME_LINK_DONE` on the final line.
