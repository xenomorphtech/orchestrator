# albion-tutorial-dodge-armed-cast — H10b compound-mechanism revival

## URGENT — 25-min cap, compound mechanism hypothesis

H10 (bare F-keypress) was mechanism-dropped 2026-05-25 ~16:00Z after worker tried 3 input-injection siblings (xdotool key F, xdotool focus+keydown/keyup, ydotool uinput). Adversarial-pair enumeration completed 16:06Z, recommended **Class B #1** as top untried mechanism:

> `xdotool key f; sleep 0.2; mousemove 960 600; click 1` — F arms the boot-ability targeter, LEFT-click on ground commits the Dodge in that direction.

Adversarial output: `/home/sdancer/albion-tutorial-dodge-adv-enum/analysis/albion-tutorial-dodge_adversarial_alternatives.md`.

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 2-6 | H2/H5/H7/H8/H9 | proximity-gate / LEFT-click / locomotion | ✅ |
| 7 bare-F | H10 (mechanism-dropped) | xdotool key f, ydotool uinput | mechanism-dropped — bare keypress class exhausted on its own |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-dodge-armed-cast`** (branch `tutorial-dodge-armed-cast`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_dodge_advanced`
- **success metric**: quest text changes from "Use the Dodge" to next step.

## Hypothesis
Albion's Dodge spell is a **directional boot ability**. Pressing F alone ARMS a targeter on the cursor (similar to other Albion skills — H8 verdict noted "F at NPC armed a combat ability earlier"). The cast COMMITS only on a second action: a LEFT-click on the ground at the desired dodge direction, OR a movement key, OR locomotion already in motion.

This is a DISTINCT path from H10. H10 falsified bare-keypress class. This path tests **compound (key + commit) class**.

## Substrate state (verified ~16:08)
- Veldra1203 at (-13.25, -65.25) zone "The Lighthouse"
- Quest panel: "Use the Dodge" (unchanged across H10 attempts)
- Action-loop STILL STOPPED — keep stopped
- /state path: `http://127.0.0.1:8765/state`

## Tasks (25-min budget)

### T0 (~2min) — Baseline
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline.
3. Touch heartbeat.

### T1 (~10min) — Class B compound mechanisms (in order)
Use this xdotool incantation pattern: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool <cmd>'`.

Try each in order. Screenshot + /state diff after each. STOP on first success.

1. **B#1 (top recommendation)**: `xdotool key f` then 200ms sleep then `mousemove 960 600 click 1` (LEFT-click screen center, ground).
2. **B#3 (movement-key commit)**: `xdotool key f` then 150ms sleep then `key Left` (arrow-key directional commit).
3. **B#4 (double-tap F)**: `xdotool key f` then 150ms sleep then `key f` (release/cancel pattern).
4. **B#5 (locomotion-first)**: `xdotool mousemove 1080 640 click 3` (start moving), then 200ms sleep then `key f` (fire Dodge mid-motion).
5. **B#2 (right-click commit)**: `xdotool key f` then 200ms sleep then `mousemove 960 600 click 3`.

### T2 (~5min) — Fallback Class C if all B fail
Try one of:
- vncdo at port :3 with compound: `vncdo -s localhost:3 key f sleep 0.2 click 1 960 600` (RFB protocol path).
- raw XSendEvent KeyPress/KeyRelease for F (different from XTest).

### T3 (~3min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_dodge_advanced "<recipe>; artifact analysis/tutorial_dodge_armed_cast_verdict_2026-05-25.md"` + verdict file.
- **Failure**: commit `analysis/tutorial_dodge_armed_cast_blocked_2026-05-25.md` with mechanism-scoped closure + ≥3 untried siblings.

## Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- Click coordinate (960, 600) is screen-center (1920x1080) where ground is visible. If you see UI/HUD there, pick a clear ground pixel slightly offset.
- B#5 is gentlest — starts movement first, so if F-cast-mid-motion is correct mechanism, B#5 commits naturally.

## Memory references
- `[[falsify-mechanism-not-path]]`
- `[[albion-tutorial-step-advance-recipe]]`
- `[[npc-drift-contamination]]`
- `[[unity-real-press-required]]` — Unity input often needs separate mousedown/mouseup; if `click 1` is inert, try `mousedown 1 → sleep 150 → mouseup 1`

## Files / endpoints
- Adversarial enumeration: `/home/sdancer/albion-tutorial-dodge-adv-enum/analysis/albion-tutorial-dodge_adversarial_alternatives.md`
- Original H10 verdict: `/home/sdancer/albion-tutorial-dodge/analysis/tutorial_dodge_blocked_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-dodge-armed-cast/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_dodge_advanced`
