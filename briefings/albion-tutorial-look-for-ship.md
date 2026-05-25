# albion-tutorial-look-for-ship — H7: 4th tutorial step after climb-tower success

## URGENT — 45-min cap, exploring uncharted quest
H5 succeeded 2026-05-25 14:51, advancing quest from "Climb the tower" → **"Look for the ship."** (per H5 verdict, NOT "wisp" — wisp was an earlier mis-transcription). The proximity-gate mechanism worked at upper lighthouse: just reaching the top perch ticked the quest, no LEFT-click needed.

## Already achieved (DO NOT re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L1 | `albion_tutorial_step_talk_to_survivor_advanced` fact | Tutorial step 2 done | ✅ DONE |
| L2 | `albion_tutorial_step_climb_tower_advanced` fact + H5 verdict | Tutorial step 3 done; LOCOMOTION-only worked at top perch | ✅ DONE |
| L3 | `/home/sdancer/albion-tutorial-climb-alt-elev/analysis/tutorial_climb_alt_elev_verdict_2026-05-25.md` | Recipe extension: proximity-gate triggers can replace LEFT-click on objective | ✅ DONE |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-look-for-ship`** (branch `tutorial-look-for-ship`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_look_for_ship_advanced` (NEW)
- **success metric**: `/state` zone change OR quest panel text change from "Look for the ship." to next step.

## Hypothesis
"Look for the ship" is likely a discovery/proximity step. Recipe candidates (apply in order, fastest first):

1. **Proximity-gate from current perch** — player already at top of lighthouse (-19.23, -50.30); the ship is likely visible from this vantage point. Just opening world view / standing at top may tick if there's a "you can see it now" trigger.
2. **Camera pan/zoom** — Mouse middle-button drag or right-click ground may reveal the ship. Some tutorials require player to "spot" the target visually.
3. **Walk further to "ship vista" position** — quest may need player to reach a specific viewpoint marker. Look at /state.location_markers for SHIP, BOAT, SEA, VIEW, VANTAGE.
4. **LEFT-click on visible ship sprite** — if a ship is visible in the scene (white ring indicator visible in H5 success screenshot), click it.
5. **Open world map** (M key) to look at ship marker.

## Falsification (mechanism-scoped)
- **Mechanism class**: locomotion + click + camera mechanics combined.
- **Falsified iff**: 5 distinct probe types tried (proximity hold, camera pan, locomotion to alt-marker, LEFT-click on visible ship, world-map M-key) with no quest text diff.
- **Untried siblings if falsified**: zoom-out + click on a far-away ship, F/E hotkeys at vista, NPC re-dialogue (if NPC nearby).

## Substrate state (verified 14:55)
- Veldra1203 at world (-19.23, -50.30) on lighthouse upper structure
- /state.zone "The Lighthouse"
- Quest panel: "Information is Key" / "Look for the ship."
- White quest-ring visible in H5's t2_success.png — likely the NEXT quest objective marker
- Action-loop STILL STOPPED — keep it stopped throughout this turn

## Tasks (45-min budget)

### T0 (~3min) — Baseline + identify ship target
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline at `analysis/t0_baseline.png`
3. Check /state.location_markers for any SHIP/BOAT/SEA/VIEW marker — record names + coords
4. Visually inspect baseline: where is the white quest-ring? Where is a visible ship?
5. Touch `/tmp/heartbeat_albion-tutorial-look-for-ship`

### T1 (~15min) — Try proximity-gate (cheapest)
1. If a marker like `SHIP` exists in /state, walk toward it (right-click ground-move).
2. Hold at the closest accessible point for 10s.
3. Re-screenshot. Check quest panel text.
4. If quest advanced: done — record verdict.

### T2 (~10min) — Camera/visibility mechanics
1. If T1 inert, try middle-mouse-button drag (camera pan) to expose more of the scene.
2. Mouse scroll wheel to zoom out — may reveal off-screen ship.
3. Walk to vantage edge of lighthouse perch (perimeter of current platform).
4. Try pressing M (world map open).

### T3 (~12min) — Direct LEFT-click on ship sprite
1. If a ship sprite is visible after camera maneuvers, LEFT-click it.
2. If multiple click candidates, try 3-5 distinct coords.

### T4 (~5min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_look_for_ship_advanced "<mechanism> advanced quest from 'Look for the ship' to <new>; artifact analysis/tutorial_look_for_ship_verdict_2026-05-25.md"` + commit verdict.
- **Failure**: commit `analysis/tutorial_look_for_ship_blocked_2026-05-25.md` mechanism-scoped, listing ≥3 untried siblings.

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- **action-loop MUST stay STOPPED**.
- NEVER spam Escape (quit-dialog risk per [[albion-client-wedge-class]]).
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- 10-min heartbeat: `touch /tmp/heartbeat_albion-tutorial-look-for-ship`.
- `/tmp/abort_albion-tutorial-look-for-ship` → commit partial + exit.
- /state may go x=null on capture-service restart — fall back to screen-based verification per H5 precedent.

## Memory references
- **`[[albion-tutorial-step-advance-recipe]]`** — VERIFIED RECIPE; extended by H5 to include proximity-gate triggers
- `[[npc-drift-contamination]]` — action-loop stopped
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure if needed

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-look-for-ship.md`
- Worktree: `/home/sdancer/albion-tutorial-look-for-ship/`
- H5 verdict (READ FIRST): `/home/sdancer/albion-tutorial-climb-alt-elev/analysis/tutorial_climb_alt_elev_verdict_2026-05-25.md`
- /state: `http://127.0.0.1:8765/state`
- Harness: `/home/sdancer/orchestrator/harness`
- Success fact: `albion_tutorial_step_look_for_ship_advanced`
