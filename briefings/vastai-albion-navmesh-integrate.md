# vastai-albion-navmesh-integrate — replace perturb tier with A* pathfinding

## Role & workdir
Wire the shipped Albion tutorial-zone navmesh (`level75_navmesh.json`) + A* pathfinder (`pathfind.py`) into the autonomous loop's movement layer, replacing the bouncing perturb tier with deterministic waypoint navigation.

**Workdir**: `/home/sdancer/vastai-albion-navmesh-integrate` (git worktree, branch `navmesh-integrate`)

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-navmesh-pathfind-integration`

## Hypothesis & falsification
**Hypothesis**: The plateau at dist≈31 is a perturb-tier limit, not a tutorial-quest gate. Replacing perturb with A* on the extracted navmesh will collapse dist below 10 because the navmesh encodes the actual walkable triangulation (1131 tris, 680 walkable, 13 components, largest=301) and A* will find a path the random-walk perturb misses.

**Falsification**: After integration, dist still hovers at 30+ for 5+ consecutive cycles, indicating the wedge is NOT terrain-pathfinding (it's likely tutorial-quest gating — see `[[albion-tutorial-clickclass]]`).

## Already achieved (do not re-falsify)

| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| 1 | `/home/sdancer/albion-navmesh/analysis/level75_navmesh.json` (58KB) | 1131 tris, 680 walkable, 13 components, largest=301 | local file | ✅ DONE |
| 2 | `/home/sdancer/albion-navmesh/scripts/pathfind.py` | A* finds 36-tri path tri 10→482, length 18.43 world units | local file | ✅ DONE |
| 3 | `/home/sdancer/albion-navmesh/extracted/level75/navmesh_settings.yaml` | agentRadius=0.5 agentHeight=2 agentSlope=45 cellSize=0.16667 tileSize=256 | local file | ✅ DONE |

## Success criteria
1. **Modified `scripts/run.py`** in this worktree: step50 reads navmesh + pathfinds when stuck_streak ≥ 3, returning A* waypoint instead of perturb-tier offset.
2. **`scripts/navmesh_runtime.py`** (new): lightweight navmesh+A* runtime imported by `run.py`. Hard RSS cap 50 MB.
3. **`analysis/navmesh_integration_test.md`** (new): documents the minimap-pixel ↔ world-XZ transform you derived, and the 3+ test cases (current dist=31 state, dist=85 brute_start state, dist=10 close-target state) showing A* output waypoint pixel.
4. **Test run**: ssh to the vast.ai instance, deploy your modified `run.py`, kick the loop, observe dist dropping below 30 within 50 step50 ticks. Set fact `albion_navmesh_integration_dist_below_<N>_2026_05_20`.
5. Fact `albion_navmesh_integrated_2026_05_20` on success, OR `albion_navmesh_integration_blocked_<reason>` on falsification.

## Progress so far (sibling worktree)
- `albion-navmesh` worker (sibling worktree, closed) extracted Unity NavMeshData from `WM_LANDMASS_TUTORIAL_01.glb` via AssetRipper 1.3.14 on the vast.ai remote. Output is on YOUR local filesystem under `/home/sdancer/albion-navmesh/`.
- Navmesh JSON schema (read `pathfind.py` for canonical form): `{triangles: [{id, verts: [[x,y,z]×3], walkable, neighbors: [tri_ids]}], components: [...]}`. Tri id 10 ↔ tri 482 is a verified A* example.
- Autonomous loop runs on Hetzner host as PID 2661682 (NOT this worker's box). Loop driver is `/home/sdancer/vastai-albion/scripts/run.py`. State files: `/tmp/albion_status.json`, `/tmp/step50_state.json`, `/tmp/shot.png`. **Loop drives a remote game on vast.ai instance 37014838 via VNC xdotool clicks.**

## Next concrete tasks (in order)

1. **Derive minimap-pixel → world-XZ transform**. The loop sees the game via a minimap region of `/tmp/shot.png`. The character icon ("yellow flag") is at a known minimap pixel each cycle (cluster centroid logic in `run.py` step50). The quest target ("yellow blob") is at another known minimap pixel. You have:
   - Current `/tmp/albion_status.json` (player_name=Veldrynx, IN_WORLD)
   - Current minimap shots from prior `/tmp/shot.png` snapshots (scp from remote if needed: `ssh -p 14838 -i ~/.ssh/id_ed25519 root@ssh8.vast.ai 'cat /tmp/shot.png' > /tmp/shot_remote.png`)
   - Navmesh world-XZ coords range x:-75..-52, z:71..131 (from prior NewCharacter positions in pcap)
   - Approach: pick 2-3 landmarks where you can identify BOTH minimap-pixel AND world-XZ. Use `albion-pcap-decode/analysis/player_position_stream.csv` for char positions (12 NewCharacter rows). The minimap pixel for each of those positions can be inferred by running the loop in record mode + cross-referencing timestamps. Output a 2x3 affine matrix.

2. **Write `scripts/navmesh_runtime.py`** that exposes:
   - `load_navmesh(path) -> NavMesh` (parses level75_navmesh.json once, caches)
   - `world_to_tri(navmesh, world_xz) -> tri_id`
   - `astar(navmesh, src_tri, dst_tri) -> [tri_ids]` (uses your sibling's pathfind.py implementation; copy into this worktree to keep paths.json invariant 2 — do NOT import across worktrees)
   - `next_waypoint(navmesh, char_world_xz, target_world_xz) -> world_xz` (returns first waypoint, ≤6 world units away)
   - `world_to_minimap_pixel(transform, world_xz) -> (px, py)` (inverse of step 1)

3. **Patch `scripts/run.py`'s step50**: when `stuck_streak ≥ 3` and `brute_mode == 'none'`, instead of incrementing perturb_idx, call `navmesh_runtime.next_waypoint(...)`. If waypoint resolves, click that minimap pixel (right-click for general movement — `[[albion-tutorial-clickclass]]` says left-click is ONLY for tutorial-quest dwell; we're past tutorial). Add a feature flag `NAVMESH_ENABLE=1` env var so the old perturb code is unchanged when disabled.

4. **Deploy + test**: scp the patched `run.py` + `navmesh_runtime.py` + `level75_navmesh.json` to the remote (or to the Hetzner driver host — confirm `ssh hetzner` works; if not, modify `/home/sdancer/vastai-albion/scripts/run.py` directly with the same patch, that's where PID 2661682 is). Restart the loop. Watch `/tmp/step50_state.json` `dists[]` for 50 cycles. Fact-set the result.

## Constraints & gotchas

- **DO NOT touch the running loop's process directly.** Modify the source file, then signal a graceful restart (the loop has a known relaunch pattern — check if `vastai-albion/scripts/restart_loop.sh` exists; if not, just kill PID 2661682 and respawn from systemd/nohup convention used in the repo).
- **No anticheat-relevant actions.** Pure file/CV/click work. Memory: `[[albion-client-wedge-class]]`.
- **Hard RSS cap 100 MB** for navmesh_runtime — the navmesh JSON is 58KB but full memory could balloon if you naively build an N² distance matrix. Use per-query A* with priority queue.
- **License hygiene**: pathfind.py is your sibling's work — copy verbatim, attribute. Do NOT pull GPL navmesh code from the internet.
- **Right-click vs left-click**: post-tutorial = right-click for movement (`xdotool click 3` over VNC). Left-click is only for the tutorial-zone dwell-on-target requirement which is already resolved. Memory: `[[albion-tutorial-clickclass]]`.
- **Status JSON live during test**: `/tmp/albion_status.json` updates every loop cycle; tail it during test to verify dist is dropping.

## Relevant files / references

- Navmesh artifact: `/home/sdancer/albion-navmesh/analysis/level75_navmesh.json`
- Pathfinder source: `/home/sdancer/albion-navmesh/scripts/pathfind.py`
- Loop source (current): `/home/sdancer/vastai-albion/scripts/run.py` (the running PID 2661682 reads from here)
- Loop state: `/tmp/step50_state.json`, `/tmp/albion_status.json`, `/tmp/shot.png`
- Pcap position stream (for landmarks): `/home/sdancer/albion-pcap-decode/analysis/player_position_stream.csv`
- SSH to vast.ai: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Memory pointers: `[[albion-tutorial-clickclass]]`, `[[albion-substrate]]`, `[[albion-client-wedge-class]]`, `[[vastai]]`

## Reporting cadence
Append a status line to `/home/sdancer/orchestrator/analysis/talk_channels/albion.jsonl` every 5 min (`{"ts":"<ISO>","from":"vastai-albion-navmesh-integrate","text":"<short status>"}`).
