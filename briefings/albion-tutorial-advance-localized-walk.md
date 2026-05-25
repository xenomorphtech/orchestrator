# albion-tutorial-advance-localized-walk — H2 from K=6 planner: proximity-gate hypothesis

## URGENT — 60-min cap, EV=second-highest after H1
H1 (skip-menu) FALSIFIED 2026-05-25 (no skip affordance in any depth-3 menu). H2 is now top-rated. Substrate JUST RECOVERED at 2026-05-25 10:46Z after a 30min wedge — `/state.self.x=7.75 z=-26.75` in tutorial cluster `@TUTORIALSINGLE@45ba084d-5459-4a57-bdfa-a170dc358930`. ACT FAST while in-zone.

## Already achieved (DO NOT re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L1 | `albion_tutorial_step_2_advanced_2026-05-25` | Random walking advanced step 1→2 WITHOUT a click — proximity-gate works AT LEAST for one transition | ✅ DONE |
| L2 | acct_3 Veldra1203 in-zone, quest text `Talk to the survivor (0/1)`, screenshot `/tmp/orch_post_recovery_1247.png` | Substrate live + quest visible | ✅ DONE |
| L3 | `/home/sdancer/albion-tutorial-skip-menu/analysis/tutorial_skip_menu_blocked_2026-05-25.md` | Menu/world-map class FALSIFIED — no skip affordance exists | ✅ DONE |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-advance-localized-walk`** (branch `tutorial-advance-localized-walk`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_talk_to_survivor_advanced`
- **success metric**: `/state` quest text transitions from "Talk to the survivor (0/1)" to a different step, OR `zone.name` changes from `@TUTORIALSINGLE@45ba084d-5459-4a57-bdfa-a170dc358930` to a different zone.

## Hypothesis (H2 from planner)
Walking the character close to the survivor NPC (or specific map markers like `INTERACTWITHCRAFTER` at world (9.69, -57.81)) triggers the quest tick via Albion's PROXIMITY-GATE system — no precise click needed. Empirically supported by the `albion_tutorial_step_2_advanced_2026-05-25` fact: random-walk by action-loop advanced step 1→2 without any deliberate NPC click.

## Falsification (mechanism-scoped per [[falsify-mechanism-not-path]])
- **Mechanism class**: locomotion-via-ground-rightclick to bring player coords near each interactive marker in turn.
- **Falsified iff**: All 5+ visible interactive markers in /state visited (player within 2 world-units of each marker for ≥10s), AND no quest text diff AND no zone change.
- **Untried siblings if falsified**: (a) explicit walk into `QUESTMARKERBOXTRIGGER02` (28.5,-70.5) trigger box, (b) chat command `/say` or `/me` near NPC, (c) hotkey F/E/Space near NPC, (d) right-click directly ON NPC sprite (different from prior XTest LEFT-click attempts because right-click is ground-move semantics in Albion).

## Substrate state (verified 12:47 by orchestrator)
- Veldra1203 in tutorial zone, world (7.75, -26.75)
- Quest text: "First Steps → Talk to the survivor (0/1)" (visible top-right)
- NPC visible to right of player in scene
- Action-loop **STOPPED** (`systemctl --user is-active albion-action-loop.service` = inactive). DO NOT restart.
- Gamestate: 5.4 evt/sec, 324 events processed since recovery.

## Visible /state markers (potential NPC anchors)
| object_id | marker_name | world (x,z) | distance from (7.75,-26.75) |
|---|---|---|---|
| 41 | INTERACTWITHCRAFTER | (9.69, -57.81) | ~31 SOUTH |
| 5 | GATE | (9.5, -27.5) | ~2 (very close!) |
| 10 | FORGE | (4.5, -51.5) | ~25 SW |
| 6 | HERETICS | (-2.0, -36.0) | ~13 W |
| 13 | LIGHTHOUSETOP | (-13.5, -38.5) | ~23 W |
| 8 | POINT | (21.0, -57.0) | ~33 SE |
| 42 | QUESTMARKERBOXTRIGGER02 | (28.5, -70.5) | ~52 SE (trigger box!) |
| 7 | LIGHTHOUSEUP | (9.5, -64.5) | ~38 S |
| 24 | LIGHTHOUSEDOWN | (-13.5, -60.5) | ~39 SW |

`GATE` is suspicious (2 units away — could be the door player is standing near). `QUESTMARKERBOXTRIGGER02` literally has "QUESTMARKER" in the name and is a "trigger box" — strong candidate.

## Tasks (60-min budget)

### T0 (~3min) — Capture baseline + verify substrate live
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-advance-localized-walk/analysis/t0_baseline.png'`
3. Confirm quest text via screenshot inspection.
4. Touch `/tmp/heartbeat_albion-tutorial-advance-localized-walk`

### T1 (~25min) — Walk toward QUESTMARKERBOXTRIGGER02 first (highest-EV — name contains "QUESTMARKER" + "TRIGGER")
1. Right-click at screen coords corresponding to world-direction SE. Use xdotool:
   ```
   sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool mousemove 700 500 click 3'
   ```
   (700,500 is SE quadrant of 1280x720 screen — adjust empirically based on screenshot.)
2. Wait 4s, poll `/state.self.x`, `self.z`. Repeat until player is within 3 world-units of (28.5, -70.5).
3. Once close, sit still for 15s and poll /state. Watch for:
   - `zone.name` change (instant transition out of tutorial)
   - Quest panel text diff (re-screenshot top-right area, compare via SHA256)
   - New objects appearing in /state
4. If trigger fires (success_fact reached): commit verdict and STOP.

### T2 (~20min) — Walk to INTERACTWITHCRAFTER (9.69, -57.81) — explicit interaction marker
1. Same procedure, target SE quadrant slightly less far.
2. Walk to within 2 units of crafter. Hold 15s. Try right-clicking ON the marker location (RIGHT-click = ground-move, NOT a quest interaction — but might land on NPC and trigger it).
3. ALSO try: press F or E or Space hotkeys at this position (interaction-keys class — untried per falsified.md).

### T3 (~10min) — Verdict + commit
1. **If success**: 
   - `harness fact-set albion_tutorial_step_talk_to_survivor_advanced "<mechanism description> — quest advanced from 'Talk to the survivor 0/1' to '<new text>'; artifact analysis/tutorial_advance_walk_verdict_2026-05-25.md"`
   - Commit `analysis/tutorial_advance_walk_verdict_2026-05-25.md` with: pre/post /state snapshots, pre/post screenshots, world-coord trajectory, mechanism that worked.
2. **If failure (no quest diff or zone change after 60min)**:
   - Commit `analysis/tutorial_advance_walk_blocked_2026-05-25.md` mechanism-scoped: "Proximity-gate via locomotion to all visible map markers — quest text invariant. Untried siblings: chat-commands /say or /me, hotkey F/E/Space at proximity, click directly on NPC sprite with right-click (not left)."

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- **DO NOT restart albion-action-loop.service** — it must stay STOPPED for this entire turn per [[npc-drift-contamination]].
- xdotool key Escape NEVER (risks quit-dialog wedge per [[albion-client-wedge-class]]).
- Right-click (button 3) = ground-move. Left-click (button 1) = action/interact. We want RIGHT-clicks for locomotion, possibly LEFT or F/E/Space for NPC interaction at close range.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- 10-min heartbeat: `touch /tmp/heartbeat_albion-tutorial-advance-localized-walk` after each major step.
- `/tmp/abort_albion-tutorial-advance-localized-walk` → commit partial + exit.

## Memory references
- `[[npc-drift-contamination]]` — must stop action-loop, per-frame re-localization
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure only
- `[[albion-tutorial-clickclass]]` — historical: left-clicks needed for quest progression — but this was inferred from anti-pattern; proximity-gate may bypass click entirely
- `[[goals-never-blocked]]` — if H2 falsifies, brief the orchestrator's next ranked path

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-advance-localized-walk.md`
- Worktree: `/home/sdancer/albion-tutorial-advance-localized-walk/`
- /state endpoint: `http://127.0.0.1:8765/state`
- Harness: `/home/sdancer/orchestrator/harness`
- Success fact: `albion_tutorial_step_talk_to_survivor_advanced`
- Recovery screenshot from substrate-recovery: `/tmp/orch_post_recovery_1247.png`
