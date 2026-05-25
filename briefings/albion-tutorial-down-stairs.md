# albion-tutorial-down-stairs — H9: tutorial step 6 after bridge-enemies success

## URGENT — 30-min cap, proven recipe
H8 succeeded ~15:19 advancing quest "Check the bridge for enemies." → **"Go back down the stairs."** via proximity-gate (no enemy clicks needed).

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe variant | Runtime | Status |
|---|---|---|---|---|
| 2 Talk-to-survivor | H2 | LEFT-click on NPC sprite | 23min | ✅ |
| 3 Climb-tower | H5 | Locomotion-only at top perch | 34min | ✅ |
| 4 Look-for-ship | H7 | Proximity-gate at ring/platform | 6min | ✅ |
| 5 Check-bridge-enemies | H8 | Proximity-gate near HERETICS marker | ~9min | ✅ |

Recipe is empirically: **proximity-gate works for visual-objective steps; LEFT-click only required for NPC-interaction steps.**

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-down-stairs`** (branch `tutorial-down-stairs`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_down_stairs_advanced`
- **success metric**: quest text changes from "Go back down the stairs." to next step.

## Hypothesis
"Go back down the stairs" likely = walk player back DOWN the lighthouse staircase to lower-elevation area (south, away from current LIGHTHOUSEUP2/HERETICS region back toward original Crafter spawn at ~(7.59, -67.25)).

Apply recipe: **proximity-gate first** (walk down stairs, hold), escalate to LEFT-click only if proximity inert.

## Substrate state (verified 15:19)
- Veldra1203 at (-2.73, -37.00) near HERETICS marker
- Quest panel: "Go back down the stairs."
- Action-loop STILL STOPPED
- /state may temporarily go null on capture-service restart — fall back to screen

## Probable trajectory
Reverse H5's path: SE+down. Player started at (7.59, -67.25) in original Crafter area. Came UP via stairs in H5. Now need to DESCEND.

H5 path was: (7.59,-67.25) → (-19.23,-50.30) climbing northwest.  
Reverse for H9: from (-2.73,-37.00) → back to ~(7.59,-67.25) descending southeast.

## Tasks (30-min budget)

### T0 (~3min) — Baseline + identify staircase
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline.
3. Visually identify staircase going DOWN (reverse direction of H5 climb).
4. Touch heartbeat.

### T1 (~15min) — Descend via right-click ground-moves
1. Right-click on visible lower stair planks / descending path. Aim SE.
2. Re-screenshot every 2-3 moves. Poll /state for x going positive and z going more negative.
3. Hold periodically (10s) for proximity-gate test (recipe step 6a).
4. Target: get within 5 world-units of any lower-elevation marker (Crafter area or FORGE(4.5,-51.5) or LIGHTHOUSEDOWN(-13.5,-60.5)).

### T2 (~7min) — If proximity inert, try LEFT-click on staircase
1. LEFT-click visible stair-floor pixels.
2. Try 3-5 distinct coords.

### T3 (~5min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_down_stairs_advanced "<mechanism>; artifact analysis/tutorial_down_stairs_verdict_2026-05-25.md"`.
- **Failure**: commit `analysis/tutorial_down_stairs_blocked_2026-05-25.md` mechanism-scoped.

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — validated 4 steps; recipe with proximity-first
- `[[npc-drift-contamination]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-down-stairs.md`
- Worktree: `/home/sdancer/albion-tutorial-down-stairs/`
- H8 verdict (read first if available): `/home/sdancer/albion-tutorial-bridge-enemies/analysis/tutorial_bridge_enemies_verdict_2026-05-25.md`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_down_stairs_advanced`
