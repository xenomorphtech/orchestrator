# albion-action-emitter — Drive Albion UI from current state to zone non-null

## Role & workdir
Codex worker (codex_app_server, durable thread). Workdir: `/home/sdancer/albion-action-emitter`. Live host: vast.ai (`ssh -p 14838 root@ssh8.vast.ai`) — Albion runs as user `albion` on `DISPLAY=:3` under Xtigervnc.

## Current goal
- `goal_key`: `albion_action_loop`
- success = https://albion.orch.run/state shows `zone.name != null` (currently null; `events_processed=830` post-respawn — fresh Albion child from c3839 supervisor swap is in startup/login-screen window)
- The Albion process WAS respawned at ~13:08 CEST under the new persistent keytap preload (c3839 supervisor swap by sibling worker). It is somewhere in startup → login UI → character-select. Driving it through into the world completes the goal.

## Turn-58 framing (READ THIS FIRST)
The sibling worker `albion-photon-sdk-research` just landed turn-57 verdict (`/home/sdancer/albion-photon-sdk-research/analysis/turn57_continuous_decrypt_2026-05-23.md`): **the encrypted Photon decode pipeline is now LIVE and durable**:
- `/opt/albion-frida-capture/preload/albion_keytap_int3.so` persistent LD_PRELOAD captures a fresh AES key on every Albion session
- `albion-photon-decrypt` systemd-supervised daemon writes `/var/log/albion-frida-decrypted.jsonl`
- `albion-zone-extractor` systemd-supervised daemon POSTs zone candidates to dashboard `/state`
- Verdict line: *"AES/session decrypt is no longer the blocker. Reaching or observing actual zone-bearing gameplay messages is the blocker."*

**Your job is no longer to decrypt or capture anything.** It is purely to drive the Albion UI past whatever screen it's currently on into a populated zone, then the live pipeline will observe + POST zone-bearing traffic automatically.

## Substrate facts (PRE-VERIFIED — do NOT re-test)
| Fact | Substrate | Source |
|---|---|---|
| Albion process alive, ~17h uptime | vast.ai pid 460039 | c3674 audit |
| Window "Albion Online Client" on `DISPLAY=:3` | xtigervnc :3 | c3674 `xdotool search` |
| Xtigervnc :3 ACCEPTS XTEST input incl. TMP_InputField | [[albion-login-substrate]] | cycle 3151 breakthrough |
| Xkasmvnc :2 REJECTS TMP_InputField XTEST — use :3 only for input | [[albion-login-substrate]] | cycle 3151 |
| xdotool DOES reach Albion — Escape-spam-wedges-quit-dialog was OWN sabotage | [[xdotool_unity_albion_blocked]] | retracted cycle 3132 |
| Tutorial quest events gate on LEFT-clicks (right-click doesn't tick the quest) | [[albion-tutorial-clickclass]] | c2305-2479 |
| `xwd -root` returns single-color on :3 — Unity-VNC framebuffer quirk, screenshot via different mechanism | c3674 | observation |

## Hypothesis
Driving the existing supervised Albion through whatever screen it's currently on → world entry → zone.name becomes non-null. xdotool against `DISPLAY=:3` is the input dispatcher.

## Falsification
After 3 supervised input cycles with screenshot-confirmed input-receipt and policy match, `state.zone` remains null AND screen state classifier reports no transition.

## Success criteria
1. `https://albion.orch.run/state` returns `zone.name != null` (the actual goal)
2. Audit JSONL records at least one classified screen state with successful input dispatch and subsequent state transition observed
3. No regression: photon_tap.so + frida-ingest + gamestate_service stay up

## Task 1 — One-shot reconnaissance (CHEAPEST first; do this before code)
On vast.ai as user `albion`:
- Take a clean screenshot of Albion via a working mechanism (NOT `xwd -root` — that's blank on :3 per c3674). Try `xdotool getactivewindow` then `xwd -id <wid>` piped to `xwdtopnm | pnmtopng`. Alternative: search `/home/sdancer/albion-photon-sdk-research/analysis/turn3[5-9]_display3_*.png` — those 2.1MB PNGs are from a working capture method; reuse it (look at how that worker captured screens).
- Classify the visible screen: { login_screen, intro_video, 2fa_dialog, character_select, loading, in_zone, quit_dialog, unknown }.
- Write the classification + screenshot path into `analysis/reconnaissance_2026-05-23.md` BEFORE writing any input code.

This is one-shot — no policy loop yet. Just see what state Albion is actually in.

## Task 2 — Implement screen-state classifier + one-shot dispatcher
Given task-1 verdict, implement minimal flow to advance ONE step.

Examples (pick based on task-1 finding):
- intro_video → ONE Escape key (NOT periodic; NOT looped) targeted at the Albion window via `xdotool windowfocus --sync $WIN && xdotool key Escape`
- character_select → LEFT click on a character entry (find via screenshot template / pixel sampling), then click "Enter World"
- 2fa_dialog (code from email) → STOP, do not attempt; this means container-IP rotated. Read [[albion-2fa-container-rotation]] and report it; will be handled externally.
- login_screen with credentials needed → see Creds section below
- loading → wait 10s, re-screenshot, loop until state changes
- quit_dialog → click "No" to dismiss (cycle-3125 pixel coords: native 1920×1080, "No" at (1230, 652), "Yes" at (713, 652) — but verify via fresh screenshot first; DO NOT trust stale coordinates blindly)

The existing `emit.py` + `policy.py` codebase targets DISPLAY=:2 (wrong) and has no login_flow branch. Either: extend `policy.py` with branches { intro_video, character_select, login_screen } each emitting ONE input, OR write a new lightweight `login_driver.py` that drives one step per invocation and exits. Either is fine — choice is yours; bias toward simple-and-readable.

## Task 3 — Verify goal
After task-2 dispatches an input, wait 5-10s, re-screenshot, re-classify, confirm STATE TRANSITION (not just dispatch ok). Curl https://albion.orch.run/state and check `zone.name`. If still null but state changed, queue next dispatch. Stop when zone non-null OR after 5 dispatches with no progress.

## Constraints & gotchas
- **NEVER spam Escape** — it toggles Albion's quit-confirmation dialog (cycle 3132 self-sabotage lesson). Each Escape press = state change, not no-op.
- **Use DISPLAY=:3, NOT :2** — :2 rejects TMP_InputField (the login text fields)
- **Use `runuser -l albion -c ...`** to dispatch as the user that owns the Albion window. Root xdotool to :3 may need explicit XAUTHORITY=/home/albion/.Xauthority.
- **Left-click for tutorial floor**, not right-click (per [[albion-tutorial-clickclass]])
- **Sibling worker (albion-photon-sdk-research) is now DORMANT after turn-57.** The persistent preload + decrypt daemons are live. Do not modify `/opt/albion-frida-capture/spawn_preload.sh` or the systemd-supervised tmux sessions `albion-photon-decrypt` and `albion-zone-extractor`. You may read `/var/log/albion-frida-decrypted.jsonl` to verify the pipeline sees your driven traffic, but do not restart Albion via supervisor — it was JUST respawned in c3839; let it complete startup naturally. Find the current Albion PID via `pgrep -u albion Albion-Online`, not stale memory.
- **Do NOT use `xdotool key --window <wid>` for Albion** — use `xdotool windowfocus --sync <wid>` then `xdotool key` without `--window` (active-window XTEST is the model Unity accepts; cycle 3151's Level-18 fix)
- **/dev/uinput does not exist** in this container; CAP_SYS_ADMIN missing. Cannot create uinput. xdotool path is the actual answer. Do NOT pursue uinput or RFB-over-WSS.

## Credentials policy
If login screen IS detected with empty fields: the email is `5fswkv6zf4@wshu.net`, password `albion260518q9`. These are committed to the briefing intentionally; type them via `xdotool type --delay 50 ...` directed at the focused TMP_InputField (per [[albion-login-substrate]] Xtigervnc :3 accepts XTEST against text inputs). NEVER log/echo these creds to JSONL, git, or the talk channel. If 2FA code is requested → STOP and report — that means container IP rotated and `[[albion-2fa-container-rotation]]` workaround is needed (separate path).

## Relevant files / references
- Existing codebase: `/home/sdancer/albion-action-emitter/{emit.py, policy.py, uinput_dispatcher.py, config/, tests/}`
- Screenshots from prior worker (working capture mechanism): `/home/sdancer/albion-photon-sdk-research/analysis/turn35_screen[1-3]_2026-05-23.png` and `turn38_display3_2026-05-23.png`, `turn39_display3_2026-05-23.png` (2.1MB each — that capture method works)
- Worker turn-40 in flight (no conflict): `/home/sdancer/orchestrator/analysis/codex-sessions/albion-photon-sdk-research/20260523T063448Z/`
- Dashboard: `https://albion.orch.run/state` — your verification endpoint
- Photon ingest session log: `/var/log/albion-frida-sessions/session-*.jsonl` on vast.ai (zone-entry would show in these)
