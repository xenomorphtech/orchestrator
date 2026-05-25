# Revivals — paths previously falsified that were proved reachable

Format per orchestrate skill:
```
- YYYY-MM-DD — original `<path>` (dropped YYYY-MM-DD by `<wname>`, mechanism `<mech>`).
  Revived as `<new-path>` via mechanism `<new-mech>`.
  Evidence: `<artifact path + sha256 prefix>`.
  Root cause of premature closure: <one line>.
```

---

- 2026-05-25 — original `albion-tutorial-advance` (XTest LEFT-click mechanism, mechanism-dropped 2026-05-25 by self-orchestrator citing 5 XTest probes failed at coords (432, [730,750,760,790])). Also covers `albion-tutorial-vncdo` (mechanism-dropped same day, 4 vncdo probes at same coords).
  Revived as `albion-tutorial-advance-localized-walk` via mechanism `xdotool LEFT-click on NPC sprite with proper experimental control (action-loop STOPPED + per-frame NPC re-localization via screenshot inspection + isometric-projection-aware locomotion to NPC)`.
  Evidence: `/home/sdancer/albion-tutorial-advance-localized-walk/analysis/t3_post_success.png` — quest panel shows "First Steps ✅ Talk to the survivor (1/1)" + Crafter dialogue panel open + "?" NPC indicator. Worker codex thread 019e5ec1-2a2f-7eb0. Player at (11.71, -57.02) in @TUTORIALSINGLE@45ba084d.
  Root cause of premature closure: **`[[npc-drift-contamination]]`** — prior probes targeted (432, 750) which was Veldra's FEET, not the NPC sprite. The action-loop daemon's autonomous walking shifted Veldra between worker turns, invalidating inherited NPC screen-coords. The mechanism class (XTest LEFT-click) was NEVER actually exercised against the NPC; the path was closed prematurely because the orchestrator pattern-matched "5 probes failed" without verifying click targets were correct. Fix structurally embedded in K=6 planner output 2026-05-25 11:37Z (per `last_planner_cycle_2026-05-25-11` fact) which mandated paused-action-loop + per-frame template-match.

- 2026-05-25 — original `albion-tutorial-dodge` (bare-keypress mechanism: xdotool key F + xdotool focus+keydown/keyup + ydotool uinput, mechanism-dropped 2026-05-25 ~16:00Z by worker `albion-tutorial-dodge` citing 3 input-injection siblings all inert at static player coords).
  Revived as `albion-tutorial-dodge-armed-cast` via mechanism `compound (F-arms-targeter + LEFT-click-ground commits cast)` — specifically `xdotool key f; sleep 0.2; xdotool mousemove 960 600 click 1`.
  Evidence: `/home/sdancer/albion-tutorial-dodge-armed-cast/analysis/tutorial_dodge_armed_cast_verdict_2026-05-25.md` — fact `albion_tutorial_step_dodge_advanced` set 2026-05-25T14:10:29Z; quest text transition from "Use the Dodge ability" to "Return to Wounded Crafter in The Lighthouse. 0/1"; player position delta (-13.25,-65.25) → (-11.19,-66.25). Codex thread 019e5fa0-XXXX.
  Root cause of premature closure: **mechanism-class mis-scoping** — original H10 worker tested only the BARE keypress class, but Albion's Dodge boot ability is a *directional skill* requiring a commit action (LEFT-click ground for direction). The adversarial-pair worker correctly identified compound (F+commit) as a DISTINCT mechanism class (Class B in enumeration), and B#1 fired Dodge on first attempt. The original mechanism-dropped row IS valid — bare-keypress IS exhausted; what was missed was that the broader path class includes compound mechanisms. Caught WITHIN-SESSION via adversarial-pair safeguard (rule working as designed; 30min budget unused, completed in 126s).

- 2026-05-25 — original `albion-tutorial-return-crafter` (right-click ground locomotion at upper-plank dead-end, mechanism-dropped 2026-05-25 ~16:40Z citing 9 east/SE probes + 1 LEFT-click NPC sprite at (1310,170) inert).
  Revived as `albion-tutorial-return-crafter-descend-lane` via mechanism `2-stage descend-then-approach + LEFT-click --repeat 2 on NPC sprite` — specifically: right-click west (420,360) released upper plank; 4 sequential descent right-clicks at (700,840), (930,900), (1000,940), (1520,872) traversed lower stair lane; LEFT-click `--repeat 2` on NPC torso (960,150) opened dialogue; Complete + Accept finalized.
  Evidence: `/home/sdancer/albion-tutorial-return-crafter-descend-lane/analysis/tutorial_return_crafter_descend_verdict_2026-05-25.md` — fact `albion_tutorial_step_return_to_wounded_crafter_advanced` set 2026-05-25T15:31:59Z; quest text transition from "Return to Wounded Crafter 0/1" to "Sticks and Stones / Go to the marked location" (tutorial hint "Gathering"); player position traversal (-1.75,-41.25)→(-14.63,-49.07)→(-14.73,-58.11)→(-9.17,-63.94)→(-1.58,-69.11)→(13.59,-62.50)→(10.08,-59.62). Codex thread 019e5fed-xxxx.
  Root cause of premature closure: **wrong-elevation mis-diagnosis** — original H11 worker treated dead-end as east-locomotion failure, but Veldra was stranded on UPPER lighthouse plank; adv-enum correctly diagnosed via final_screen.png inspection as a Class A descent problem (echoes H5 climb-tower "wrong elevation, not no-path" learning). Also: `--repeat 2` LEFT-click overcame Unity UI button drift that single-click missed. **New mechanism axis discovered**: elevation-aware locomotion (4th in tutorial recipe taxonomy after proximity-gate, LEFT-click sprite, armed-cast).
