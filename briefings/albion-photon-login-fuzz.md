# albion-photon-login-fuzz — extract S_trim from live PID 665993 memory and decrypt the captured post-2FA wire

## Role & workdir
Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-photon-login-fuzz` (existing). Substrate: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`.

## Current goal / sub-goal
- **goal_key:** `albion_action_loop`
- **sub_goal_key:** `live_memory_strim_extraction_from_supervised_pid_665993`
- **Success metric:** `https://albion.orch.run/state` shows `zone != null` after the post-2FA Photon f3 03 reply (already captured at 20:14:50Z) is decrypted, its token + GameServerIP + GameServerPort fields are extracted, and `game_server_connect.py` is fired against them via the now-shipped epic-args harness.

## Already achieved (do NOT re-falsify)

| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `/home/sdancer/albion-photon-sdk-research/analysis/turn60_live_s_trim_capture_2026-05-23.md` | Live 96B S_trim capture via INT3 hook on gyd::a6e→ComputeHash is reproducible | ✅ |
| 2 | `albion_strim_DECRYPT_VERIFIED_2026-05-23` fact + `/home/sdancer/albion-photon-sdk-research/analysis/c4149_login_plaintext_decode.md` | Full SHA256(S_trim)→AES-256-CBC IV=0 PKCS#7 chain verified end-to-end on Turn 60 corpus | ✅ |
| 3 | `albion_post_2fa_wire_seq_captured_2026-05-23` fact | Fresh post-2FA Photon sequence (f3 06/07/82/03) captured at 20:14:50Z — present in your worktree's wire log | ✅ |
| 4 | `albion_state_clarification_2026-05-23` fact | 20:14:50Z f3 82 did NOT decrypt with pre-2FA S_trim — different Photon session. Most likely owned by SUPERVISED PID 665993 (which is past-2FA, currently STILL ALIVE on substrate) | ✅ |
| 5 | `albion_harness_self_test_PASSED_2026-05-23` fact + `/home/sdancer/albion-epic-args-prepend/scripts/game_server_connect.py` | game_server_connect.py is cryptographically validated; ready to fire on real token + endpoint data when extracted | ✅ |
| 6 | `albion_supervised_respawn_diagnosed_2026-05-23` + `/home/sdancer/albion-photon-login-fuzz/analysis/c4181_supervised_respawn_diagnosis.md` | Your previous path: identified `tmux session albion-client → albion-supervise → spawn_preload.sh → ./Albion-Online` chain with 3s while-true restart loop | ✅ DONE-this-worker |

## Why this is the highest-value next path

PID 665993 is the SUPERVISED Albion that DID complete the full 2FA handshake (c4170 fact). Its post-2FA Photon traffic was captured on the wire (c4171 fact, the 20:14:50Z f3 03 reply). But we never tapped 665993, so we don't have its S_trim — without it, the f3 03 is undecryptable.

If we can **lift 665993's S_trim out of its live process memory** (still alive, still has the 96B blob in some anon arena per turn60 mapping), we decrypt the already-captured post-2FA f3 03 reply WITHOUT needing a new tap-armed login cycle. That's an order-of-magnitude shortcut over sdk-research's "fresh-tap+fresh-2FA" path.

This is a kernel-instrumentation-class lift (per memory `[[kernel_instrumentation]]`). Root on substrate has `CAP_SYS_PTRACE` even at `kernel.yama.ptrace_scope=1`, so `process_vm_readv` from a root-owned reader should work; if not, `/proc/665993/mem` direct read after PTRACE_ATTACH works regardless of yama scope. Albion's anti-debug zenity-abort fires at STARTUP, not on a live attach to an already-running process (per memory `[[albion_antidebug_ptrace]]`).

## Falsification (closes the path if hit)

If after 3 turns: (a) 665993 has died and was not relaunched (no live target), AND (b) no S_trim file can be recovered from any other live Albion process via memory scan, AND (c) the 20:14:50Z f3 82/f3 03 wire slice cannot be decrypted against any known S_trim — write `/home/sdancer/albion-photon-login-fuzz/analysis/live_memory_strim_blocked.md` with sanitized evidence and exit.

## Next 3 concrete tasks

1. **Verify 665993 is still alive + map its memory.** SSH `root@ssh8.vast.ai`. Confirm `ps -p 665993 -o pid,etime,cmd` (was 27+ min alive at orchestrator c4184 = 20:42Z). Read `/proc/665993/maps` and identify anon/heap arenas (the turn60 scanner already knows the shape — `last-whitespace-token` parse handling anon maps; size cap >0x2000000 needed per the turn-41 fixes). 

2. **Scan 665993's memory for the 96B S_trim blob.** S_trim is the SHA256-truncated shared secret after the DH derivation. It's a 96-byte run that, when SHA256'd, produces the 32-byte AES key used to decrypt the captured f3 82 at 20:14:50Z (already in your wire log). The scan loop:
   - For each candidate 96B window in 665993's heap/anon arenas
   - Compute `key = SHA256(window).digest()[:32]`
   - Decrypt the 20:14:50Z f3 82 ciphertext block with AES-256-CBC IV=0 PKCS#7
   - If plaintext starts with a known Photon header byte AND PKCS#7 padding validates → MATCH. Save to `/home/sdancer/albion-photon-login-fuzz/secrets/strim_665993_extracted_2026-05-23.bin` (mode 600, gitignored).
   - Memory budget: cap scan at 4GB resident, batched 256MB chunks (per memory `[[bulk_enumeration_memory_budget]]`).

3. **Decrypt the post-2FA f3 03 reply + extract token + GameServerIP+Port.** Once you have the matching S_trim → run the full decrypt pipeline (already shipped in your worktree's `scripts/photon_login_same_session_synth.py`) against the 20:14:50Z f3 03 ciphertext. Parse the Photon plaintext for fields named (per c4163 handoff plan in your worktree): `GameServerIP`, `GameServerPort`, and any token-like string. Fact-set `albion_post_2fa_response_decoded_2026-05-23` with field count + sanitized lengths (NEVER the token bytes themselves) — pointing to the verdict file `analysis/c4184_live_memory_decrypt_verdict.md`. This is the signal that fires epic-args's `fire_on_fact_daemon` (currently in flight) which calls `game_server_connect.py` to drive the actual zone events.

## Constraints & gotchas

- **NEVER echo Albion credentials, S_trim bytes, AES key bytes, or token bytes in stdout/logs/chat.** Sanitize: only print field counts, byte-lengths, SHA256-hex-prefixes. Secrets live mode-600 gitignored.
- **Do NOT kill 665993.** It's the live source. If accidentally killed: there's no current-state replacement; sdk-research is independently building a tap-armed relaunch path. Coordinate via fact ledger, not by sharing process state.
- **Do NOT touch sdk-research worktree** (`/home/sdancer/albion-photon-sdk-research/`) — they're in the middle of a long turn writing fresh tap-relaunch logic. Read their secrets/ + analysis/ READ-ONLY; do not write to their tree.
- **Do NOT touch epic-args worktree** (`/home/sdancer/albion-epic-args-prepend/`) — they're shipping fire_on_fact_daemon this turn. Just set the fact when you have the decrypted payload; the daemon will pick it up.
- ptrace anti-debug: Albion's zenity-abort fires at startup time, not on live attach. process_vm_readv from root should work without triggering anything. If it does abort: fall back to `/proc/665993/mem` via PTRACE_ATTACH+detach (memory `[[albion_antidebug_ptrace]]`).
- Memory cap: hard ceiling 4GB. Bounded ranges, batched. Per memory `[[bulk_enumeration_memory_budget]]`.
- Time budget: **45 min for this turn**.

## Relevant files / references
- Your existing decrypt pipeline: `/home/sdancer/albion-photon-login-fuzz/scripts/photon_login_same_session_synth.py`
- Your decrypt watcher status: `/home/sdancer/albion-photon-login-fuzz/analysis/decrypt_watcher_status_2026-05-23.md`
- Reference S_trim format (turn60): `/home/sdancer/albion-photon-sdk-research/secrets/turn60/s_trim_live.1779565243262937147.643559.bin` (96 bytes, prefix `5b5e64a1`)
- Reference AES key (turn60): `/home/sdancer/albion-photon-sdk-research/secrets/turn60/albion_rijndael_key.bin` (64 bytes — first 32 are the AES-256 key, second 32 likely reserved/IV-material)
- Wire-log location (captured 20:14:50Z f3 06/07/82/03 sequence): your worktree's local wire log dump under analysis/ (your last turn wrote it; cross-reference c4171's photon_login_post_f307_sequence_2026-05-23.md)
- game_server_connect.py harness (what fires once token+endpoint are known): `/home/sdancer/albion-epic-args-prepend/scripts/game_server_connect.py`
- Dashboard (success metric): `https://albion.orch.run/state` — watch for `zone.name != null`
