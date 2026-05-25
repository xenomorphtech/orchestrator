# albion-navmesh — extract Unity NavMesh from Albion game assets

## Role & workdir
Extract the walkable navmesh + minimap↔world transform from Albion Online's Unity assets on the vast.ai instance, so the autonomous loop can replace random-walk perturb with proper A* pathfinding.

**Workdir**: `/home/sdancer/albion-navmesh`

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-navmesh-extract`

## Hypothesis & falsification
**Hypothesis**: Albion's `Albion-Online_Data/StreamingAssets/cluster/` (or equivalent) contains per-cluster Unity asset bundles that include a serialized NavMesh asset and static MeshColliders. AssetRipper or UABE can extract these to a triangulated walkable mesh. A 2-3-landmark affine transform then maps minimap pixels to world XZ, allowing A* on the navmesh to drive direct waypoint clicks.

**Falsification**: NavMesh asset is encrypted/missing from the bundles, OR the extraction tools can't read Albion's specific Unity version, OR the cluster files don't contain navmesh data at all (server-only).

## Success criteria
1. **`/home/sdancer/albion-navmesh/analysis/asset_inventory.md`** — confirmed paths + sizes of `StreamingAssets`, `*.assets`, `*.resS`, and any `cluster/` directory on the vast.ai instance.
2. **`/home/sdancer/albion-navmesh/analysis/extracted_navmesh.json`** (or `.bin`) — at least ONE cluster's navmesh extracted to a parsable format (vertices + indices + connectivity).
3. **`/home/sdancer/albion-navmesh/analysis/minimap_to_world_transform.json`** — 2x3 affine matrix derived from 2-3 known landmarks (NPC positions visible on both minimap and in-game).
4. **`/home/sdancer/albion-navmesh/scripts/pathfind.py`** — proof-of-concept A* taking `(char_world_xz, target_world_xz)` and emitting a list of waypoints.
5. Fact: `albion_navmesh_extracted_<cluster_id>_<date>` set on success, OR `albion_navmesh_extract_blocked_<reason>` on falsification.

## Context

The current `vastai-albion-sonnet` worker is driving a minimap-vision random-walk loop on the vast.ai instance `37014838`. dist is converging slowly (92→74) because perturb/brute tiers are inefficient. Your job replaces that with proper pathfinding.

**Do NOT touch the running loop** — it stays as backstop. Your output is consumed in a later integration cycle.

**SSH access to the vast.ai box**:
- Host: `ssh8.vast.ai`
- Port: `14838`
- User: `root`
- Key: `/home/sdancer/.ssh/id_ed25519` (vast.ai key id 612169)
- Albion install path on remote: `/home/albion/albion-online/`
- Test command: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai 'ls /home/albion/albion-online/'`

## Next 2-3 concrete tasks (in order)

1. **Asset reconnaissance.** SSH to the remote, locate `Albion-Online_Data/` (likely under `/home/albion/albion-online/`). Inventory `StreamingAssets/`, all `*.assets` and `*.resS` files, any directory named `cluster*`, `nav*`, or `terrain*`. Note Unity version from `Albion-Online_Data/globalgamemanagers` first 4 bytes. Write `analysis/asset_inventory.md` with paths + sizes + Unity version.

2. **Install + run AssetRipper on a tutorial-zone asset bundle**. AssetRipper releases live at https://github.com/AssetRipper/AssetRipper/releases. Download the Linux build, run against the smallest/most-likely-tutorial-zone bundle. Extract scenes. Look for `NavMeshData` assets. If AssetRipper fails on the Unity version, try UABE (https://github.com/SeriousCache/UABE/releases) as fallback. Stash extraction output under `/home/sdancer/albion-navmesh/extracted/`.

3. **Parse the navmesh + derive minimap transform**. Once you have a NavMeshData asset, parse it (Unity's NavMesh binary format: header → vertex count → vertices (XYZ floats) → tri count → triangles (3 indices each) → tile/area metadata). Output JSON. Then derive the minimap-to-world affine: pick 2-3 anchor points where you can identify both world-XZ and minimap-XY (e.g. an NPC at known world coord, visible on minimap — request the current `vastai-albion-sonnet` worker's minimap screenshot via `scp ssh8.vast.ai:/tmp/shot.png .` if helpful).

## Constraints & gotchas

- **No Frida, no kernel hooks, no game-process attachment.** Pure file-asset extraction only. Albion has anticheat (EAC) and any in-process touching would risk a ban. Memory: `[[albion-client-wedge-class]]`.
- **Albion runs as user `albion` on remote**. Read-only access to its install dir is fine; don't pkill the game process. Memory: `[[albion-substrate]]`.
- **Hard RSS cap 2 GB** on AssetRipper/UABE — they can blow memory on large bundles. If a bundle exceeds 1 GB on disk, target a smaller cluster first.
- **Disk on remote**: vast.ai container has limited disk. Extract to `/tmp/ripper-out/` on remote, scp the result back, then `rm -rf /tmp/ripper-out` on remote.
- **Use AssetRipper community Unity version detection** if the standalone build complains — there's a `--unity-version` override flag.

## Relevant files / references

- Albion install: `/home/albion/albion-online/Albion-Online_Data/` (on vast.ai remote)
- Current loop's minimap shot: `/tmp/shot.png` (on remote, updated each loop cycle)
- Status JSON: `/tmp/albion_status.json` (on remote, shows current char position)
- Community Albion data parser (XML cluster definitions, NOT navmesh): `https://github.com/broderickhyman/albiondata-client` — has type definitions that may help cross-reference world-coord scales
- AssetRipper: https://github.com/AssetRipper/AssetRipper
- UABE fallback: https://github.com/SeriousCache/UABE
- Memory pointers: `[[albion-substrate]]`, `[[albion-waydroid-works]]`, `[[vastai]]`, `[[albion-client-wedge-class]]`

## Reporting cadence

Append a status line to `/home/sdancer/orchestrator/analysis/talk_channels/albion.jsonl` every 5 min (`{"ts":"<ISO>","from":"albion-navmesh","text":"<short status>"}`).
