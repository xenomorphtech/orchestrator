# albion-tutorial-bridge-enemies — H8: tutorial step 5 after look-for-ship

## URGENT — 30-min cap, recipe-proven
H7 succeeded 2026-05-25 15:06 in **6 min** via proximity-gate. Quest advanced "Look for the ship." → **"Check the bridge for enemies."** Recipe is working very well — proximity-gate triggers without LEFT-click for the visual-objective steps.

## Already achieved (DO NOT re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L1-L4 | facts `albion_tutorial_step_talk_to_survivor_advanced` + `climb_tower_advanced` + `look_for_ship_advanced` | Tutorial steps 2-4 done; recipe validated 3x | ✅ DONE |
| L5 | H7 verdict at `/home/sdancer/albion-tutorial-look-for-ship/analysis/tutorial_look_for_ship_verdict_2026-05-25.md` | Proximity-gate alone advances visual-objective steps | ✅ DONE |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-bridge-enemies`** (branch `tutorial-bridge-enemies`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_check_bridge_enemies_advanced`
- **success metric**: `/state` zone change OR quest panel text change from "Check the bridge for enemies." to next step.

## Hypothesis
"Check the bridge for enemies" likely involves:
- (a) Walking to a "bridge" structure (need to find — likely visible from current LIGHTHOUSETOP elevation)
- (b) Either proximity-gate at bridge OR look-action (camera pan / hover over enemy sprites)
- (c) Possibly LEFT-click on visible enemy sprites if proximity alone is insufficient

Apply recipe: **proximity-gate first** (try walking to bridge marker), then escalate to click if needed.

## Substrate state (verified 15:09)
- Veldra1203 at (-13.43, -38.59) on LIGHTHOUSETOP marker (lighthouse upper platform)
- Quest panel: "Information is Key" / "Check the bridge for enemies."
- Action-loop STILL STOPPED — keep stopped
- /state may go x=null on capture-service restart — fall back to screen-based verification

## Probable target markers
Check /state.location_markers for: `BRIDGE`, `ENEMIES`, `HERETICS` (-2.0,-36.0 was visible earlier), `QUESTMARKERBOXTRIGGER02` (28.5,-70.5), or any new marker that appeared.

If "HERETICS" marker (-2.0, -36.0) is the bridge enemies — Δx=+11.4 east + Δz=-2.6 south from current position. Very close.

## Tasks (30-min budget)

### T0 (~3min) — Baseline + identify bridge
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-bridge-enemies/analysis/t0_baseline.png'`
3. Inspect baseline for visible bridge + enemy sprites
4. Check /state.location_markers for BRIDGE / HERETICS / similar
5. Touch `/tmp/heartbeat_albion-tutorial-bridge-enemies`

### T1 (~15min) — Walk to bridge area + proximity hold
1. Right-click ground-moves toward bridge candidates. Target HERETICS(-2.0,-36.0) first since it's closest.
2. Poll `/state.self` every 3s to verify movement.
3. When within 5 world-units of target: hold 10s for proximity-gate test.
4. Screenshot post-hold. Check quest text.

### T2 (~7min) — If proximity inert, try LEFT-click on enemies
1. Find enemy sprites visible at bridge area (red name tags / hostile mob indicators).
2. LEFT-click each (5+ candidates).
3. Verify quest after each.

### T3 (~5min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_check_bridge_enemies_advanced "<mechanism>; artifact analysis/tutorial_bridge_enemies_verdict_2026-05-25.md"` + commit verdict.
- **Failure**: commit `analysis/tutorial_bridge_enemies_blocked_2026-05-25.md` mechanism-scoped.

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- **action-loop MUST stay STOPPED**.
- NEVER spam Escape (quit-dialog risk).
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- /state may temporarily go null on capture-service restart — fall back to screen.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — validated 4 steps now (step 6 has proximity-gate-or-click duality)
- `[[npc-drift-contamination]]` — action-loop stopped
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure if needed

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-bridge-enemies.md`
- Worktree: `/home/sdancer/albion-tutorial-bridge-enemies/`
- H7 verdict (reference): `/home/sdancer/albion-tutorial-look-for-ship/analysis/tutorial_look_for_ship_verdict_2026-05-25.md`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_check_bridge_enemies_advanced`
