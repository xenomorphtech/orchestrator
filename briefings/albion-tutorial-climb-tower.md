# albion-tutorial-climb-tower — H4: next tutorial step after H2 success

## URGENT — 45-min cap, reuse proven recipe
H2 worker `albion-tutorial-advance-localized-walk` SUCCEEDED 2026-05-25 13:12 — quest advanced "Talk to the survivor 0/1 → 1/1" via direct LEFT-click on Wounded Crafter NPC. Complete-button accepted at 13:21. Next quest exposed: **"Information is Key" → Climb the tower.**

Apply the **proven recipe** from `[[albion-tutorial-step-advance-recipe]]` (project memory) to this new objective.

## Already achieved (DO NOT re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L1 | `albion_tutorial_step_talk_to_survivor_advanced` fact + verdict commit `075cca5` | Step 2 of tutorial complete | ✅ DONE |
| L2 | `/home/sdancer/albion-tutorial-advance-localized-walk/analysis/tutorial_advance_walk_verdict_2026-05-25.md` | LEFT-click on objective sprite works (under proper experimental control) | ✅ DONE |
| L3 | `revivals.md` 2026-05-25 entry | Prior XTest LEFT mechanism-dropped closures REVIVED (contamination root cause documented) | ✅ DONE |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-climb-tower`** (branch `tutorial-climb-tower`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_climb_tower_advanced` (NEW)
- **success metric**: `/state` zone change OR quest panel text change from "Climb the tower" to next step.

## Hypothesis
The "Climb the tower" objective gates on (a) locomotion to a tower object in the tutorial zone + (b) a direct LEFT-click on the tower or its base, mirroring the H2 NPC mechanism. The "Lighthouse" markers from earlier /state likely identify it: `LIGHTHOUSETOP (-13.5,-38.5)`, `LIGHTHOUSEUP (9.5,-64.5)`, `LIGHTHOUSEUP2 (-15.5,-47.5)`, `LIGHTHOUSEDOWN (-13.5,-60.5)`.

## Falsification (mechanism-scoped per [[falsify-mechanism-not-path]])
- **Mechanism class**: locomotion + direct LEFT-click on visible tower/climb-point sprite.
- **Falsified iff**: 5 LEFT-clicks on visible tower/climb objects with NPC-style template-match each, AND no quest-text diff AND no zone change.
- **Untried siblings if falsified**: right-click on tower (object-interact), F/E/Space at tower base (use-key), DOUBLE-click, click-and-hold (drag to climb), use map-marker waypoint click.

## Substrate state (verified by orchestrator 13:31)
- Veldra1203 still at world (11.71, -57.02) in `@TUTORIALSINGLE@45ba084d-5459-4a57-bdfa-a170dc358930`
- Player has NOT moved since H2 finished (action-loop is STOPPED — keep stopped per [[npc-drift-contamination]])
- Quest panel: "Information is Key" / "Climb the tower" (visible 13:21 screenshot)
- Display: 1920x1080 on Xtigervnc :3
- /state may show 0 location_markers right now (decoded marker entries age out); worker should rely on screenshot for tower identification.

## Tasks (45-min budget)

### T0 (~3min) — Capture baseline + verify substrate
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-climb-tower/analysis/t0_baseline.png'`
3. Inspect t0_baseline.png — find tower sprite (look for tall structure, may have "?" or quest-objective ring). Lighthouse is the most likely candidate.
4. Touch `/tmp/heartbeat_albion-tutorial-climb-tower`

### T1 (~20min) — Walk to tower + click
1. Identify tower screen-coords from t0_baseline.png. Likely off-screen initially (player at (11.71,-57.02) but Lighthouse marker at (-13.5,-38.5) is WEST and NORTH of player).
2. Right-click ground-moves toward tower world-coords. Account for isometric projection: world-NW ≠ screen-NW.
3. Re-screenshot every 3-5 ground-moves. Poll `/state.self.x` and `self.z` to verify world-position movement.
4. Stop when within 5 world-units of estimated tower coords OR when tower sprite is centered on screen with interaction ring visible.

### T2 (~15min) — LEFT-click on tower + verify
1. With tower visible: `xdotool mousemove <tower_x> <tower_y> click 1`.
2. Wait 3s. Screenshot post-click. Check `/state` zone change AND quest-panel text change.
3. If no progress: try right-click on tower (button 3) — alternative mechanism.
4. If no progress: try F/E/Space hotkeys at tower base (interact keys).
5. If progress: continue any dialog/Complete buttons same as H2.

### T3 (~5min) — Verdict + fact
1. **If success**:
   - `harness fact-set albion_tutorial_step_climb_tower_advanced "<mechanism> advanced quest from 'Climb the tower' to <new>; artifact analysis/tutorial_climb_tower_verdict_2026-05-25.md"`
   - Commit verdict with full trajectory + working mechanism details.
2. **If failure (no quest diff after 5 LEFT-clicks)**:
   - Commit `analysis/tutorial_climb_tower_blocked_2026-05-25.md` mechanism-scoped per [[falsify-mechanism-not-path]].
   - List ≥3 untried siblings (right-click on tower, F/E/Space, map waypoint, climb-via-character-side-click).

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- **albion-action-loop.service MUST stay STOPPED**. Do NOT restart it. Verify with `XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus systemctl --user is-active albion-action-loop.service` should return `inactive`.
- NEVER press Escape repeatedly — risks quit-dialog wedge ([[albion-client-wedge-class]]).
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- Screenshots: ALWAYS absolute paths inside sudo-runuser.
- 10-min heartbeat: `touch /tmp/heartbeat_albion-tutorial-climb-tower` after each major step.
- `/tmp/abort_albion-tutorial-climb-tower` → commit partial + exit.

## Memory references
- **`[[albion-tutorial-step-advance-recipe]]`** — VERIFIED RECIPE (just-shipped 2026-05-25, applies directly here)
- `[[npc-drift-contamination]]` — why we keep action-loop stopped
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure
- `[[albion-tutorial-clickclass]]` — LEFT-clicks tick quests
- `[[albion-login-substrate]]` — Xtigervnc :3 accepts XTEST

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-climb-tower.md`
- Worktree: `/home/sdancer/albion-tutorial-climb-tower/`
- /state endpoint: `http://127.0.0.1:8765/state`
- Harness: `/home/sdancer/orchestrator/harness`
- Predecessor verdict (for reference): `/home/sdancer/albion-tutorial-advance-localized-walk/analysis/tutorial_advance_walk_verdict_2026-05-25.md`
- Success fact: `albion_tutorial_step_climb_tower_advanced`
