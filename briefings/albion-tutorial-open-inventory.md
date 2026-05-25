# albion-tutorial-open-inventory — H17: tutorial step 13 open inventory

## URGENT — 15-min cap, simple keypress (per tutorial hint)

H16 succeeded ~16:38Z. Quest now: **"Open your Inventory by pressing the glowing button."** Tutorial banner says shortcut: I.

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 1-12 | various | 6 mechanism subclasses (proximity / LEFT-click / armed-cast / descent / two-stage gather / crafting-UI) | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-open-inventory`** (branch `tutorial-open-inventory`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_open_inventory_advanced`
- **success metric**: quest text changes from "Open your Inventory" to next step (likely "Equip the Broadsword").

## Hypothesis
Tutorial banner explicitly says "Shortcut: I". Press I key. If keypress inert, LEFT-click --repeat 2 on the glowing inventory button (likely bottom-right HUD).

This is the simplest step in the campaign — just press I.

## Substrate state (verified ~18:38)
- Veldra1203 zone "The Lighthouse"
- Quest panel: "Open your Inventory by pressing the glowing button."
- Tutorial banner: "Your Inventory — You can find your Beginner's Broadsword in your inventory (Shortcut: I)."
- Action-loop STILL STOPPED

## Tasks (15-min budget)

### T0 (~2min) — Baseline
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline
3. Touch heartbeat

### T1 (~5min) — Press I key
1. `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool key i'`
2. Wait 2s, screenshot
3. If inventory window opens AND quest text changes → success
4. If keypress inert, try variants: `xdotool key --window 0x200006 i`, capital I, `--repeat 2 i`

### T2 (~5min) — Click glowing button if keypress inert
1. Identify glowing button on HUD (likely bottom-right, may have animated glow ring)
2. LEFT-click `--repeat 2` on button
3. Verify inventory opens via screenshot

### T3 (~3min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_open_inventory_advanced "<recipe>; artifact analysis/tutorial_open_inventory_verdict_2026-05-25.md"`
- **Failure**: mechanism-scoped `analysis/tutorial_open_inventory_blocked_2026-05-25.md`

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 6 mechanism subclasses now
- `[[unity-real-press-required]]` — if I-key inert, try mousedown-hold-mouseup on button
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H16 reference verdict: `/home/sdancer/albion-tutorial-craft-broadsword/analysis/tutorial_craft_broadsword_verdict_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-open-inventory/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_open_inventory_advanced`
