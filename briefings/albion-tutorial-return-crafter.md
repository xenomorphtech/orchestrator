# albion-tutorial-return-crafter — H11: tutorial step 8 'Return to Wounded Crafter'

## URGENT — 30-min cap, NPC interaction (H2-style)
H10b succeeded 2026-05-25 ~16:10Z via Class B #1 compound mechanism (F+L-click). Quest advanced "Use the Dodge ability" → **"Return to Wounded Crafter in The Lighthouse. 0/1"**.

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | NPC/objective coords | Status |
|---|---|---|---|---|
| 2 Talk-to-survivor | H2 | LEFT-click on NPC sprite | Wounded Crafter @ world (11.71,-57.02) | ✅ |
| 3-7 | H5/H7/H8/H9/H10b | proximity-gate / locomotion / armed-cast | various | ✅ |

**Reusable recipe**: locomotion + LEFT-click NPC sprite. Same pattern as H2.

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-return-crafter`** (branch `tutorial-return-crafter`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_return_to_wounded_crafter_advanced`
- **success metric**: quest text changes from "Return to Wounded Crafter" to next step OR counter 0/1 → 1/1.

## Hypothesis
Return to Wounded Crafter NPC = same NPC as H2 (Wounded Crafter at world (11.71,-57.02)). Walk Veldra back east, then LEFT-click on NPC sprite with "?" indicator. May open dialogue panel → click "Complete" button.

## Substrate state (verified ~16:10)
- Veldra1203 at (-11.19, -66.25) zone "The Lighthouse"
- Quest panel: "Return to Wounded Crafter in The Lighthouse. 0/1"
- Action-loop STILL STOPPED — keep stopped
- Wounded Crafter @ world (11.71, -57.02) per H2 success fact

## Trajectory
Δx=+22.9 east + Δz=+9.2 north from current (-11.19,-66.25) → target (11.71,-57.02). NPC may have moved during cutscene; re-localize via screenshot once close.

## Tasks (30-min budget)

### T0 (~3min) — Baseline
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root analysis/t0_baseline.png'`
3. Check /state location_markers for INTERACTWITHCRAFTER / CRAFTER / QUESTNPC
4. Touch heartbeat

### T1 (~15min) — Locomotion to Crafter ring
1. Right-click ground-moves toward (11.71,-57.02). Use isometric projection (east on screen = +x in world).
2. Poll /state every 3s. Stop when within 5 world-units of NPC marker.
3. Hold 10s for proximity-gate test (recipe step 6a). Screenshot + check quest text.

### T2 (~7min) — LEFT-click NPC sprite (recipe step 6b)
If proximity-gate fails:
1. Screenshot at Crafter ring. Find NPC sprite pixel coords (visually identify Wounded Crafter; she has "?" indicator above head when interactable).
2. `xdotool mousemove <obj_x> <obj_y> click 1` on NPC sprite (NOT on Veldra's feet — see [[npc-drift-contamination]]).
3. If dialogue panel opens, click "Complete" button — may need `--repeat 2` or sleep between tries (Unity UI button drift).

### T3 (~5min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_return_to_wounded_crafter_advanced "<mechanism>; artifact analysis/tutorial_return_crafter_verdict_2026-05-25.md"` + commit verdict.
- **Failure**: mechanism-scoped `analysis/tutorial_return_crafter_blocked_2026-05-25.md` with ≥3 untried siblings.

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- Per-frame template-match: NPC screen-coords drift between turns — re-localize each tick rather than trusting inherited coords (this is the lesson from contamination revival).

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — validated 6 steps now, with new mechanism class (c) armed-ability cast added
- `[[npc-drift-contamination]]` — re-localize per-frame; don't trust inherited NPC coords
- `[[falsify-mechanism-not-path]]`
- `[[albion-tutorial-clickclass]]` — NPC interactions need LEFT-click on sprite, not proximity

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-return-crafter.md`
- Worktree: `/home/sdancer/albion-tutorial-return-crafter/`
- H2 verdict (reference for Crafter NPC interaction): `/home/sdancer/albion-tutorial-advance-localized-walk/analysis/tutorial_advance_walk_verdict_2026-05-25.md`
- H10b verdict (immediate predecessor): `/home/sdancer/albion-tutorial-dodge-armed-cast/analysis/tutorial_dodge_armed_cast_verdict_2026-05-25.md`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_return_to_wounded_crafter_advanced`
