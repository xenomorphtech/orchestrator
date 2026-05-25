# albion-tutorial-craft-broadsword — H16: tutorial step 12 crafting

## URGENT — 20-min cap, crafting UI

H15 succeeded ~16:23Z. Forge UI `Makeshift Warrior's Forge` is open with quest **"Crafting / Select the Beginner's Broadsword in the Blacksmith's inventory."**

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 1-11 | various | proximity / LEFT-click / armed-cast / descent / two-stage gather / NPC dialogue / forge-NPC double-click | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-craft-broadsword`** (branch `tutorial-craft-broadsword`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_craft_broadsword_advanced`
- **success metric**: quest text changes from "Crafting / Select the Beginner's Broadsword" to next step (likely "Craft the Broadsword" or "Equip the Broadsword").

## Hypothesis
Forge UI is open. Quest objective is to select Beginner's Broadsword in the inventory list, then craft it. Standard MMO crafting UI: 
1. Recipe list on side; click Broadsword entry
2. Click "Craft" button (often bottom-right)
3. Wait for crafting animation
4. Quest auto-advances on craft completion

Establishes **6th mechanism subclass**: crafting-UI interaction (distinct from world-sprite click / NPC dialogue / dialogue button).

## Substrate state (verified ~18:23)
- Veldra1203 at (6.10, -54.01) zone "The Lighthouse"
- Forge UI open: `Makeshift Warrior's Forge`
- Quest panel: "Crafting / Select the Beginner's Broadsword in the Blacksmith's inventory."
- Action-loop STILL STOPPED

## Tasks (20-min budget)

### T0 (~3min) — Baseline + UI recon
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root analysis/t0_baseline.png'`
3. Inspect forge UI — identify Beginner's Broadsword entry pixel coords and Craft button
4. Touch heartbeat

### T1 (~10min) — Select Broadsword + Craft
1. LEFT-click `--repeat 2` on Beginner's Broadsword entry in inventory list
2. Verify item highlighted via screenshot
3. LEFT-click `--repeat 2` on Craft button
4. Wait for craft animation (5-10s)
5. Screenshot post-craft; check quest text

### T2 (~4min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_craft_broadsword_advanced "<recipe>; artifact analysis/tutorial_craft_broadsword_verdict_2026-05-25.md"`
- **Failure**: mechanism-scoped `analysis/tutorial_craft_broadsword_blocked_2026-05-25.md`

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape (esp. during crafting UI — could close forge).
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- `--repeat 2` on UI clicks.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 5 mechanism subclasses; H16 may establish 6th
- `[[unity-real-press-required]]` — Unity UI buttons may need mousedown-hold-mouseup
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H15 reference: `/home/sdancer/albion-tutorial-sword-and-shield/analysis/tutorial_sword_and_shield_verdict_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-craft-broadsword/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_craft_broadsword_advanced`
