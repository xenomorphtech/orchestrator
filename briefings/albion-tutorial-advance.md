# albion-tutorial-advance — turn-3 NPC interact (xdotool LEFT + key-e falsified)

## URGENT — 20-min turn, commit-or-falsify enforced
This is turn-3. Two prior turns spent 75+min on navigation + reacquisition + caution-loop. Direct orchestrator probes already falsified 2 mechanisms (`xdotool LEFT real-press at (432,687)` and `xdotool key e`). Veldra is ADJACENT to the survivor NPC, already in click range — you do NOT need to navigate.

**Hard rule: if no quest tick by minute 18, write `analysis/tutorial_advance_partial_2026-05-25.md` (do not exit silently). Touch `/tmp/heartbeat_albion-tutorial-advance` every 5 min so orchestrator can see liveness.**

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-advance`** (branch `tutorial-advance`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_talk_to_survivor_advanced`
- **success metric**: quest panel text changes from "First Steps — Talk to the survivor (0/1)" to next quest step. Capture before/after screenshots as proof.

## Substrate truth (just verified by orchestrator at 10:39)
- Veldra1203 at world (7.75, -44.25) in zone The Lighthouse
- Survivor NPC visible at native ~(430, **730–790**) — y was probably too high in prior probe (clicked the `?` icon above the sprite, not the body)
- Watchdog in_zone HB current (08:39:34Z)
- /state x/z recovered from earlier mask drift — decoder healthy

Reference screenshot: `/tmp/orch_post_e_1035.png` (1920×1080). The NPC `?` mark + white quest ring is bottom-left of frame. NPC body sprite is below the `?`.

## Mechanism class & ALREADY FALSIFIED

**Class**: input-injection to advance Albion NPC dialog/interact.

| Mechanism | Falsified by | Notes |
|---|---|---|
| `xdotool mousedown 1/150ms/mouseup 1` at (432, 687) | orchestrator 10:34 | Probably clicked above sprite (y too low) — RE-TRY at y=750 first |
| `xdotool key e` | orchestrator 10:35 | No quest tick. Albion may not bind interact to e. |

## Untried mechanisms (THIS turn, in priority order)

1. **xdotool LEFT real-press at LOWER coord (432, 750)** — easy retest of mechanism #1 with corrected y. If quest ticks, we're done.
2. **xdotool LEFT real-press at multiple y bands**: 730, 760, 790 — bracket the sprite hitbox.
3. **xdotool key f** (interact key in some games)
4. **xdotool key space** (advance dialog in many games)
5. **vncdo (or vncdotool) RFB pointer left-click** — `pip install vncdotool` then `vncdo -s :3 click left X Y`. Bypasses X server input filter via raw RFB protocol.
6. **`ydotool click 0xC0`** if ydotool is available (uses /dev/uinput kernel-side, invisible to XTest filters)
7. **Double-click LEFT** — Albion may need double-click on NPC.

## Next concrete tasks (~20min)

### T0 (~3min) — Fresh screenshot + pre artifact
1. Fresh `:3` screenshot to `analysis/t3_pre.png` via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-advance/analysis/t3_pre.png'` (absolute path; no `runuser` cwd surprises)
2. Inspect — confirm NPC still bottom-left and Veldra adjacent. If NPC moved out of frame, ONE right-click at (220, 250) to re-reveal (and ONLY one — do not enter the prior reacquisition recursion).
3. Touch `/tmp/heartbeat_albion-tutorial-advance`.

### T1 (~8min) — Run the priority-1..7 mechanism sequence
For each mechanism in the list, capture pre/post screenshots and grep quest panel area for text change. 1 attempt per mechanism = 7 attempts max in this phase. As soon as ONE mechanism produces a visible quest panel diff or dialog overlay → STOP, proceed to T2.

Mechanism dispatch templates:
- LEFT at (X,Y): `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool mousemove X Y mousedown 1; sleep 0.15; DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool mouseup 1'`
- Key: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool key <name>'`
- vncdo: `pip install --user vncdotool && vncdo -s localhost:3 click 1 X Y` — note: VNC may need password; check if Xtigervnc :3 is accessible (likely no password since same machine).
- ydotool: check `which ydotool && systemctl status ydotool` first; if not installed, skip.

Touch `/tmp/heartbeat_albion-tutorial-advance` after each mechanism attempt.

### T2 (~5min) — Verdict + fact + restart action-loop
- **If success**: `analysis/tutorial_advance_verdict_2026-05-25.md` with mechanism + coord + pre/post sha256 + quest panel text diff. Set fact `albion_tutorial_step_talk_to_survivor_advanced`.
- **If 7/7 fail**: `analysis/tutorial_advance_partial_2026-05-25.md` mechanism-scoped (each one named + falsification reason). Enumerate ≥3 still-untried mechanisms: Frida hook on Unity NPC.OnPointerClick, evdev raw HID, AT-SPI a11y, LD_PRELOAD libX11.
- **Always**: restart action-loop on exit: `XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus systemctl --user start albion-action-loop`. Confirm `is-active`.

## Commit-or-falsify contract (HARD)
- 20min hard cap. **No exceptions.** If no verdict by minute 18, write partial.md (which only takes ~1min).
- 5min heartbeat: `touch /tmp/heartbeat_albion-tutorial-advance`. Orchestrator monitors this file; if mtime stalls >7min, force-stop.
- `/tmp/abort_albion-tutorial-advance` → commit partial + exit IMMEDIATELY. **You will be force-killed at next orchestrator tick if you ignore /tmp/abort.**

## Constraints (HARD — unchanged from prior turn)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- You MAY temporarily stop albion-action-loop.service if it's running; restart on exit.
- NEVER spam Escape.
- xdotool MUST go through `sudo runuser -l sdancer -c '...'`. Direct xdotool from worker context will fail.
- Screenshots: ALWAYS absolute paths inside sudo-runuser commands (relative paths break per prior turn's bug).
- **The prior 75min was caution-loop. Don't recurse on "one more verify before commit". COMMIT FIRST, observe after.**

## Memory references
- `[[falsify-mechanism-not-path]]`
- `[[unity-real-press-required]]`
- `[[albion-tutorial-clickclass]]`
- `[[macromanage-workers]]`

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-advance.md`
- Worktree: `/home/sdancer/albion-tutorial-advance/`
- Prior NPC location notes: `analysis/t1_npc_location.txt`
- Prior pre/post: `analysis/t2_pre_click.png`, `analysis/t2_post_click.png`
- Orchestrator probe screenshots: `/tmp/orch_pre_click_1034.png`, `/tmp/orch_post_click_1034.png`, `/tmp/orch_post_e_1035.png`
- /state: `http://127.0.0.1:8765/state`
- Harness: `/home/sdancer/orchestrator/harness`
- Heartbeat file: `/tmp/heartbeat_albion-tutorial-advance` (you create + touch)
- Abort file: `/tmp/abort_albion-tutorial-advance` (orchestrator may touch)
