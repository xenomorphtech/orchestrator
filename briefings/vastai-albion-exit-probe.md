# vastai-albion-exit-probe — precision test of zone-exit confirm input

## Role & workdir
The navmesh path successfully navigated Vyqmzsni to The Cove zone-exit boundary, where a "You're about to exit to Forgotten Woods" banner appears. The prior worker (`vastai-albion-camera-rotation`) tested 5 inputs (left/right click banner center, edge-icon click, Enter key, forward-path right-click) WHILE THE LOOP WAS STILL RUNNING — meaning each test was contaminated by the loop's continuous movement clicks. Your job: STOP THE LOOP FIRST, then probe inputs in isolation with clean 15-second observation windows.

**Workdir**: `/home/sdancer/vastai-albion-exit-probe` (git worktree, branch `exit-probe`)

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-zone-exit-confirm-discover-input`

## Hypothesis & falsification
**Hypothesis**: Albion's "exit to Forgotten Woods" prompt is dismissed by one of: (a) walking continuously northeast (the boundary direction) so the char crosses the threshold, (b) pressing the Y key for confirm, (c) clicking the small chevron/arrow on the minimap's exit-edge marker, or (d) right-clicking on the actual game-world gate visible center-screen. The PRIOR worker's 5-input probe failed because the loop's continuous movement clicks moved the char away from the boundary before each test could complete. Running the probe with the LOOP STOPPED will discover the correct input.

**Falsification**: After 4 isolated 15-second probe windows with loop stopped, none of (a)-(d) transition the zone. Then the input is something more obscure (NPC interaction, key combo, specific menu, etc.) and lane-z libUE4-sidechannel is warranted.

## Already achieved (do not re-falsify)

| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| 1 | `/home/sdancer/albion-navmesh/analysis/level75_navmesh.json` | navmesh extracted (1131 tris, 680 walkable) | local | ✅ DONE |
| 2 | `/home/sdancer/albion-pcap-decode/analysis/player_position_stream.csv` (2256 rows) | Move x/z field layout resolved | local | ✅ DONE |
| 3 | `/home/sdancer/vastai-albion/scripts/navmesh_runtime.py` | minimap-pixel↔world-XZ transform verified against pcap anchors | live driver | ✅ DONE |
| 4 | `/home/sdancer/vastai-albion/scripts/steps/50_move_to_zone_exit.py` | game-world click projection + UI-region clamp | live driver | ✅ DONE |
| 5 | navmesh-integrate empirical | walked Vyqmzsni from dist 85→30 at zone boundary | live | ✅ DONE |
| 5.5 | `/home/sdancer/vastai-albion-camera-rotation/analysis/zone_exit_confirm_needed.md` | enumerated 5 failed input probes WITH LOOP RUNNING | live | ✅ DONE |
| 6 | (open) | discover which input transitions Cove → Forgotten Woods | live | ❌ THIS TURN |

## Success criteria
1. Stop `albion-loop.service` (`sudo systemctl stop albion-loop.service`).
2. Verify the banner is visible — if not, the character drifted; restart the loop briefly to re-navigate, then stop again when dist <= 31 AND the screenshot shows the "Forgotten Woods" banner upper-center.
3. Once banner is visible WITH LOOP STOPPED, test inputs one at a time with 15-second observation window between each:
   - **Probe A** (most likely): walk char NE continuously via ~5 successive right-clicks at NE-edge game-world coords like (1700, 200), (1700, 200), ... — at 1s intervals. Observe if char crosses boundary.
   - **Probe B**: press `Y` key once via `xdotool key Y` (Albion's typical confirm). Wait 15s.
   - **Probe C**: left-click on the minimap exit chevron — examine minimap region X:1640-1912, Y:730-948 for any arrow-like icon near the edge pointing NE.
   - **Probe D**: right-click on the game-world wooden gate visible center-screen (approximately X:900-1100, Y:200-400 based on prior screenshots). Wait 15s.
4. After each probe: take screenshot via `scp -P 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai:/tmp/shot.png /tmp/shot_probe_<letter>.png`. Look for either: (i) zone-name change on minimap from "The Cove" to "Forgotten Woods", (ii) loading-screen flash, (iii) char position teleporting beyond minimap.
5. When you find the winning input, restart the loop AND patch step50 to apply it whenever the exit banner is detected. Document in `analysis/zone_exit_input_discovered.md`.
6. Set fact `albion_zone_exit_input_2026_05_20` with: the winning input (e.g. "right-click (1700, 200) repeated 5x") + before/after screenshot paths.
7. If all 4 probes fail: write `analysis/all_exit_probes_failed.md` listing what was tried (with screenshot evidence) + verdict that the zone-exit needs substrate access (lane-z).

## Next 2-3 concrete tasks (in order)

1. **Stop the loop** (cleanest test environment):
   ```
   sudo systemctl stop albion-loop.service
   ```
   Verify with `systemctl is-active albion-loop.service` → "inactive".

2. **Verify banner state**: read `/tmp/shot.png` locally OR `scp` fresh from remote. Look for "Forgotten Woods" text upper-center. If not present, briefly restart loop to re-navigate (~2 min), then stop again.

3. **Execute probes A-D sequentially**. For each:
   - Send input via remote SSH xdotool: `ssh -i /home/sdancer/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai 'DISPLAY=:1 XAUTHORITY=/home/albion/.Xauthority sudo -u albion xdotool ...'`
   - Wait 15 seconds.
   - Take new screenshot, compare zone name on minimap.
   - Log result in talk channel.
4. Restart loop after probes, patch step50 with the winning input or write the failure report.

## Constraints & gotchas

- **The loop must be STOPPED during probes** — that was the prior worker's missed step. Use `sudo systemctl stop`, not `kill` (which would lose the cgroup metadata).
- **Each probe needs a clean before/after screenshot pair**. Do NOT interleave probes.
- **No Frida, no kernel hooks**. Pure xdotool over SSH.
- **License hygiene**: no community decoder code copied.
- **Hard RSS cap 100 MB**.
- **adb localhost:5558**: not relevant to this work (Albion is on vast.ai, not the RK3588).
- **DO NOT modify** /home/sdancer/vastai-albion or other workers' worktrees outside your own except for the final deploy step.

## Relevant files / references

- Live driver: `/home/sdancer/vastai-albion/scripts/`
- Prior worker analysis: `/home/sdancer/vastai-albion-camera-rotation/analysis/zone_exit_confirm_needed.md`
- Live loop unit: `albion-loop.service` (transient systemd in user.slice)
- Screenshot: `/tmp/shot.png` (local cache, refreshed by loop) or `/tmp/shot_for_probe.png` after manual scp
- Vast.ai SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Memory pointers: `[[albion-tutorial-clickclass]]`, `[[albion-client-wedge-class]]`, `[[worker-artifact-isolation]]`

## Reporting cadence
Append per-probe results to `/home/sdancer/orchestrator/analysis/talk_channels/albion.jsonl` with `{ts, from: "vastai-albion-exit-probe", text}`.
