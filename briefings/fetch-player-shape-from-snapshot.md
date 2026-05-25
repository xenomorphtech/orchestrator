# fetch-player-shape-from-snapshot — Mine the trampoline memdump for fetch-player request body

## Role & workdir
Memory-disciplined recon worker. Workdir: `/home/sdancer/nmss-emu-fetch-player-shape-from-snapshot`. Outbound HTTPS allowed via Korean proxy for final shape-verification probes only.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `fetch-player-shape-from-snapshot`

## Why this turn exists
Sibling `thered-game-link` (cycle 958) confirmed `/cpp-auth/v1/fetch-player` is reachable but rejects all 10 JSON probes with `errorCode 1100 required-parameter-not-found` once `gameCode: thered` is set. The shape is server-validated, server-side fields list unknown. The thered Android process (pid 8715 on the RK3588) made this call at least once historically; its heap residue lives in the canonical snapshot `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/`. This turn mines that snapshot for the actual request body or for `fetch-player` URL literals + adjacent constant tables.

## Hypothesis
The trampoline memdump contains either (a) the JSON request body that thered actually sent to `/cpp-auth/v1/fetch-player`, or (b) the libUnreal.so rodata that lists the field-set the server expects. Either gives us the working shape.

## Falsification (3 outcomes)
- (a) **Heap-residue request body recovered** → fact `cpp_auth_fetch_player_request_body_recovered_<short_hash>`. Use it to re-probe via the Korean proxy and confirm `playerId` returns.
- (b) **No heap residue but rodata field-list / schema recovered** → fact `cpp_auth_fetch_player_schema_from_libunreal_rodata`. Construct best-effort body, re-probe.
- (c) **Neither heap nor rodata yields the shape** → fact `cpp_auth_fetch_player_shape_undocumented_in_snapshot`. Surface as resource ask (need a live thered-app pcap or mitm capture).

## Constraints (HARD)
- **RSS cap 512 MB.** No `objdump -s` on libUnreal.so. No multi-region loads.
- **Time cap 30 min wall.**
- **Snapshot tools**: `grep -a -b -F` (memory-mapped, streaming), `strings -a -n 12`, `dd if=... bs=4M skip=N count=1` for small windows. NO Python that reads entire shards into memory.
- **Probe budget**: ≤6 outbound HTTPS to apis.netmarble.com after shape is identified; 2-second pacing.
- Honor `[[feedback_bulk_enumeration_memory_budget]]`.

## Tasks

### Task 1: Snapshot grep for fetch-player residue (≤10 min)
- The snapshot directory: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`
- Find shards: `ls /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/*.bin` (don't recurse, just list)
- For each shard, **streaming grep** without loading: `grep -a -b -F 'fetch-player' <shard>` → record file:offset hits
- For top 5 hits, dump ±256 bytes context: `dd if=<shard> bs=1 skip=<offset-256> count=512 | xxd`
- Note any JSON-looking blocks with `gameCode`, `playerId`, `channelKey`, `nidV`, etc. nearby
- Save to `analysis/heap_grep_2026-05-17.md`

### Task 2: If Task 1 finds a JSON request body fragment, build the exact-shape probe (≤10 min, ≤6 probes)
- Through Korean proxy `http://14a5fdfb7aaa7:0cf8669540@88.223.47.170:12323`
- Use the fresh account access_token from `/home/sdancer/games/autoproto/accounts/netmarble_thered_rplovhfdkm.json` (or re-issue via `/home/sdancer/nmss-emu-fresh-thered-account-signup/` patched scripts)
- POST `https://apis.netmarble.com/cpp-auth/v1/fetch-player` with the snapshot-derived shape
- Save probes to `analysis/artifacts/snapshot_shape_probe_N.{headers,body.json}`

### Task 3: If fetch-player returns a fresh thered playerId, complete the chain
- cpp-auth/v1/sign-in → lobby login → GS login → opcode 901 → 902
- Save artifacts. Set fact `nmss_fresh_account_clientless_login_complete` if 902 received.

### Task 4 (fallback): If Tasks 1-2 don't yield a working shape
- Document what was tried, declare outcome (c), suggest live-pcap as next path.

## Output
Final report `analysis/shape_from_snapshot_2026-05-17.md` with stage-by-stage results + closing fact + `FETCH_PLAYER_SHAPE_FROM_SNAPSHOT_DONE` on final line.

## Progress so far
- All prior phases through cycle 958: cert pipeline 5/5 done, fresh account exists, Korean proxy works, fetch-player endpoint reachable, body shape unknown (errorCode 1100).
- Sibling artifacts: `/home/sdancer/nmss-emu-thered-game-link/analysis/` (10 probe captures), `/home/sdancer/nmss-emu-fresh-thered-account-signup/analysis/signup_artifacts/api_account_probe.json`

## Relevant files
- Memdump shards: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/*.bin`
- Korean proxy creds: fact `vampir_korean_proxy_88_223_47_170_live` (in vampir/proxy_ex/test_lobby.exs)
- Fresh account: `/home/sdancer/games/autoproto/accounts/netmarble_thered_rplovhfdkm.json`
- Patched signup pipeline: `/home/sdancer/nmss-emu-fresh-thered-account-signup/`
- Harness: `/home/sdancer/orchestrator/harness`
