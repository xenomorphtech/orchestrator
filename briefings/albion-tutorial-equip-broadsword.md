# albion-tutorial-equip-broadsword — H18: tutorial step 14 equip broadsword (drag-and-drop)

## URGENT — 20-min cap, NEW mechanism subclass (drag-and-drop)

H17 succeeded ~16:43Z with `xdotool key i`. Inventory opened, Broadsword visible. Quest now: **"Equipping / Equip Items by dragging and dropping them into the proper inventory slot."**

This is a candidate **7th mechanism subclass**: drag-and-drop (distinct from single-click, double-click, hold).

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 1-13 | various | 6 mechanism subclasses + I-key keypress | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-equip-broadsword`** (branch `tutorial-equip-broadsword`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_equip_broadsword_advanced`
- **success metric**: quest text changes from "Equipping" to next step OR Broadsword visible in weapon slot.

## Hypothesis
Drag-and-drop Beginner's Broadsword from inventory bag → weapon slot on character paperdoll.

Standard xdotool drag recipe (PRIMARY mechanism — try first):
```
xdotool mousemove <src_x> <src_y> mousedown 1
xdotool mousemove <dst_x> <dst_y>
xdotool mouseup 1
```

The Broadsword icon is in the bag (right panel); weapon slot is typically on the character paperdoll (left panel) — main-hand slot.

## Substrate state (verified ~18:43)
- Veldra1203 zone "The Lighthouse"
- Inventory window OPEN
- Quest panel: "Equipping / Equip Items by dragging and dropping them into the proper inventory slot."
- Tutorial banner: "Equipping"
- Action-loop STILL STOPPED

## Tasks (20-min budget)

### T0 (~3min) — Baseline + UI recon
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline: `import -window root analysis/t0_baseline.png`
3. Identify pixel coords for:
   - Beginner's Broadsword in bag (likely right-panel, first available bag slot)
   - Empty main-hand weapon slot on character paperdoll (likely left-panel, weapon slot icon)
4. Touch heartbeat

### T1 (~12min) — Drag-and-drop mechanism enumeration
Primary recipe:
1. `xdotool mousemove <broadsword_x> <broadsword_y> mousedown 1; sleep 0.2; mousemove <weapon_slot_x> <weapon_slot_y>; sleep 0.2; mouseup 1`
2. Verify Broadsword now in weapon slot via screenshot.

If primary inert, try alternates (3+ siblings per `[[falsify-mechanism-not-path]]`):
- **right-click on Broadsword**: Albion convention often has right-click → auto-equip.
- **double-LEFT-click on Broadsword** (`--repeat 2`): may auto-equip.
- **slow drag**: `mousedown 1` → multiple `mousemove` steps with sleeps in between (simulate real drag motion) → `mouseup 1`.
- **Drag with shift held**: some Unity games need modifier.

### T2 (~3min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_equip_broadsword_advanced "<recipe>; artifact analysis/tutorial_equip_broadsword_verdict_2026-05-25.md"` ← REMEMBER TO CALL fact-set (H16 worker forgot)
- **Failure**: mechanism-scoped `analysis/tutorial_equip_broadsword_blocked_2026-05-25.md` listing ≥3 untried drag-class siblings.

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape (inventory will close).
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- MUST call `harness fact-set` on success per orchestrator protocol.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 6 subclasses; drag-and-drop is candidate 7th
- `[[unity-real-press-required]]` — drag may need slower mousedown→mousemove→mouseup
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H17 reference verdict: `/home/sdancer/albion-tutorial-open-inventory/analysis/tutorial_open_inventory_verdict_2026-05-25.md`
- Inventory layout reference screenshot: `/home/sdancer/albion-tutorial-open-inventory/analysis/t1_after_key_i.png`
- Worktree: `/home/sdancer/albion-tutorial-equip-broadsword/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_equip_broadsword_advanced`
