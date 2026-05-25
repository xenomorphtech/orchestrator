# albion-native-password-mode

## Role & workdir

Third parallel sibling on the `albion_action_loop` goal. The browser-based `accountportal` path is closed (CF Turnstile + 405 wall). The token-recovery sibling is surveying disk artifacts. Your job is the THIRD orthogonal angle: drive Albion's in-client `password` login mode that the metadata strings clearly document, completely bypassing the web/browser stack.

Workdir: `/home/sdancer/albion-native-password-mode` (git worktree, branch `albion-native-password-mode`).

## Already proven on goal (do NOT re-falsify)

| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L0 substrate alive | vast.ai container, Albion proc PID 549987 | game binary running at LoginScreen | ✅ |
| L_alt_a — captcha-vision-solve | `/home/sdancer/albion-captcha-vision-solve/analysis/post_submit_diagnosis.md` | browser path closed at CF Turnstile/`/login_check` 405 | ✅ falsified |
| L_alt_b — token-recovery (running) | `/home/sdancer/albion-token-recovery/analysis/unity_prefs_task1_2026-05-23.md` | Unity prefs has no JWT, only opaque login.hash | ⏳ task 2 in flight |
| **L_alt_c — native password mode** | YOUR DELIVERABLE | direct in-client login | ⏳ OPEN |

## Hypothesis

**H**: Albion's `global-metadata.dat` documents 4 login modes: `accountportal`, `exchangecode`, `password`, `refreshToken`. The `password` mode likely takes plain credentials and authenticates against Albion's auth backend directly — no browser, no CF gating. If we can flip the client to launch in `password` mode (via CLI flag, env var, or prefs key) AND supply credentials inline, we skip the entire web stack and Unity should accept and proceed.

## Current goal / sub-goal

- `goal_key`: `albion_action_loop`
- `sub_goal_key`: `native_password_mode_authentication`
- Success: dashboard `https://albion.orch.run/state` shows `zone != null` after Albion launches via `password` mode with the configured account.

## Falsification

If after 3 turns of investigation+experiment you cannot find a way to invoke `password` mode (no CLI flag, no env, no prefs key, no boot.config switch), OR if invoking it fails Albion's CF-equivalent backend check, this path is closed. Write `/home/sdancer/albion-native-password-mode/analysis/password_mode_blocked.md` and exit.

## Next 3 concrete tasks

1. **Discover the invocation mechanism for `password` mode.** Inspect:
   - `Albion-Online` binary's argv parsing (objdump/strings on `/home/albion/albion-online/Albion-Online`)
   - `global-metadata.dat` strings for command-line flags adjacent to the mode literals (`accountportal`, `password`, `exchangecode`, `refreshToken`)
   - boot.config / `/home/albion/albion-online/Albion-Online_Data/boot.config` for runtime mode toggles
   - environment variables / prefs keys the Albion startup code reads
   - launcher script `/usr/local/bin/run-albion-client` (read what flags it currently passes)
   
   Identify the EXACT mechanism that selects login mode. Document with file + line + literal flag-name.

2. **Construct + invoke a `password`-mode launch.** With the discovered mechanism:
   - Stop the current Albion via supervisor (PID 443749 is `albion-supervise`; SIGSTOP or SIGTERM the child Albion-Online PID 549987)
   - Modify boot.config / env / launch args to use `password` mode + inject credentials (read from `/home/albion/accountportal-headed/accountportal.env`)
   - Restart via supervisor and capture Player.log for login attempt
   - Look for SUCCESS lines like `LastKnownState: WorldEntry` instead of `LoginScreen`

3. **Verify zone non-null OR document the failure mode.** Poll `https://albion.orch.run/state` for 5 minutes (10s interval). If `zone != null` → fact-set `albion_native_password_mode_unblocked_2026-05-23` + announce. Else extract Player.log error lines, post-mortem to `/home/sdancer/albion-native-password-mode/analysis/password_mode_attempt_log.md`.

## Constraints & gotchas

- **DO NOT** echo Albion credentials in any output. Read them from `accountportal.env` into env vars; never log the values.
- **DO NOT** touch the browser-based driver `accountportal_login.py` — that path is closed-falsified.
- **DO NOT** kill `albion-supervise` (PID 443749) — only the Albion-Online child.
- **DO NOT** create new chromium browsers; this path is browser-free.
- The vast.ai container Albion install lives at `/home/albion/albion-online/`. Unity Player.log lives at `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/Player.log`.
- **Concurrent sibling path**: `token-recovery` is doing a disk-cache survey. Don't step on its files; you should be inspecting binary internals + boot config, which is orthogonal.

## Multi-instance scaling note

The user asked for ≥3 parallel workers. If this path succeeds with one account, the natural extension is multi-account → multi-Albion. That requires additional Albion accounts (resource-ask from user). Note this in your verdict if you reach a working state.

## Relevant files / references

- Albion binary: `/home/albion/albion-online/Albion-Online`
- Game assembly: `/home/albion/albion-online/Albion-Online_Data/Managed/Assembly-CSharp.dll`
- Boot config: `/home/albion/albion-online/Albion-Online_Data/boot.config`
- Launcher script: `/usr/local/bin/run-albion-client`
- Supervisor: `/usr/local/bin/albion-supervise`
- Creds env: `/home/albion/accountportal-headed/accountportal.env` (DO NOT echo)
- Closed sibling diagnosis: `/home/sdancer/albion-captcha-vision-solve/analysis/post_submit_diagnosis.md`
- Token-recovery sibling (concurrent): `/home/sdancer/albion-token-recovery/`
- Dashboard: `https://albion.orch.run/state` for `zone != null`
- Substrate access: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`
