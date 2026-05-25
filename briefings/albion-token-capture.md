# albion-token-capture — capture & persist Albion refresh-token across container rotations

## Role & workdir
Fresh Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-token-capture`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **capture the Albion `refreshToken` once (post a one-time manual 2FA login), persist it across vast.ai container rotations, and replay it on cold start so the client logs in via `refreshToken` mode (skipping 2FA) and reaches `self.zone != null`.**

## Big-picture rationale
Static RE of Albion's IL2CPP binary (cycle 3210-3212) discovered the client supports **four login modes**: `'password'`, `'refreshToken'`, `'exchangecode'`, `'accountportal'`. The error strings `"don't have a refreshToken, giving up!"` and `"try to refresh accessToken and try again"` show the client's launch flow attempts `refreshToken` mode FIRST — only when missing does it drop to the Login form (which then requires 2FA on a fresh device/IP). Server-side validation strings `AuthInvalidRefreshToken` + `AccountLoginDeniedNeedTwoFactor` confirm 2FA only fires in `password` mode against an untrusted device. Therefore **one manual 2FA followed by token capture + persistence = permanent autonomous unblock**.

Prior facts already established (do not re-falsify):
- Unity prefs file BEFORE any successful login contains NO refresh/access/session token (cycle 3194 fact `albion_prefs_no_session_token_2026_05_22`). This means the token is written somewhere AFTER a successful login completes — your job is to find that "somewhere" and ensure it survives container rotation.
- All five userspace input-synthesis substrates (xdotool variants, raw RFB, fully-initialized RFB, vncdotool) are filtered at Albion's app layer (cycle 3191 fact `albion_all_input_substrates_no_op_2026_05_22`). Do NOT pursue an "automate the 2FA click" angle — the one-time 2FA will be user-driven via external TigerVNC. Your job starts AFTER that.
- Photon login server is `loginserver.live.albion.zone:5055` (UDP). Authentication is Photon-UDP-native, not HTTPS-REST (cycle 3212 fact `albion_no_http_auth_api_2026_05_22`).
- LD_PRELOAD photon_tap.so MUST stay `-DDISABLE_SEND_HOOKS` per memory `[[albion-send-hooks-break-client]]` — read photon recv only; never intercept send.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | Static binary strings recovered `refreshToken` login mode | Mechanism exists in Albion client | ✅ DONE (cycle 3212 strings_global_metadata.txt) |
| 2 | Account credentials present at `/home/albion/.albion_credentials.txt` on container | Material for the one-time manual login | ✅ DONE |
| 3 | mail.tm inbox poller built at `/home/sdancer/albion-prod-login/scripts/inbox_poll.py` | Receives the 2FA code (used during user-driven manual login if needed) | ✅ DONE |
| 4 | Unity prefs file at `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` enumerated PRE-login (no tokens) | Baseline known | ✅ DONE |
| 5 | `albion-prod-login/scripts/persist_unity_state.sh` + `restore_unity_state.sh` exist for snapshot/restore of the config dir | Persistence machinery already drafted | ✅ DONE |

## Success criteria
1. **Capture proof**: a refresh-token string (or its in-memory address + format) recovered from a live Albion process post-2FA, archived to `/home/sdancer/albion-token-capture/secrets/refresh_token.json` (mode 600, gitignored).
2. **Replay proof**: a fresh container restart (or simulated reset by `systemctl restart` + wipe + restore) followed by `tail -f` of the Photon recv side showing the client immediately login via `refreshToken` mode (no 2FA dialog seen on `/screenshot.png` or KasmVNC :2 viewer). Audit-log entry capturing the transition.
3. **End-to-end**: `curl https://albion.orch.run/state` returns `self.zone != null` within ≤5 min of post-restore launch, with no user-driven 2FA on this restore cycle.
4. **Non-regression**: 5 production daemons stay healthy (cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest, Albion-Online).

## Tasks (sequential, branch on intermediate findings)

### Task 1 — locate where Albion stores the refresh-token AFTER a successful login
The user will perform a one-time manual 2FA login via TigerVNC (we treat this as a given prerequisite; per the standing resource ask). BUT you can START investigating NOW without waiting:

1. Connect to the live container. Check if Albion is currently in a logged-in state (`/state` returns `self.zone != null`) OR if a prior session has produced any cached token files. Look at:
   - `~/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/` (entire dir, not just `prefs`)
   - `~/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/Profiles/`
   - `~/.local/share/`, `~/.cache/`, `/tmp/AlbionOnline*`
   - The Albion install dir under `/home/albion/albion-online/` for any session-state files (looking for *.session, *.token, *.cache, *.json containing what looks like a JWT or refresh blob)
2. Read `/home/sdancer/albion-binary-recon/analysis/strings_global_metadata.txt` for tokens near the `refreshToken` string: there may be filename templates (`{0}_session.dat`), preference key names (`LastRefreshToken`, `auth.refreshToken`), or directory hints.
3. Search the Albion process map for hot pages containing JWT-like patterns: `pidof Albion-Online` → read `/proc/<pid>/maps` → scan rw-p anonymous regions for `eyJ` (JWT header b64 of `{"alg":...`) or 32-64 byte high-entropy ASCII blobs near the strings `refreshToken=` or `accessToken=`. Use a simple memory-scan script (Python + `/proc/<pid>/mem`).
4. Document findings at `analysis/token_storage_locations.md`.

### Task 2 — design the capture mechanism
Based on Task 1 findings, choose the LEAST-INVASIVE capture mechanism:
- (a) **File capture** (preferred): if the token is in a regular file, just copy it. Track which file(s).
- (b) **Memory scan**: if only in process memory, write `/home/sdancer/albion-token-capture/scripts/scan_for_refresh_token.py` that snapshots `/proc/<pid>/mem` for the rw-p regions, regex-matches the token format, and saves it.
- (c) **LD_PRELOAD recv-side hook**: hook `recv()` / `recvfrom()` on libc inside the Albion process to intercept the Photon UDP payload that delivers the `accessToken`+`refreshToken` pair after successful auth. NEVER hook send-side per the DISABLE_SEND_HOOKS invariant.
- (d) **photon_tap pcap mining**: photon-pcap-send already taps recv-side traffic; mine its captures for the token exchange frame (server→client) carrying the token blob. Decoder logic may need to handle Photon's encryption layer if used — confirm whether the auth-channel is encrypted.

Document the chosen approach + falsification criterion at `analysis/capture_design.md`.

### Task 3 — implement & test capture
Once the user-driven 2FA happens (signal: `/state` shows `self.zone != null` or screenshot shows char-select), immediately run your capture mechanism and verify a token blob was retrieved. Save to `secrets/refresh_token.json` with timestamp + scope metadata. **Treat this file as credential material — mode 600, gitignored, NEVER committed.**

### Task 4 — design the replay mechanism
Two complementary approaches; implement whichever is correct based on Task 1 findings:
- **File-replay**: if the token lives in a file, write a `scripts/restore_token.sh` that restores it to the correct path BEFORE Albion starts.
- **Pref-injection**: if the token belongs in Unity prefs, write/update the right `<pref name="..." type="string">...</pref>` entry. Make sure the encoded format matches what Albion expects (base64? ascii? prefixed length?).
- **Photon-injection**: if the token is consumed via a Photon op (worst case), build a small Photon-UDP client that handshakes + sends the refresh-token login op directly — Albion can then resume. This is high-effort; treat as last resort.

### Task 5 — verify across simulated restart
1. Note the current `/state`. Snapshot.
2. Stop Albion (`systemctl stop` or `pkill Albion-Online` on remote).
3. Wipe the Unity config dir BUT also restore the persisted token via your mechanism.
4. Relaunch Albion.
5. Observe: no 2FA prompt should appear; login should proceed silently; `/state` should within ≤5 min report `self.zone != null` again.

### Task 6 — milestone
Append milestone to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` with full state-transition trace + verbatim `/state` JSON. Set facts: `albion_refresh_token_captured_2026_05_22 = <storage-mechanism-summary>`, `albion_refresh_token_replay_verified_2026_05_22 = <true|false-with-reason>`, `albion_autologin_e2e_2026_05_22 = <self.zone-value>` (set this last fact only on full success).

## Constraints & gotchas
- **No LD_PRELOAD on libUnreal.so** — per `[[no-frida]]` memory (anticheat-protected). LD_PRELOAD on Unity-side X libs / libc-side network is OK.
- **photon_tap.so MUST stay `-DDISABLE_SEND_HOOKS`** — see `[[albion-send-hooks-break-client]]`. Read-only Photon taps. Do NOT add a send-side hook to inject the refresh token via Photon — instead use the client's own login flow with the token in the correct on-disk location.
- **No re-falsification** of the input-synthesis paths — those are closed (cycle 3191 fact). Don't try xdotool / RFB / vncdotool again.
- **DO NOT commit tokens or credentials** to git. Anything in `secrets/` must be gitignored.
- **Sense-before-action**: read the current `/state` and `/screenshot.png` before any restart attempt — if zone is already non-null, the system is in a working state and your work is largely already done.
- **One worker per path**: you own this work alone. Other workers (albion-prod-login, albion-binary-recon, albion-magic-link) have been retired or are on different paths. Don't restart them.
- **Production daemons stay healthy**: cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest. If you must restart Albion-Online itself, use the same `LD_PRELOAD photon_tap.so` (-DDISABLE_SEND_HOOKS) wrapper invariant.

## Relevant files / references
- Live container: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Binary inventory (start here): `/home/sdancer/albion-binary-recon/analysis/binary_inventory.md`, `strings_global_metadata.txt`, `auth_keywords.txt`
- Token-keyword references: search `strings_global_metadata.txt` for `refreshToken`, `accessToken`, `AuthInvalidRefreshToken`, `'refreshToken'`, `'exchangecode'`, `'accountportal'`
- Unity prefs path: `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs`
- Existing persistence scripts: `/home/sdancer/albion-prod-login/scripts/persist_unity_state.sh`, `restore_unity_state.sh`
- mail.tm credentials (read-only): `/home/sdancer/albion-prod-login/secrets/mailbox.json`
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Memory pointers: `[[albion-2fa-container-rotation]]`, `[[albion-login-substrate]]`, `[[albion-send-hooks-break-client]]`, `[[albion-vastai-daemon-stack]]`, `[[orchestrator-role]]`, `[[no-frida]]`.

## Reporting
Concise progress at each task boundary. Milestone with verbatim `/state` JSON showing `self.zone != null` after a simulated container restart with NO user-driven 2FA = success signal. Anything partial: use "Achievement levels + gaps" framing — what IS achieved, what is open. Never declare goal-level "impossible" without an explicit level table.
