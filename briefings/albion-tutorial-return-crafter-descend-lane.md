# albion-tutorial-return-crafter-descend-lane — H11b descend-then-NPC revival

## URGENT — 30-min cap, descent-first revival of H11

H11 (`albion-tutorial-return-crafter`) was mechanism-dropped after worker stalled at (-1.75,-41.25) trying east/SE right-click locomotion. Adv-enum diagnosed root cause: **Veldra is stranded on the UPPER LIGHTHOUSE PLANK, not on the lower NPC approach lane**. (Echoes H5 climb-tower's "wrong elevation, not no-path" insight.) The worker exhausted east/SE clicks at wrong elevation — needs to DESCEND to lower walkable lane first.

Top adv-enum recommendation: Class A #1 — re-localize from final_screen.png and click visible descent surface to reach the lower lane, THEN proceed to Crafter NPC.

Adv-enum output: `/home/sdancer/albion-tutorial-return-crafter-adv-enum/analysis/albion-tutorial-return-crafter_adversarial_alternatives.md` (20 net-new mechanisms across 3 classes).

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 2 Talk-survivor | H2 | LEFT-click NPC sprite at world (11.71,-57.02) | ✅ |
| 6 Down-stairs | H9 | Right-click descent on exterior stair chain | ✅ (descent IS solvable) |
| 7 Dodge | H10b | F+L-click ground compound | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-return-crafter-descend-lane`** (branch `tutorial-return-crafter-descend-lane`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_return_to_wounded_crafter_advanced`
- **success metric**: quest text changes from "Return to Wounded Crafter" to next step OR counter 0/1 → 1/1.

## Hypothesis
Two-stage path:
1. **Descend**: from current player position on upper plank to lower walkable lane (via H9-pattern stair/plank descent). This unblocks Albion pathfinding.
2. **Approach + interact**: standard H2/H7/H8 pattern — right-click locomotion E+slight-N to Crafter at world (11.71,-57.02), then LEFT-click sprite. Likely opens dialogue → click Complete.

## Substrate state (verified ~16:48)
- Player likely still near (-1.75,-41.25) zone "The Lighthouse" (last verified at H11 close)
- Verify via /state at task start
- Quest panel: "Return to Wounded Crafter in The Lighthouse. 0/1"
- Action-loop STILL STOPPED — keep stopped

## Tasks (30-min budget)

### T0 (~3min) — Baseline + read final_screen.png from H11
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root analysis/t0_baseline.png'`
3. **Inspect** `/home/sdancer/albion-tutorial-return-crafter/analysis/final_screen.png` to visually identify the descent surface (lower stone/bridge band below-right of tower per adv-enum hint).
4. Touch heartbeat.

### T1 (~10min) — DESCEND to lower lane (priority over east-locomotion)
Try in order. Stop on first that produces >2 world-unit movement OR clear elevation change:
1. **Class A #1**: Click the visible SE descent surface (lower stone band, not east horizon). Aim screen-Y > current Veldra Y on screen.
2. **Class A #2**: Cardinal stuck-break — try N (960,140), W (560,476), S (960,860) one at a time. First success = continue from new position.
3. **Class A #5**: WASD keyboard step — `xdotool key s` (south), check /state delta, then resume mouse.
4. **Class A #7**: Minimap click — click near quest arrow / bridge exit on minimap (NOT (1697,900) which already failed).

### T2 (~12min) — Approach NPC + interact (H2 pattern)
After descent confirmed:
1. Right-click ground SE toward Crafter at (11.71,-57.02). Hop in 3-5 unit increments.
2. At NPC ring (within 5 world-units of target): LEFT-click NPC sprite. Untried siblings:
   - (1310,150), (1310,200) — alternate Y-coords
   - `?` indicator hitbox above NPC head
   - RIGHT-click NPC for auto-walk-and-interact
   - Double-click or `click --repeat 2`
   - Real-press mousedown-hold-mouseup per [[unity-real-press-required]]
3. If dialogue opens, click Complete button. Try `--repeat 2` if first click misses Unity UI.

### T3 (~5min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_return_to_wounded_crafter_advanced "<recipe>; artifact analysis/tutorial_return_crafter_descend_verdict_2026-05-25.md"` + verdict.
- **Failure**: mechanism-scoped `analysis/tutorial_return_crafter_descend_blocked_2026-05-25.md` with NEW set of ≥3 untried siblings (don't re-list adv-enum's 20 since they cover the same classes).

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- DO NOT trust inherited coords without per-frame visual verification ([[npc-drift-contamination]]).

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 3-subclass taxonomy (proximity / LEFT-click / armed-cast); descent is the 4th mechanism axis being discovered now
- `[[npc-drift-contamination]]`
- `[[unity-real-press-required]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- Adv-enum output: `/home/sdancer/albion-tutorial-return-crafter-adv-enum/analysis/albion-tutorial-return-crafter_adversarial_alternatives.md`
- H11 verdict + final_screen.png: `/home/sdancer/albion-tutorial-return-crafter/analysis/`
- H2 verdict (NPC interaction reference): `/home/sdancer/albion-tutorial-advance-localized-walk/analysis/tutorial_advance_walk_verdict_2026-05-25.md`
- H9 verdict (descent reference): `/home/sdancer/albion-tutorial-down-stairs/analysis/tutorial_down_stairs_verdict_2026-05-25.md`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_return_to_wounded_crafter_advanced`
