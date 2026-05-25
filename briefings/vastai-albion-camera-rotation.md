# vastai-albion-camera-rotation — fix minimap-direction projection to account for camera rotation

## Role & workdir
The deployed navmesh A* + minimap→world transform + game-world-click projection is correct EXCEPT for one thing: it assumes minimap-north = screen-up, which is wrong when Albion's camera is rotated. The character is stuck at minimap (80, 102) trying to reach flag (97, 76) because the click goes to upper-RIGHT (1455, 186) but the actual quest gate visible on screen is upper-LEFT.

**Workdir**: `/home/sdancer/vastai-albion-camera-rotation` (git worktree, branch `camera-rotation`)

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-camera-rotation-correction`

## Hypothesis & falsification
**Hypothesis**: Detecting Albion's camera rotation angle (from minimap compass arrow OR via trial-and-error of probe clicks in 8 directions) and applying the inverse rotation to the minimap-delta-to-game-world projection in `scripts/navmesh_runtime.py` will unstick the character and let dist drop below 25.

**Falsification**: After detecting and applying rotation correction, 5+ consecutive navmesh-fire cycles still produce 0 movement OR rotation can't be reliably detected from the available screen pixels.

## Already achieved (do not re-falsify)

| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| 1 | `/home/sdancer/vastai-albion/scripts/navmesh_runtime.py` | minimap_pixel_to_world correctly maps anchors to pcap world coords | live driver | ✅ DONE |
| 2 | `/home/sdancer/vastai-albion/scripts/steps/50_move_to_zone_exit.py` | navmesh A* fires when stuck; click goes to game-world viewport (not minimap) | live driver | ✅ DONE |
| 3 | `/tmp/step50_state.json` | reports `navmesh_waypoint_world`, `navmesh_waypoint_local`, `navmesh_click_screen` | runtime | ✅ DONE |
| 4 | character moves N→S→E→W when correct game-world click direction is supplied | brute_north earlier moved char from 32.0→30.5 | empirical | ✅ DONE |

## The bug (visual evidence)

`/tmp/shot.png` shows:
- Player character (Vyqmzsni, red marker) in lower-middle-right of screen.
- Quest gate (wooden fence with NPC marker) at **upper-LEFT** of game viewport.
- Minimap (bottom-right) shows char at (80, 102) and exit flag at (97, 76) → delta (+17, -26) = "east and north on minimap".

The current code projects this minimap delta as "upper-RIGHT on screen" (click goes to ~(1455, 186)). But the actual game-world direction to the gate is upper-LEFT. **The camera is rotated approximately 90° clockwise from minimap-north-up orientation.**

## Success criteria
1. New function `detect_camera_rotation(screenshot) -> degrees` in `scripts/navmesh_runtime.py` that returns the rotation angle (or applies a hardcoded fallback if detection fails).
2. Modified `waypoint_to_game_world_click()` (or whatever you named the game-world-click projection function) to first rotate the minimap-delta vector by `-camera_rotation` degrees, then project to game-world screen.
3. Deploy + verify: within 90s of deploy, `navmesh_click_screen` should land in the upper-LEFT quadrant of the screen (X < 960, Y < 540) AND `step50_state.json` `dists` should show progression (not all the same value).
4. Set fact `albion_camera_rotation_applied_2026_05_20` with detected rotation angle + first 5 post-deploy dist values.

## Approach (pick the simplest that works)

**Approach A — compass arrow detection** (~1h):
- Albion's minimap typically has a small compass/north arrow somewhere along its frame. Inspect `/tmp/shot.png` minimap region (Y:730-948, X:1640-1912). Look for a triangular or arrow-shaped colored shape. If found, its angle from vertical = camera rotation.
- May not be reliable if Albion's minimap doesn't display a compass in this UI mode.

**Approach B — trial probe** (~1h, robust):
- Have step50 fire test right-clicks at 8 cardinal/intercardinal game-world directions (N, NE, E, SE, S, SW, W, NW — at known screen positions e.g. (960, 100), (1700, 100), (1820, 540), (1700, 950), (960, 980), (200, 950), (100, 540), (200, 100)).
- Each test click: take screenshot, wait 1s, take another screenshot, see if char position changed.
- The direction where char moved most ≈ "north in game-world screen". Apply that as a hardcoded rotation constant.
- This is empirically grounded; needs to happen ONCE per zone (cache the rotation).

**Approach C — gate template match** (~2h, tutorial-zone only):
- Template-match the wooden fence / gate shape directly in `/tmp/shot.png` game-world region (X:0-1640, Y:0-730).
- Click on its center.
- Doesn't generalize but should unstick this specific chokepoint.

**Pick A first; if compass arrow not visually distinct, fall back to B.**

## Next 2-3 concrete tasks (in order)

1. **Take a screenshot snapshot** and visually inspect the minimap region. Look for compass arrow / N indicator. If you don't see one, skip A and go to B.

2. **If Approach A succeeds**: Write `detect_camera_rotation()` in `scripts/navmesh_runtime.py`. Test on `/tmp/shot.png`. Then modify the game-world-click function to rotate the direction vector by `-rotation` degrees before projection. Deploy + verify.

3. **If Approach B**: Implement `probe_camera_rotation_via_test_clicks()` as a one-shot routine that the loop runs on startup or when env var `RECALIBRATE_ROTATION=1`. Record the angle, write it to `/tmp/albion_camera_rotation.json`. Then modify navmesh_runtime to read this and apply.

4. **Deploy**: 
   - `cp /home/sdancer/vastai-albion-camera-rotation/scripts/navmesh_runtime.py /home/sdancer/vastai-albion/scripts/navmesh_runtime.py`
   - `sudo systemctl restart albion-loop.service`
   - `rm -f /tmp/step50_state.json` (clear stuck state)
   - Watch `journalctl -u albion-loop.service -f` for 90s.

## Constraints & gotchas

- **Loop is now under `albion-loop.service`** (systemd transient unit in user.slice). Memory `[[worker-artifact-isolation]]`: don't spawn nohup daemons — use `sudo systemctl restart albion-loop.service` if you need to bounce the loop.
- **No anticheat-relevant actions** (`[[albion-client-wedge-class]]`). All work is local CV + xdotool clicks.
- **Hard RSS cap 100 MB** for the runtime; if Approach C uses cv2 template-matching, free the screenshot bytes immediately.
- **Right-click for movement** (`[[albion-tutorial-clickclass]]` — post-tutorial zone, RIGHT-click).
- **The dist metric is a minimap-pixel distance** (so it's rotation-invariant). When dist drops, the char actually moved closer to the flag world-coord.
- **DO NOT modify** the `albion-pcap-decode` worktree or its files. The navmesh_runtime.py copy in YOUR worktree is the canonical source.

## Relevant files / references

- Live driver: `/home/sdancer/vastai-albion/scripts/`
- Screenshot snapshot: `/tmp/shot.png` (updated per loop cycle)
- Loop state: `/tmp/step50_state.json` (read for current dist, stuck_streak)
- Loop log: `journalctl -u albion-loop.service -n 100`
- Memory pointers: `[[albion-tutorial-clickclass]]`, `[[albion-substrate]]`, `[[albion-client-wedge-class]]`, `[[worker-artifact-isolation]]`
- Fact ledger: `albion_navmesh_camera_rotation_bug_2026_05_20` (the bug diagnosis from cycle 2561)

## Reporting cadence
Append phases to `/home/sdancer/orchestrator/analysis/talk_channels/albion.jsonl` (`{"ts":"<ISO>","from":"vastai-albion-camera-rotation","text":"<short>"}`).
