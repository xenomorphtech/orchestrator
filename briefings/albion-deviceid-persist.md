# albion-deviceid-persist — eliminate 2FA prompt via Albion DeviceId persistence

## Role & workdir
Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-deviceid-persist`. Live target: vast.ai `ssh -p 14838 root@ssh8.vast.ai`. Albion runs as user `albion` on `DISPLAY=:3`.

## Current goal / sub-goal
- `goal_key`: `albion_action_loop`
- success = `https://albion.orch.run/state` shows `zone.name != null` (currently null, events_processed=830 flat 18 ticks)
- **This path's sub-goal**: ensure the Albion 2FA "new device" prompt does NOT fire on future sessions, by persisting Unity prefs `DeviceId` across container rotations.

## Why this path now
The sibling worker `albion-prod-login` has been blocked at the 2FA modal input dispatch for 50+ minutes (c3848 spawn through c3859). Empirical finding: Albion's 2FA modal field appears to reject XTEST keystroke synthesis (xdotool type AND ctrl+v paste both produce empty fields per turn58_root_after_type.png, turn58_manual_filled.png). This blocks the immediate session.

**Parallel solution**: if DeviceId persists across Albion restarts, the 2FA modal simply doesn't fire on subsequent launches. The user can do a one-time manual paste of THIS session's 2FA code OR we wait for prod-login to find a working input method — either way, the DeviceId-persistence mechanism you build prevents recurrence.

## Hypothesis
DeviceId is stored in `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` (per fact `albion_no_saved_creds_2026_05_22`). If we (a) copy the current DeviceId to a host-side persistent location, AND (b) restore it on every Albion-supervise launch before Albion-Online spawns, AND (c) the egress IP is sufficiently stable, then the 2FA modal will not fire on subsequent sessions.

## Falsification
After implementing the persistence mechanism and restarting Albion (NOT this session — wait for prod-login to complete or fail first), the 2FA modal STILL fires. Possible causes if falsified: (a) DeviceId is part of a wider trust signature including egress IP — verify the IP is stable across container restarts; (b) Albion has additional device-trust fields beyond just DeviceId — inspect the prefs file holistically; (c) the prefs file is signed/integrity-checked — diff before/after restoration for canary fields.

## Already achieved (from prior facts)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | Fact `albion_no_saved_creds_2026_05_22` | prefs file at known path contains DeviceId=411a9ed8-... (now reset to new value post-c3839 supervisor swap) | ✅ DONE |
| 2 | Path P scope already documented in `albion-prod-login` briefing | Plan exists | ✅ DONE |
| 3 | Memory `[[albion_2fa_container_rotation]]` | 2FA fires on DeviceId+egress IP reset | ✅ DONE |

## Tasks (in order)
1. **Inspect current prefs.** SSH to vast.ai. `cat /home/albion/.config/unity3d/Sandbox\ Interactive\ GmbH/Albion\ Online\ Client/prefs` and report the structure (relevant lines only, NOT raw content to JSONL). Identify ALL fields that look device-trust-related (DeviceId, LastKnownState, login.auto.*, any UUID-looking values).
2. **Locate a persistent host-side store.** vast.ai containers reset on rotation, but `/root/` outside the container, or a vast.ai-managed persistent volume, survives. Find a path. Likely options: `/root/albion-persist/`, or a vast.ai "persistent storage" mount if available.
3. **Implement copy-on-write persistence.**
   - On Albion-supervise STOP (or via systemd `ExecStartPre`): copy current prefs file to `<persistent>/prefs.last`
   - On Albion-supervise START: if `<persistent>/prefs.last` exists, copy it back to the in-container prefs path BEFORE Albion-Online launches, with correct ownership (`chown albion:albion`)
4. **Hook into the existing `albion-supervise` wrapper.** This wrapper is at `/usr/local/bin/albion-supervise` (or similar). Modify it to call the persistence hooks. Do NOT break the existing photon_tap.so + keytap_int3.so preload chain (sibling worker `albion-photon-sdk-research` turn-57 depends on it).
5. **Smoke test** (only after `albion-prod-login` releases its session OR with explicit user permission — do NOT kill PID 523248 unilaterally). Stop Albion-Online, verify prefs is copied to persistent store. Start Albion-Online, verify prefs is restored. Take a screenshot at the next session-launch and verify the 2FA modal does NOT fire (i.e., screen advances directly to character-select OR LastKnownState screen).
6. **Verdict.** Write `analysis/turn1_deviceid_persist_2026-05-23.md`: persistence mechanism details, fact set with key persisted, post-restart screenshot path, 2FA-fired-or-not observation. Set fact `albion_deviceid_persisted` with the mechanism path.

## Constraints & gotchas
- **Do NOT kill Albion-Online PID 523248** (the in-flight session being driven by prod-login). Work on the persistence mechanism, but the smoke-test restart waits for the current session to terminate naturally OR for explicit handoff.
- **Do NOT modify `/opt/albion-frida-capture/spawn_preload.sh`** — turn-57 depends on it. The DeviceId-persist hooks go in `/usr/local/bin/albion-supervise` (or a systemd unit), NOT in spawn_preload.sh.
- **Do NOT log raw DeviceId UUID to JSONL/git/talk** — it's device-trust material. Refer to it by SHA256 if needed for verification.
- **Per memory `[[no_frida]]`**: no Frida on libUnreal.so. Albion is Unity (libUnreal isn't loaded), but stay clear of Frida attach on Albion-Online process to avoid anti-debug.
- **Per `[[albion_2fa_container_rotation]]`**: device trust = DeviceId + egress IP. If THIS container's egress IP changes on next rotation, persisting only DeviceId won't be sufficient. Verify IP stability separately.

## Success criteria
1. Persistence mechanism implemented + tested (prefs copy-out and copy-back work).
2. Albion launches AFTER persistence is active SKIP the 2FA modal (LastKnownState=LoginScreen rather than fresh-device-prompt).
3. Fact `albion_deviceid_persisted` set with mechanism description.
4. No regression to photon_tap.so / keytap_int3.so / frida-ingest / dashboard.

## Relevant files / references
- Prefs path: `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs`
- Supervisor wrapper: `/usr/local/bin/albion-supervise` (run `cat` to read)
- Memory: `[[albion_2fa_container_rotation]]`, `[[albion_no_saved_creds_2026_05_22]]`, `[[albion_vastai_daemon_stack]]`
- Dashboard verification: `curl https://albion.orch.run/state`

## Reporting
End-of-turn verdict at `analysis/turn1_deviceid_persist_2026-05-23.md`. Include: inspection findings, persistence mechanism code, smoke-test screenshot before/after, 2FA fire/no-fire observation.
