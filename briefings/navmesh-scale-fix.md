## Role & workdir
Codex worker at `/home/sdancer/vastai-albion-navmesh-scale` (worktree, branch `navmesh-scale-fix`, off `vastai-albion@14b7ace`).

## Goal
User reports via /talk channel `vastai-albion-web` at 14:57:44+02:00:
> the svg of the navmesh is rendering too small compared to the unit positions

The dashboard at https://albion.orch.run/ overlays player/mount/NPC markers on top of a navmesh SVG background. The two coordinate systems are not in sync — the navmesh SVG covers a smaller pixel range than where the unit dots appear, so units overshoot the rendered walkable area.

## Success criteria
Open https://albion.orch.run/ in any browser. The navmesh SVG and the player/entity overlay share a single coordinate system: a player dot inside the walkable area visually sits inside the SVG polygons; a player dot at the world edge sits at the SVG edge. Take a screenshot showing live entities (currently 25 players + 6 mounts visible — there's traffic flowing now) sitting on the navmesh, not floating off it. Save the screenshot to `/home/sdancer/vastai-albion-navmesh-scale/analysis/navmesh_scale_fix_screenshot.png`.

## Key facts (DO NOT re-derive)
- Live production dashboard runs from `/opt/albion-gamestate/gamestate_service.py` on vast.ai (`ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`).
- Local source-of-truth snapshot of that file is at `/home/sdancer/albion-gamestate-rust/albion-gamestate-service-snapshot/gamestate_service.py` (cycle 2754 worker shipped it). Edit your copy in the worktree first, then deploy.
- Navmesh JSON: `/home/sdancer/albion-navmesh/analysis/level75_navmesh.json` (1131 tris / 680 walkable). The dashboard endpoint that serves it is part of the Python service — find the route in the source.
- World coords come from `/state.players[*].x / .z` and `/state.self.x / .z` (recv-side capture; self position currently degraded to None — see [[albion-send-hooks-break-client]] — that's expected, do not chase it).
- Right now: 25 players visible (see `/state` JSON). Use that as live data while debugging.

## Likely failure mode
The HTML/JS for the dashboard is embedded in `gamestate_service.py` (large string literal returned by a Flask route). Look for the SVG `viewBox` attribute and the JS code that maps world coords → pixel coords for entity dots. They almost certainly:
- Use different bounding boxes (the SVG was set up from a hard-coded min/max derived from the navmesh JSON; the dot overlay scales world coords by a different factor)
- OR: the SVG's `width`/`height` (in CSS px) is smaller than the canvas it sits within
- OR: a transform/translate is applied to one layer but not the other

Diagnose with browser devtools (you don't have a browser — use `curl https://albion.orch.run/ -o /tmp/dash.html` + grep for `viewBox`, look at the JS scaling functions). The fix is a single coordinate-system definition shared by both layers.

## Next 2-3 concrete tasks
1. **Diagnose.** Read `gamestate_service.py` (snapshot path above). Locate (a) the navmesh SVG render block and its viewBox/scaling, (b) the entity-dot render block and its world→pixel scaling. Identify the mismatch in 1-2 sentences.
2. **Fix.** Edit `gamestate_service.py` so both layers share one world→pixel transform. Easiest: compute the navmesh bounding box server-side once, expose it via a `/navmesh_bbox` endpoint or embed it in the page-load JSON, and have the JS scale BOTH the SVG viewBox AND the entity dot positions from that same bbox. Or: stretch the SVG to fill the same canvas the dots are rendered onto. Pick whichever is shorter — your call.
3. **Deploy.** scp the patched gamestate_service.py to `/opt/albion-gamestate/gamestate_service.py` on vast.ai. Restart the service (it likely runs as a systemd unit or `nohup python3 …` — check `pgrep -af gamestate_service`). Confirm `/state` still returns valid JSON.
4. **Verify.** Take a screenshot of the live dashboard (use headless chromium / wkhtmltoimage / playwright — whatever's installed) showing units sitting on the navmesh polygons. Save to `analysis/navmesh_scale_fix_screenshot.png`.
5. **Report.** Post one milestone line to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`: `{"ts":"<utc>","from":"navmesh-scale-fix","text":"navmesh + entity overlay now share single bbox; <details>; screenshot at <path>"}`.

## Anti-patterns
- Don't touch the photon_tap.so build — that's a separate path. Leave `-DDISABLE_SEND_HOOKS` in place; it just confirmed the post-login crash class.
- Don't rebuild the dashboard from scratch — only fix the coordinate alignment.
- Don't disable the entity overlay or the navmesh layer to "simplify" — the user wants BOTH visible and aligned.
- Don't redeploy the Python service without verifying `/state` still serves JSON afterwards (downtime breaks the gamestate dashboard everyone watches).
- Don't drive the change through a fresh rebuild of the Rust port — production is still the Python service at `:8765`. Edit Python.

## Substrate facts
- vast.ai SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Live service: `:8765` on vast.ai, public tunnel `https://albion.orch.run/`
- Python service launch wrapper: probably `/opt/albion-gamestate/start.sh` or systemd — find with `pgrep -af gamestate_service` + `cat /proc/<pid>/cmdline`

## Memory pointers
- [[macromanage-workers]] — pick your own tactics
- [[albion-send-hooks-break-client]] — confirms why self_zone is None right now (don't try to "fix" that here)
- [[worker-artifact-isolation]] — vast.ai daemons survive turn-end on their own
