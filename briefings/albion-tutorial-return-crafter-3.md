# albion-tutorial-return-crafter-3 — H20: tutorial step 16 third NPC turn-in

## URGENT — 15-min cap, well-proven recipe (3rd time)

H19 succeeded ~18:53Z binding broadsword spell to Q. Quest now: **"Sword and Shield / Return to Wounded Crafter in The Lighthouse. 0/1"** — same NPC turn-in as H11b (step 8) and H14 (step 10). Third reuse.

## Already achieved (DO NOT re-falsify)
| Step | Worker | Recipe | Status |
|---|---|---|---|
| 8 return-crafter | H11b | descend + LEFT-click --repeat 2 NPC + Complete + Accept | ✅ |
| 10 return-crafter-2 | H14 | right-click ground + LEFT-click --repeat 2 NPC + Complete + Accept | ✅ |
| 15 spells | H19 | LEFT-click --repeat 2 on spell icon at (876,483) | ✅ |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-return-crafter-3`** (branch `tutorial-return-crafter-3`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_return_to_wounded_crafter_3_advanced`
- **success metric**: quest text changes from "Return to Wounded Crafter" to next step.

## Hypothesis
Reuse H14 recipe verbatim: locomotion to Crafter at (11.71,-57.02) + LEFT-click --repeat 2 on NPC sprite + Complete + Accept. The Crafter NPC may be on screen already (if H19 closed inventory near her).

## Substrate state (verified ~18:53)
- Veldra1203 zone "The Lighthouse"
- Quest panel: "Sword and Shield / Return to Wounded Crafter in The Lighthouse. 0/1"
- Action-loop STILL STOPPED
- Broadsword equipped, Q hotbar slot bound

## Tasks (15-min budget)

### T0 (~2min) — Baseline + check Crafter visibility
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline. Check if Wounded Crafter NPC visible (with `?` indicator).
3. Touch heartbeat.

### T1 (~10min) — NPC interaction
1. If Crafter visible: LEFT-click --repeat 2 directly on NPC sprite.
2. If NOT visible: right-click ground west/southwest (per H14 pattern at (300,500) or similar) to reposition, then click NPC.
3. Dialogue opens → LEFT-click --repeat 2 Complete button → LEFT-click --repeat 2 Accept button.

### T2 (~3min) — Verdict + fact
- **Success**: `harness fact-set albion_tutorial_step_return_to_wounded_crafter_3_advanced "<recipe>; artifact analysis/tutorial_return_crafter_3_verdict_2026-05-25.md"` ← MUST CALL fact-set
- **Failure**: mechanism-scoped `analysis/tutorial_return_crafter_3_blocked_2026-05-25.md`

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- MUST call `harness fact-set` on success.

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — 7 mechanism subclasses incl. NPC dialogue with Complete/Accept retries
- `[[unity-real-press-required]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H14 reference verdict: `/home/sdancer/albion-tutorial-return-crafter-2/analysis/tutorial_return_crafter_2_verdict_2026-05-25.md`
- H11b reference: `/home/sdancer/albion-tutorial-return-crafter-descend-lane/analysis/tutorial_return_crafter_descend_verdict_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-return-crafter-3/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_return_to_wounded_crafter_3_advanced`
