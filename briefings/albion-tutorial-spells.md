# albion-tutorial-spells — H19: tutorial step 15 spells chapter

## URGENT — 20-min cap, new chapter

H18 succeeded ~16:48Z with slow-stepped drag. Quest now: **"Spells"** chapter. Exact objective TBD from screenshot recon — likely first spell tutorial (use Q skill on dummy, or open spell book).

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 1-14 | various | 7 mechanism subclasses now (incl. drag-and-drop) | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-spells`** (branch `tutorial-spells`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_spells_advanced`
- **success metric**: quest text changes from "Spells" tutorial to next step.

## Hypothesis
Tutorial chapter "Spells" probably involves casting a basic spell. Likely candidates (in order of EV):
1. **Press Q** (or other ability key) to cast main attack: simple keypress like H17 i-key.
2. **Armed-cast pattern** (H10b): Q-key + LEFT-click on target dummy.
3. **Use auto-attack** by left-clicking on dummy.
4. **Open spell book first** (P key) then drag spell to action bar.

Read tutorial banner / quest text to identify exact mechanism.

## Substrate state (verified ~18:48)
- Veldra1203 zone "The Lighthouse"
- Inventory window still open (from H17/H18). Maybe closed by now.
- Quest panel: "Spells" (exact objective TBD)
- Action-loop STILL STOPPED
- Beginner's Broadsword EQUIPPED in main-hand

## Tasks (20-min budget)

### T0 (~3min) — Baseline + recon
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline. Read quest text carefully — may say "Cast a spell", "Press Q", "Attack the dummy", etc.
3. If inventory still open, may need to close it first (press I again or click X).
4. Touch heartbeat.

### T1 (~12min) — Mechanism enumeration
Try in order (cheapest first):
1. **Pure keypress**: `xdotool key q` (or whatever key tutorial banner mentions)
2. **Armed-cast**: `xdotool key q; sleep 0.2; mousemove <dummy_x> <dummy_y> click 1` (H10b pattern)
3. **Direct click on dummy**: `xdotool click 1` on target sprite (auto-attack)
4. **Combo**: press 1 (basic attack key) at dummy
5. **Drag from spell book**: if spell book UI is the gate, open P, find spell, slow-drag to action bar (H18 recipe), then cast

### T2 (~3min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_spells_advanced "<recipe>; artifact analysis/tutorial_spells_verdict_2026-05-25.md"` ← MUST CALL fact-set
- **Failure**: mechanism-scoped `analysis/tutorial_spells_blocked_2026-05-25.md`

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- `--repeat 2` on UI clicks.
- MUST call `harness fact-set` on success.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 7 mechanism subclasses now incl. drag
- `[[unity-real-press-required]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H18 reference verdict: `/home/sdancer/albion-tutorial-equip-broadsword/analysis/tutorial_equip_broadsword_verdict_2026-05-25.md`
- H10b reference (armed-cast pattern): `/home/sdancer/albion-tutorial-dodge-armed-cast/analysis/tutorial_dodge_armed_cast_verdict_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-spells/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_spells_advanced`
