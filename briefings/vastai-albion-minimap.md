# vastai-albion-minimap — V3: render quest goals on the minimap

## Role & workdir
Codex worker, workdir `/home/sdancer/vastai-albion-minimap`. **V1 done** (commit `5c4bdaf` ALBION_MINIMAP_DONE): ws_bridge.py + canvas frontend on `http://127.0.0.1:8789/`. **V2 done** (commit `3023b58`): status panel + ws watcher for `/tmp/albion_status.json`. **V3 NEW** (this briefing): render **quest goals** on the minimap canvas — sourced from the sibling tables worker's `/tmp/albion_events.jsonl`.

## User directive (verbatim, 2026-05-19 ~18:13)
> "make a minimap" + "identify quest goals" (sibling tables worker handles identification; you render).

## Current substrate
- Your existing `ws_bridge.py` runs as a long-lived process on `127.0.0.1:8788` (WS) + `8789` (HTTP). Currently PID 1303725 with `--loop --serve-frontend --status-file /tmp/albion_status.json` (started cycle 2290).
- Sonnet is now playing the game → status JSON updates → banner already renders state/player_name/pos.
- Sibling worker `vastai-albion-tables` (codex_thread, V2 in flight) will stream quest events to `/tmp/albion_events.jsonl` as the live tcpdump capture decodes. Each line is a JSON quest_state event with `{quest_id, quest_name, step, goal, target_pos, target_npc, target_mob_type}`.

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-minimap-quest-goal-rendering`

## Success criteria
- Canvas renders the player's current position (from `albion_status.json.pos`) as a centered dot.
- Active quest goals appear as colored markers on the canvas:
  - **kill mobs** → red X on `target_pos`
  - **deliver to NPC** → green flag on `target_pos` with `target_npc` name
  - **explore region** → yellow circle on `target_pos`
- Side panel lists active quests by `quest_name` with current `step` and one-line `goal` text.
- Quest events arriving in real time (via WS broadcast) update the canvas and side panel.
- Verdict at `analysis/albion_minimap_v3_verdict.md` (≤150 lines). Final line `ALBION_MINIMAP_V3_DONE`.
- Fact `albion_minimap_v3_quest_render_<date>` set.

## Concrete tasks (do in order)

1. **Add quest-event tail to ws_bridge.py**: extend the existing `watch_status_file` pattern to also tail `/tmp/albion_events.jsonl` (appending file — track byte offset). Each new line, broadcast as `{"kind":"quest_state", ...payload}` to all WS clients. Make `--quest-events` flag default `/tmp/albion_events.jsonl`. Keep watcher in its own asyncio task. (~40 LoC change.)

2. **Add `analysis/wire_format_v3.md`** documenting the `quest_state` frame (appending to v2 spec, do NOT rewrite v2).

3. **Frontend update — `frontend/index.html`**:
   - Track an `activeQuests` map keyed by `quest_id`.
   - On `quest_state` frame: if `step==="complete"` → remove from map and add to a "completed quests" log row; else update entry with latest `goal` + `target_pos` + `target_npc`.
   - Render goal markers on canvas using the world→screen transform you already have:
     - red X for kill-mob (`target_mob_type !== null`)
     - green flag for deliver-to-NPC (`target_npc !== null`)
     - yellow circle for explore (neither set)
   - Add a vertical right-side panel (max-width 30% of canvas) listing active quests: `quest_name | step | goal` rows.
   - Player position dot stays centered (from `status` frame's `pos`).
   - All vanilla JS + canvas, NO npm. ≤400 LoC total for index.html (was ~200 in v2).

4. **Smoke test**: hand-write 3 lines to `/tmp/albion_events.jsonl` (mix of kill/deliver/explore). Watch the WS bridge broadcast them. Open `http://127.0.0.1:8789/` via headless Chromium screenshot (use the same recipe as v2 smoke test — see `analysis/albion_minimap_frontend_smoke.png`). Confirm markers + panel appear.

5. **Verdict + fact**: `analysis/albion_minimap_v3_verdict.md`, final line `ALBION_MINIMAP_V3_DONE`. Write fact to `analysis/fact_to_set.txt` (key `albion_minimap_v3_quest_render_2026_05_19`). Single commit on branch `codex-albion-minimap` named `Render quest goals on minimap (v3)`.

## Constraints
- The current ws_bridge process is PID 1303725. When you commit, the orchestrator will kill+restart it with the new code. Don't manage that process yourself.
- Frontend stays vanilla JS — no React/Vue/npm.
- Player position from `status.pos` may be null while sonnet's pos-extractor isn't shipping yet — gracefully fall back (e.g. center canvas, no player dot, but still render quest markers).
- Don't break v2 — banner status pill must still render even if no quest events arrive.

## Memory references
- `[[albion-minimap-webapp-v1-shipped-2026-05-19]]`, `[[albion-dashboard-v2-shipped-2026-05-19]]`.
- v1 wire format: `analysis/wire_format_v1.md`.
- v2 spec: `analysis/wire_format_v2.md`.
- v3 spec: `analysis/wire_format_v3.md` (you'll create).
- Sibling tables worker emits to: `/tmp/albion_events.jsonl`.
