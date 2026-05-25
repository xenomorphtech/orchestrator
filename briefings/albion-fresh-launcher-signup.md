# albion-fresh-launcher-signup — turn-8: xev observation surface

## Role & workdir
Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-fresh-launcher-signup`. Substrate: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`. Thread continues from turn-7.

## Already achieved (do NOT re-falsify)

| Level | Artifact | Status |
|---|---|---|
| L1 | Production PID 688424 holding zone=YEKZMHL on `:3` | ✅ DO NOT TOUCH |
| L7 | `analysis/launcher_form_submit_2026-05-24.md` | Page-1 SOLVED; page-2 password TMP_InputField rejects 6 xdotool variants |
| L8 | `analysis/launcher_cli_probe_2026-05-24.md` | No CLI bypass flags — Path2A FALSIFIED |
| L9 | `analysis/launcher_path2b_not_attempted_2026-05-24.md` | Path2B (LD_PRELOAD) retired |
| L10 | `analysis/launcher_path2c_blocked.md` | uinput EPERM on container — Path2C BLOCKED |
| L11 | `analysis/launcher_log_blocked.md` | Player.log SILENT during page-2 input cycle BUT framebuffer DID change (focus/hover altered). Input reaches UI, characters don't persist. Path2D FALSIFIED |

## Critical new insight (from turn-7)

Page-2 input is NOT being dropped at the X server or window-manager layer. The Albion window receives focus/hover changes during xdotool input. The rejection happens **inside the Unity TMP_InputField with `ContentType.Password`** — the field accepts focus, accepts mouse, but discards typed characters from the synthesized input path.

Page-1 email field (`ContentType.EmailAddress` or `Standard`) accepts the same xdotool input. The distinction is the `ContentType.Password` field type itself.

## Current goal / sub-goal
- **goal_key:** `albion_action_loop`
- **sub_goal_key:** `fresh_accounts_via_desktop_client_signup`
- **Success metric:** `secrets/account_1.json` mode 600 with `{email, password, mailbox_jwt, confirmed_at_iso}` + verified post-confirmation login reaching at least character-select.

## Pivot hypothesis (turn-8 — Path2E, observational xev capture)

The X server receives keyboard events for the password field but Unity's TMP_InputField filter discards them. To confirm this and discover the field's actual input source, **run `xev -id <WID> -event keyboard` on the Albion window** while xdotool fires page-1 vs page-2 input. Compare the KeyPress/KeyRelease event traces. If page-1 gets `KeyPress xK_e` events but page-2 doesn't (or gets a different event class), we've found the filter point.

### Concrete plan (Path2E — xev keyboard-event capture)

1. **Install `xev`** if not present: `apt install x11-utils` (probably already there).

2. **Setup paired captures** on `DISPLAY=:4`:
   - Find Albion WID: `WID=$(xdotool search --onlyvisible --name Albion | head -1)`
   - Page-1 case (proven-working email field):
     - Navigate the launcher to the page-1 Email field (re-do reopen+trim+Tab pattern if needed)
     - Start: `xev -id $WID -event keyboard 2>&1 > /tmp/xev_page1.log &`
     - `xdotool type --window $WID "testdummy"`
     - Stop xev, save log
   - Page-2 case (failing password field):
     - Navigate to page-2 (proven recipe: trim+Tab+paste+forward-arrow on page-1)
     - Click into Password field
     - Start: `xev -id $WID -event keyboard 2>&1 > /tmp/xev_page2.log &`
     - `xdotool type --window $WID "testdummy"`
     - Stop xev, save log

3. **Diff the two logs**:
   - `diff /tmp/xev_page1.log /tmp/xev_page2.log | head -40` — sanitized (no real password)
   - Save findings to `analysis/launcher_xev_findings_2026-05-24.md` with:
     - Page-1 event count + first 5 events
     - Page-2 event count + first 5 events  
     - Diff summary (e.g. "page-1 receives 9 KeyPress/Release pairs; page-2 receives 0 KeyPress events but 1 FocusIn")
     - Verdict: which layer is filtering

## Falsification (HARD CONTRACT)

Path2E falsified if:
- `xev` shows IDENTICAL KeyPress event sequences for page-1 and page-2 (would mean the filter is purely inside Unity, undiscoverable from X side), OR
- `xev` cannot attach to the Albion window (XGrabKeyboard race, EPERM, etc.)

If falsified: commit `analysis/launcher_xev_blocked.md` with sanitized counts + verdict.

## Reusable assets
- mail.tm bundle: `secrets/mailbox_1.json` mode 600
- account state: `secrets/account_1_pending.json` mode 600

## Trajectory regulation
- 15min heartbeat if no artifact.

## Side-channel abort
- Each iteration: `test -f /tmp/abort_albion-fresh-launcher-signup`.

## Constraints
- **NEVER touch PID 688424.**
- **NEVER echo creds to logs/chat.** Use dummy string `testdummy` for the xev test; never the real alnum password.
- Pure read-only X-event observation. No injection.

## Relevant files
- turn-7 verdict: `analysis/launcher_log_blocked.md`
- turn-7 path inventory: `analysis/launcher_log_paths_2026-05-24.md`
- Sandbox HOME: `/tmp/albion_clean_test/`
- SSH: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`
- Dashboard: `https://albion.orch.run/state` — production zone must remain `YEKZMHL`
