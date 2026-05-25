# albion-tutorial-dodge — H10: tutorial step 7 'Use the Dodge'

## URGENT — 15-min cap, simple keybind quest
H9 succeeded ~15:41 advancing quest "Go back down the stairs." → **"Use the Dodge"** (visible in tooltip: "Use the spell on your boots by pressing [Shortcut: F]"). This is the SIMPLEST quest yet — just press F.

## Already achieved
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 2-6 | H2/H5/H7/H8/H9 | proximity-gate + LEFT-click + locomotion-descent | ✅ All done |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-dodge`** (branch `tutorial-dodge`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_dodge_advanced`
- **success metric**: quest text changes from "Use the Dodge" to next step.

## Hypothesis
Press F key. Tooltip says "[Shortcut: F]". The Dodge spell is on Veldra's boots. This is a key-press quest, NOT a click or locomotion quest.

## Substrate state (verified ~15:43)
- Veldra1203 at (-13.25, -65.25) on stone path/staircase
- Quest panel: "Information is Key" / "Use the Dodge"
- Tutorial tooltip visible top-center: "Dodge — Use the spell on your boots by pressing [Shortcut: F]"
- Action-loop STILL STOPPED

## Tasks (15-min budget)

### T0 (~2min) — Baseline
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline.
3. Touch heartbeat.

### T1 (~5min) — Press F
1. `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool key f'`
2. Wait 3s.
3. Screenshot + check quest text.
4. If inert: try `xdotool key --repeat 2 f` or F1, or capital F.

### T2 (~3min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_dodge_advanced "F key press advanced quest from 'Use the Dodge' to <new>; artifact analysis/tutorial_dodge_verdict_2026-05-25.md"`.
- **Failure (rare)**: commit blocked.md with siblings: `xdotool keydown f; sleep 0.2; keyup f` (real press), `ydotool key 33` (raw uinput), Shift+F, etc.

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- F at NPC armed a combat ability earlier (per H8 verdict). But here, F IS the intended action — let combat-ability-armed fire (it's the Dodge).

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-dodge.md`
- Worktree: `/home/sdancer/albion-tutorial-dodge/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_dodge_advanced`
