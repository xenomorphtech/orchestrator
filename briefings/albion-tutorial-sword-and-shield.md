# albion-tutorial-sword-and-shield — H15: tutorial step 11 Blacksmith chapter

## URGENT — 30-min cap, new chapter (Sword and Shield)

H14 succeeded ~16:11Z advancing "Return to Wounded Crafter 0/1" → **"Sword and Shield / Go to the Blacksmith."** New tutorial chapter. Likely involves walking to a Blacksmith NPC marker and interacting.

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 1-10 | various | proximity / LEFT-click / armed-cast / descent / two-stage gather / NPC turn-in | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-sword-and-shield`** (branch `tutorial-sword-and-shield`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_sword_and_shield_advanced`
- **success metric**: quest text changes from "Sword and Shield / Go to the Blacksmith" OR counter ticks toward objective.

## Hypothesis
"Go to the Blacksmith" = walk to a Blacksmith NPC marker. Standard MMO convention:
1. Quest marker appears on minimap / world; locate via /state markers or visible quest arrow.
2. Locomotion to marker via right-click ground.
3. At marker: proximity-gate first, then LEFT-click NPC sprite if proximity inert.

Recipe is now well-mature (10 steps banked); should resolve in 5-10min.

## Substrate state (verified ~18:12)
- Veldra1203 at (11.84, -57.07) zone "The Lighthouse" (post-H14 dialogue)
- Quest panel: "Sword and Shield / Go to the Blacksmith."
- Action-loop STILL STOPPED

## Tasks (30-min budget)

### T0 (~3min) — Baseline + marker recon
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline
3. Check /state.location_markers for BLACKSMITH / FORGE / SMITH / NPCSMITH
4. Visually identify quest marker on screen (likely yellow arrow or "?" on minimap)
5. Touch heartbeat

### T1 (~15min) — Locomotion to Blacksmith marker
1. Right-click ground toward marker. Hop in 5-10 unit increments.
2. Poll /state every 3s for position delta.
3. At within 5 world-units of target: hold 10s for proximity-gate test.

### T2 (~8min) — NPC interaction (if proximity inert)
1. LEFT-click --repeat 2 on Blacksmith NPC sprite
2. If dialogue opens: Complete + Accept buttons (--repeat 2 each)

### T3 (~4min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_sword_and_shield_advanced "<recipe>; artifact analysis/tutorial_sword_and_shield_verdict_2026-05-25.md"`
- **Failure**: mechanism-scoped `analysis/tutorial_sword_and_shield_blocked_2026-05-25.md`

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- `--repeat 2` on UI/sprite clicks.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 5+ mechanism subclasses; "go to marker" pattern matches Step 4 (look-for-ship) and Step 9 (gather marker)
- `[[unity-real-press-required]]`
- `[[npc-drift-contamination]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H14 reference verdict: `/home/sdancer/albion-tutorial-return-crafter-2/analysis/tutorial_return_crafter_2_verdict_2026-05-25.md`
- H7 look-for-ship reference (also "go to" pattern): `/home/sdancer/albion-tutorial-look-for-ship/analysis/`
- Worktree: `/home/sdancer/albion-tutorial-sword-and-shield/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_sword_and_shield_advanced`
