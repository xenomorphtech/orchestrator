# albion-tutorial-sticks-and-stones — H12: tutorial step 9 gathering

## URGENT — 30-min cap, new chapter (Sticks and Stones)

H11b succeeded ~15:32Z advancing quest "Return to Wounded Crafter 0/1" → **"Sticks and Stones / Go to the marked location"** with tutorial hint **"Gathering"**.

This is a NEW tutorial chapter focused on resource gathering. Likely:
1. Walk to gathering marker (resource node = tree/rock/plant)
2. Interact (LEFT-click on the resource node) to gather

## Already achieved
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 1-8 | various | proximity / LEFT-click / armed-cast / descent | ✅ |
| 8 return-crafter | H11b descend-lane | descent + LEFT-click --repeat 2 NPC | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-sticks-and-stones`** (branch `tutorial-sticks-and-stones`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_sticks_and_stones_advanced`
- **success metric**: quest text changes from "Sticks and Stones / Go to the marked location" OR counter advances toward gathering completion.

## Hypothesis
H11b verdict reports tutorial hint "Gathering" appeared with new quest "Sticks and Stones". Standard MMO convention: "Go to the marked location" means walk to a quest-marker waypoint visible on minimap. Then interact with whatever node is there (likely a tree or rock).

Mechanism try-order:
1. Read /state.location_markers for new GATHERING/STICKS/STONES/TREE/ROCK marker
2. Right-click ground-walk toward marker (locomotion may now traverse to new area — possibly zone-transition triggers)
3. At marker: proximity-gate first; if inert, LEFT-click on visible resource sprite (tree/rock/plant)
4. Resource gathering may need press-and-hold OR repeated clicks per `[[albion-tutorial-step-advance-recipe]]` mechanism subclasses

## Substrate state (verified ~17:32)
- Veldra1203 at (10.08, -59.62) zone "The Lighthouse" (post-H11b dialogue)
- Quest panel: "Sticks and Stones / Go to the marked location"
- Tutorial hint: "Gathering" visible
- Action-loop STILL STOPPED

## Tasks (30-min budget)

### T0 (~3min) — Baseline + marker recon
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root analysis/t0_baseline.png'`
3. Inspect /state.location_markers and minimap visually for "marked location" indicator
4. Touch heartbeat

### T1 (~15min) — Locomotion to marked location
1. Right-click ground-walk toward visible quest marker (minimap arrow likely shows direction)
2. Poll /state every 3s for position delta. If zone changes mid-walk, screenshot and reassess.
3. Hold 10s at marker for proximity-gate test.

### T2 (~8min) — Gathering interaction
If proximity-gate didn't advance:
1. LEFT-click visible resource sprite (tree/rock) at marker — use `xdotool click --repeat 2 1` per Unity-UI-drift workaround
2. If no progress, try press-and-hold mousedown (Albion gathering often uses hold-to-channel)
3. Check for E/F prompt at NPC ring (interaction keys)

### T3 (~4min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_sticks_and_stones_advanced "<recipe>; artifact analysis/tutorial_sticks_and_stones_verdict_2026-05-25.md"`
- **Failure**: mechanism-scoped `analysis/tutorial_sticks_and_stones_blocked_2026-05-25.md` with ≥3 untried siblings

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- Use `--repeat 2` on UI button clicks (Unity drift).

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 4 mechanism subclasses now; new candidate: gathering-interaction
- `[[unity-real-press-required]]` — gathering may need mousedown-hold-mouseup
- `[[npc-drift-contamination]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H11b verdict (descent reference): `/home/sdancer/albion-tutorial-return-crafter-descend-lane/analysis/tutorial_return_crafter_descend_verdict_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-sticks-and-stones/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_sticks_and_stones_advanced`
