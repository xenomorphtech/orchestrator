# navmesh-proportion-fix — QUEUED next-turn directive

User report: "navmesh dimensions proportion is still incorrect compared to units distance"

The runtime-calibrated bbox approach from your earlier turn (commit on navmesh-scale-fix branch) **stretches the navmesh non-uniformly** to fit a per-axis entity bbox. That distorts the shape: the navmesh JSON's intrinsic aspect ratio (width/height in world coords) doesn't match the entity sample's bbox aspect, so X and Z scale factors diverge and the navmesh ends up squished or stretched along one axis.

## The fix
Replace the per-axis bbox calibration with a **single uniform world→pixel scale**:

1. Compute the **navmesh's intrinsic world bbox** ONCE at startup from `/home/sdancer/albion-navmesh/analysis/level75_navmesh.json` (min/max X, min/max Z across all polygon vertices). The navmesh is authored in world coords — its bbox IS the zone's playable bounds.
2. Pick a **single scale** = `min(canvas_width / world_width, canvas_height / world_height)`. Use min (not avg, not stretch-to-fit) so the navmesh fits inside the canvas without distortion. Compute `tx, ty` translate to center it.
3. Apply the SAME `(x, z) → (px, py) = (tx + scale * (x - world_xmin), ty + scale * (z - world_zmin))` transform to BOTH the navmesh SVG/polygon rendering AND every entity dot (players, mounts, buildings, mobs, NPCs, markers, self).
4. The runtime entity-anchor calibration was wrong; remove it. Entities and navmesh share **the same world coordinate system** — they don't need different calibrations.
5. Add aspect-correct letterboxing if the canvas's aspect differs from the world's: black bars on top/bottom or left/right, navmesh centered inside the visible region. Or make the canvas itself fit the navmesh aspect (preferred — eliminates letterboxing entirely).

## Verification
- Open https://albion.orch.run/ in your browser-equivalent (curl + visual diff)
- Take a screenshot showing the navmesh polygons + entity dots; entities at known fixed positions (Captain Tia marker if visible, or a player standing still) should land **inside** the navmesh AND at the correct relative position. If you have two entities at known world positions (e.g. self_x/self_z and one other player), the pixel distance between them should be `scale × world_distance` — not different by axis.
- Save screenshot to `/home/sdancer/vastai-albion-navmesh-scale/analysis/navmesh_proportion_fix_screenshot.png`.

## Constraints
- Do NOT touch the marker-color/legend work from your previous turn (commit it separately, before this).
- Do NOT touch send-hook config, pcap daemon, or the poll-rate setting.
- Keep the navmesh aspect uniform — no per-axis scaling.

## Reporting
Post one milestone line to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` with: commit SHA, the chosen scale value, world bbox dims, canvas dims, screenshot path.
