# albion-tutorial-climb-alt-elev — H5: top-1 from adversarial enumeration

## URGENT — 30-min cap, top-ranked successor to H4 (mechanism-dropped)
H4 (`albion-tutorial-climb-tower`) was mechanism-dropped 2026-05-25 after 13 mouse-pointer probes on the LIGHTHOUSEUP perch. The adversarial-enum worker `climb-tower-adv-enum` (`0ede00a "Add climb tower adversarial alternatives"` — see `/home/sdancer/climb-tower-adv-enum/analysis/albion-tutorial-climb-tower_adversarial_alternatives.md`) ranked 7 untried alternatives. **You are executing Path 1** (highest-ranked, lowest-cost spatial hypothesis).

## Already achieved (DO NOT re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L1 | `albion_tutorial_step_talk_to_survivor_advanced` fact | Step 2 of tutorial complete | ✅ DONE |
| L2 | H4 verdict (commit 6084fff) | Tower-base perch reachable at LIGHTHOUSEUP(9.5,-64.5); 13 click-mechanism probes there inert | ✅ DONE |
| L3 | `albion-tutorial-climb-tower_adversarial_alternatives.md` (commit 0ede00a) | 7 ranked untried alternatives; you are executing top-1 | ✅ DONE |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-climb-alt-elev`** (branch `tutorial-climb-alt-elev`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_climb_tower_advanced` (NEW)
- **success metric**: `/state` zone change OR quest panel text change from "Climb the tower" to next step.

## Hypothesis (top-1 from adversarial)
The "Climb the tower" objective requires the player to reach a HIGHER lighthouse perch than the LIGHTHOUSEUP marker the H4 worker tested. Specifically: `LIGHTHOUSEUP2(-15.5,-47.5)` or `LIGHTHOUSETOP(-13.5,-38.5)`. Once at the correct elevation, a click on the **topmost plank / arch-interior / tower-top ring pixel** (NOT the lower log sprite H4 tested) should tick the quest.

Quote from adversarial verdict: *"the blocked run already discovered one real tower walk surface. That makes 'reach the correct higher perch' the strongest remaining non-invasive hypothesis"*.

## Falsification (mechanism-scoped)
- **Mechanism class**: locomotion-to-higher-marker + LEFT-click on top-of-tower sprite.
- **Falsified iff**: Player reached within 3 world-units of BOTH LIGHTHOUSEUP2 AND LIGHTHOUSETOP, AND ≥3 LEFT-clicks at each perch on topmost visible plank/arch/ring pixels, AND no quest-text diff AND no zone change.
- **Untried siblings if falsified**: navmesh-guided route (Path 2 from adversarial), NPC dialogue replay (Path 3), F/T/G/E keybind sweep at perch (Path 4).

## Substrate state (verified 14:13)
- Veldra1203 at (7.59, -67.25) near LIGHTHOUSEUP. Action-loop STILL STOPPED.
- /state.zone "The Lighthouse" (bootstrap source — display label is the same as decoded zone)
- 1920x1080 display on Xtigervnc :3
- Quest panel: "Information is Key" / "Climb the tower."

## Target world coords (per H4 baseline /state markers)
- `LIGHTHOUSEUP2 (-15.5, -47.5)` — NORTH-WEST of player, Δx≈-23, Δz≈+19
- `LIGHTHOUSETOP (-13.5, -38.5)` — NORTH-WEST, slightly more N+E than UP2, Δx≈-21, Δz≈+28

Both targets are SIGNIFICANTLY WEST (-x) of player and NORTH (+z, since z is more negative-deeper).

## Tasks (30-min budget)

### T0 (~3min) — Baseline + verify substrate
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-climb-alt-elev/analysis/t0_baseline.png'`
3. Verify action-loop stopped: `systemctl --user is-active albion-action-loop.service` should return `inactive`.
4. Touch `/tmp/heartbeat_albion-tutorial-climb-alt-elev`

### T1 (~10min) — Walk to LIGHTHOUSEUP2
1. From baseline screen, identify the lighthouse upper structure (taller wooden tower, likely on left/west of view).
2. Right-click ground-moves SW/NW direction. Account for isometric projection — west world ≠ left screen necessarily.
3. Re-screenshot every 2-3 moves. Poll /state.self.x/.z each move.
4. Stop when player within 3 world-units of (-15.5, -47.5).
5. Capture `t1_at_up2.png` + `t1_at_up2_state.json`.

### T2 (~7min) — LEFT-click topmost tower features at LIGHTHOUSEUP2
1. From the UP2 perch, identify topmost plank / arch-interior / tower-top ring pixels. NOT the lower log sprite H4 tested.
2. LEFT-click each candidate. Re-screenshot + poll /state after each.
3. Try 3-5 distinct click coords at this elevation.
4. Verify quest text change OR zone change.

### T3 (~7min) — If T2 fails, walk to LIGHTHOUSETOP and repeat
1. From UP2, right-click ground-moves further N/W until within 3 units of (-13.5, -38.5).
2. Repeat T2 (3-5 LEFT-clicks on topmost features).

### T4 (~3min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_climb_tower_advanced "<mechanism> at <marker> advanced quest from 'Climb the tower' to <new>; artifact analysis/tutorial_climb_alt_elev_verdict_2026-05-25.md"` + commit verdict.
- **Failure**: commit `analysis/tutorial_climb_alt_elev_blocked_2026-05-25.md` mechanism-scoped (per `[[falsify-mechanism-not-path]]`). List remaining untried siblings: navmesh-guided routing (Path 2), Wounded Crafter dialogue replay (Path 3), F/T/G/E at perch (Path 4).

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- **action-loop MUST stay STOPPED** for entire turn.
- NEVER press Escape repeatedly — quit-dialog risk per `[[albion-client-wedge-class]]`.
- 10-min heartbeat: `touch /tmp/heartbeat_albion-tutorial-climb-alt-elev` after each major step.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- `/tmp/abort_albion-tutorial-climb-alt-elev` → commit partial + exit.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — verified for H2; H4 showed simple "click sprite" doesn't always transfer
- `[[npc-drift-contamination]]` — keep action-loop stopped
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure if you can't advance

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-climb-alt-elev.md`
- Worktree: `/home/sdancer/albion-tutorial-climb-alt-elev/`
- Adversarial verdict (READ FIRST): `/home/sdancer/climb-tower-adv-enum/analysis/albion-tutorial-climb-tower_adversarial_alternatives.md`
- H4 blocked verdict: `/home/sdancer/albion-tutorial-climb-tower/analysis/tutorial_climb_tower_blocked_2026-05-25.md`
- /state: `http://127.0.0.1:8765/state`
- Harness: `/home/sdancer/orchestrator/harness`
