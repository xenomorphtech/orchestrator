# albion-photon-sdk-research — Fresh tap-armed Albion relaunch + 2FA cycle to capture post-2FA S_trim + wire data

## Role & workdir
Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-photon-sdk-research` (existing). Substrate: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`.

## Current goal / sub-goal
- **goal_key:** `albion_action_loop`
- **sub_goal_key:** `tap_armed_relaunch_post_2fa_wire_capture`
- **Success metric:** `https://albion.orch.run/state` shows `zone != null` after post-2FA Photon f3 03 (with actual game token) is decoded by login-fuzz's verified pipeline, daemon fires `game_server_connect.py`, and frida-ingest catches zone events.

## Already achieved (do NOT re-falsify)

| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `/home/sdancer/albion-photon-sdk-research/secrets/turn60/s_trim_live.1779565243262937147.643559.bin` (96B) + `albion_rijndael_key.bin` (64B) | Live S_trim capture pipeline via INT3 hook on gyd::a6e→ComputeHash WORKS | ✅ |
| 2 | `albion_strim_DECRYPT_VERIFIED_2026-05-23` + `c4149_login_plaintext_decode.md` | SHA256→AES-256-CBC IV=0 PKCS#7 chain verified end-to-end | ✅ |
| 3 | `/home/sdancer/albion-photon-login-fuzz/analysis/c4186_validator_20_14_50.json` + `c4186_f303_20_14_50.json` | login-fuzz's cross-session-decrypt-validator works on pre-2FA wire data using existing Turn 60 key | ✅ |
| 4 | `albion_supervised_respawn_diagnosed_2026-05-23` + login-fuzz c4181 diagnosis | Launch chain = tmux session `albion-client` → `albion-supervise` → `run-albion-client` → `runuser -l albion ... spawn_preload.sh` → `./Albion-Online` | ✅ |
| 5 | `c4186_live_memory_blocked.md` (login-fuzz, c4195) | Live-memory path on PID 665993 is BLOCKED (ptrace_scope=ro, cap_sys_ptrace not in bounding set, eBPF lockdown=confidentiality) — your sibling-path independence is therefore the campaign-critical path | ✅ |
| 6 | `/etc/systemd/system/albion-fire-on-fact.service` running on orchestrator host | fire-on-fact daemon polls every 30s for `albion_post_2fa_response_decoded_2026-05-23` and auto-fires `game_server_connect.py` with token + GameServerIP/Port | ✅ |

## Hypothesis

**H**: If we kill the supervised orphan 665993 (now safe — login-fuzz's live-memory path is closed) and launch a FRESH tap-armed Albion-Online on Xtigervnc :3 via the proven spawn_preload.sh chain WITH the full LD_PRELOAD trap stack (2fa_trap + strim_tap + photon_tap + x11_shim), then drive Unity login + 2FA via xdotool + inbox_poll, the same-session post-2FA f3 03 will be captured on the wire log + a new S_trim will land in `/tmp/albion-s-trim/`. Login-fuzz's verified validator will then decrypt the new f3 03 → extract token at param[57]/[251] + GameServerIP/Port → fact-set → daemon fires → zone events.

## Falsification

If, after 2 turns of substrate-side work: (a) tap-armed Albion fails to launch with full LD_PRELOAD chain, OR (b) launches but Unity wedges before 2FA modal, OR (c) modal completes but no new S_trim file appears in `/tmp/albion-s-trim/`, OR (d) S_trim appears but new f3 03 still 211 — write `/home/sdancer/albion-photon-sdk-research/analysis/c4195_tap_relaunch_blocked.md` with sanitized evidence and exit.

## Next 3 concrete tasks

1. **Clear substrate + relaunch tap-armed Albion.** SSH `root@ssh8.vast.ai`. The supervised orphan PID 665993 is now safe to kill (login-fuzz blocked-verdict committed; preserve no longer required). Steps:
   - `tmux kill-session -t albion-client` (kills the dormant supervisor session — per c4181 diagnosis)
   - `pkill -f Albion-Online` (kill the orphan 665993)
   - Wait 5s, verify zero Albion-Online procs
   - `chmod 777 /tmp/albion-s-trim` (ensure tap can write)
   - Re-launch via the proven chain. The simplest path: `tmux new-session -d -s albion-client /usr/local/bin/albion-supervise albion-client /usr/local/bin/run-albion-client` — this restores the supervisor + auto-relaunch pattern.
   - BUT before restoring supervisor, you may want to MANUALLY launch with the full trap chain (since `run-albion-client` may not include the 2fa_trap + strim_tap libs):
     ```sh
     runuser -l albion -c \
       "HOME=/home/albion DISPLAY=:3 XAUTHORITY=/home/albion/.Xauthority \
        XDG_RUNTIME_DIR=/home/albion/.xdg-runtime \
        LD_PRELOAD=/tmp/albion_2fa_trap.so:/tmp/albion_strim_tap.so:/opt/albion-frida-capture/preload/photon_tap.so:/opt/albion-x11-shim/x11_send_event_shim.so \
        /opt/albion-frida-capture/spawn_preload.sh"
     ```
   - Wait 60-90s for Unity to render LoginScreen on Xtigervnc :3. Verify the new tap-child PID via `pgrep -a Albion-Online`.

2. **Drive Unity login + 2FA via xdotool + inbox_poll.** Use the proven flow from prior turns:
   - Type email into Unity TMP_InputField via xdotool (Xtigervnc :3 accepts XTEST per memory `[[albion_login_substrate]]`).
   - Type password (creds in `/home/albion/accountportal-headed/accountportal.env` — NEVER echo).
   - Click Login button.
   - Wait for 2FA modal screen (~30s).
   - Run `/home/sdancer/albion-prod-login/scripts/inbox_poll.py` to fetch the latest 2FA code from the mail.tm inbox. Write to `/tmp/albion_2fa_code.txt` mode 600 chown albion (the 2fa_trap helper thread polls this file and INT3-substitutes the code via il2cpp_string_new at the modal's getter).
   - Click Submit / OK on the 2FA modal.
   - Wait 30-60s for either: (a) post-2FA loading screen advances, (b) modal accepts and game-loading starts.

3. **Verify post-2FA S_trim capture + sibling-decrypt pipeline + fact-set.** When 2FA is accepted:
   - Check `/tmp/albion-s-trim/` for a NEW `s_trim_live.<ts>.<pid>.bin` file (96 bytes, prefix entropy check). Copy to `/home/sdancer/albion-photon-sdk-research/secrets/turn-<n>/`.
   - Locate the post-2FA wire packets in the wire log (sibling tap-child's f3 06/07/82/03 sequence post-acceptance — newer than the 20:14:50Z capture).
   - Run login-fuzz's validator (in their worktree at `/home/sdancer/albion-photon-login-fuzz/scripts/photon_login_same_session_synth.py` OR similar) against the new (S_trim, ciphertext) pair. The f3 03 plaintext should now have `return_code = 0` (or non-211) AND param[57]/[251] should hold the REAL game-session token (NOT the nonce) AND additional params should encode `GameServerIP` and `GameServerPort`.
   - Fact-set `albion_post_2fa_response_decoded_2026-05-23` with sanitized field structure (NEVER token bytes; use SHA256-prefix or count-only). Example: `{"token_len": 16, "token_sha256_prefix": "abc...", "GameServerIP": "<ip>", "GameServerPort": 5056, "s_trim_path": "/home/sdancer/.../s_trim_live.<ts>.<pid>.bin"}`.
   - The `albion-fire-on-fact.service` daemon on the orchestrator host will detect the fact within 30s and auto-fire `game_server_connect.py`.

## Constraints & gotchas

- **DO NOT echo Albion credentials, S_trim bytes, AES key bytes, or token bytes in stdout/logs/chat.** Sanitize: only print field counts, byte-lengths, SHA256-hex-prefixes. Secrets live mode-600 gitignored.
- **2FA codes are SINGLE-USE + 5min validity.** Re-fetch via inbox_poll AT the modal-click moment, not earlier.
- **photon_tap.so MUST be built with -DDISABLE_SEND_HOOKS** per memory `[[albion_send_hooks_break_client]]`. If the existing one isn't, rebuild before launch.
- **Xtigervnc :3, NOT Xkasmvnc :2** per memory `[[albion_login_substrate]]` — Albion's TMP_InputField rejects XTEST input on Xkasmvnc.
- **Do NOT touch login-fuzz worktree** — they're done; their artifacts are read-only references.
- **Do NOT touch epic-args worktree** — their daemon is running as albion-fire-on-fact.service. Just trigger via fact-set.
- Time budget: **45 min per turn**.

## Relevant files / references
- Substrate: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`
- Creds: `/home/albion/accountportal-headed/accountportal.env`
- Inbox poller: `/home/sdancer/albion-prod-login/scripts/inbox_poll.py`
- LD_PRELOAD libs (substrate): `/tmp/albion_2fa_trap.so`, `/tmp/albion_strim_tap.so`, `/opt/albion-frida-capture/preload/photon_tap.so`, `/opt/albion-x11-shim/x11_send_event_shim.so`
- Spawn wrapper: `/opt/albion-frida-capture/spawn_preload.sh`
- Run wrapper: `/usr/local/bin/run-albion-client`
- Supervisor: `/usr/local/bin/albion-supervise` (3s while-true relaunch loop)
- Dashboard: `https://albion.orch.run/state` (zone metric)
- Login-fuzz validator (read-only): `/home/sdancer/albion-photon-login-fuzz/scripts/photon_login_same_session_synth.py`
- Login-fuzz reference decodes: `/home/sdancer/albion-photon-login-fuzz/analysis/c4186_validator_20_14_50.json`, `c4186_f303_20_14_50.json`
- Game-server harness (downstream consumer, auto-fired by daemon): `/home/sdancer/albion-epic-args-prepend/scripts/game_server_connect.py`
- Daemon source: `/home/sdancer/albion-epic-args-prepend/scripts/fire_on_fact_daemon.py` (DEFAULT_TOKEN_KEY=57, MIRROR=251, OP_CODE=0xBC)
