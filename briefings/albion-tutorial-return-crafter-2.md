# albion-tutorial-return-crafter-2 — H14: turn-in after gathering

## URGENT — 20-min cap, H11b recipe reuse

H13 succeeded ~15:57Z completing the gather sub-quest. Quest now: **"Return to Wounded Crafter in The Lighthouse. 0/1"** — SAME mechanism as Step 8 (H11b) but new instance (turn-in after gathering).

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 8 return-crafter | H11b | descend + LEFT-click --repeat 2 NPC sprite + Complete + Accept | ✅ verified |
| 9 sticks-and-stones-completed | H13 | gather rocks (single-step) + logs (two-step) | ✅ verified |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-return-crafter-2`** (branch `tutorial-return-crafter-2`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_return_to_wounded_crafter_2_advanced`
- **success metric**: quest text changes from "Return to Wounded Crafter" (counter 0/1) to next step.

## Hypothesis
Walk back west+south from current player position (33.56, -43.56) to Wounded Crafter NPC at world (11.71, -57.02). Δ remaining = -21.85 west, -13.46 south. Once at NPC ring, LEFT-click --repeat 2 on NPC sprite → dialogue → Complete → Accept.

This is the SAME mechanism class as H11b (NPC turn-in interaction). Should resolve fast (~10min).

## Substrate state (verified ~17:58)
- Veldra1203 at (33.56, -43.56) zone "The Lighthouse" (post-H13 gathering)
- Quest panel: "Return to Wounded Crafter in The Lighthouse. 0/1"
- Action-loop STILL STOPPED

## Tasks (20-min budget)

### T0 (~2min) — Baseline
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline
3. Touch heartbeat

### T1 (~8min) — Locomotion to Crafter
1. Right-click ground west+south toward (11.71, -57.02). Hop in 5-unit increments.
2. Poll /state for x decreasing, z increasing (south = z++).
3. At within 5 world-units of target, screenshot to find NPC sprite.

### T2 (~6min) — NPC interaction
1. LEFT-click --repeat 2 on NPC sprite (use H11b coords (960,150) as starting hint if visible at similar screen position)
2. If dialogue opens: LEFT-click --repeat 2 on Complete button
3. LEFT-click --repeat 2 on Accept button to finalize

### T3 (~4min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_return_to_wounded_crafter_2_advanced "<recipe>; artifact analysis/tutorial_return_crafter_2_verdict_2026-05-25.md"`
- **Failure**: mechanism-scoped `analysis/tutorial_return_crafter_2_blocked_2026-05-25.md`

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- `--repeat 2` on all UI/sprite clicks.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 5 mechanism subclasses now (proximity / LEFT-click sprite / armed-cast / descent / two-stage gather)
- `[[unity-real-press-required]]`
- `[[npc-drift-contamination]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H11b reference verdict: `/home/sdancer/albion-tutorial-return-crafter-descend-lane/analysis/tutorial_return_crafter_descend_verdict_2026-05-25.md`
- H13 reference verdict: `/home/sdancer/albion-tutorial-sticks-and-stones-complete/analysis/tutorial_sticks_complete_verdict_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-return-crafter-2/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_return_to_wounded_crafter_2_advanced`
