# albion-tutorial-sticks-and-stones-complete — H13: complete gather sub-quest

## URGENT — 30-min cap, gather-grind continuation

H12 succeeded ~15:38Z advancing "Go to the marked location" → gather counters. PARTIAL state: `Gather Rough Stone 2/6 + Gather Rough Logs 0/6`. H13's job: complete the gathering grind so quest text advances past the counters.

## Already achieved (recipe verified)
| Probe | Recipe | Result |
|---|---|---|
| H12 #1 | right-click (1475,355) | locomotion to marker, quest flipped to counters |
| H12 #2 | LEFT-click --repeat 2 (885,380) on rock | Rough Stone 0/6 → 2/6 |

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-sticks-and-stones-complete`** (branch `tutorial-sticks-and-stones-complete`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_step_sticks_and_stones_completed`
- **success metric**: quest text changes from "Gather Rough Stone X/6 + Gather Rough Logs Y/6" to the next chapter step.

## Hypothesis
Repeat the H12 click-on-resource recipe ~10 more times until both counters fill: 4 more Rough Stone + 6 Rough Logs. Albion typically auto-advances quest when both counters hit 6/6 — may need to return to NPC after.

## Substrate state (verified ~17:38)
- Veldra1203 at (14.42, -46.11) zone "The Lighthouse"
- Quest counters: Rough Stone 2/6, Rough Logs 0/6
- Action-loop STILL STOPPED

## Tasks (30-min budget)

### T0 (~2min) — Baseline
1. `curl -s http://127.0.0.1:8765/state | python3 -m json.tool > analysis/t0_state.json`
2. Screenshot baseline
3. Identify visible Rough Stone + Rough Log sprites in viewport
4. Touch heartbeat

### T1 (~15min) — Gather Rough Stone 2→6
For each remaining stone (4 needed):
1. LEFT-click --repeat 2 on next visible highlighted Rough Stone sprite. If none visible nearby, right-click ground east/south to walk to one.
2. Verify counter increments via screenshot or /state polling.
3. Continue until 6/6.

### T2 (~10min) — Gather Rough Logs 0→6
Similar pattern — find visible Rough Log (tree) sprites and LEFT-click --repeat 2 on each.
- Logs sprites are usually trees (tall green sprite vs stone's gray rock cluster)
- May need to walk further if no logs near current location

### T3 (~3min) — Verdict + fact
- **Success**: When both counters at 6/6, quest text should advance. `harness fact-set albion_tutorial_step_sticks_and_stones_completed "<recipe>; artifact analysis/tutorial_sticks_complete_verdict_2026-05-25.md"` + verdict file.
- **Partial fallback**: If stuck partway (e.g. 6/6 stones but 0/6 logs visible), commit `analysis/tutorial_sticks_partial_blocked_2026-05-25.md` with progress + ≥3 untried siblings (e.g. explore further, return to NPC for hint).

### Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- action-loop STAYS STOPPED.
- NEVER spam Escape.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- `--repeat 2` on all UI/sprite clicks (Unity drift workaround).
- Gathering may require small wait between clicks (Albion has gathering animation duration ~2-3s).

## Memory references
- `[[albion-tutorial-step-advance-recipe]]` — try-order including elevation-aware locomotion + Unity --repeat 2
- `[[unity-real-press-required]]`
- `[[falsify-mechanism-not-path]]`

## Files / endpoints
- H12 verdict (recipe reference): `/home/sdancer/albion-tutorial-sticks-and-stones/analysis/tutorial_sticks_and_stones_verdict_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-sticks-and-stones-complete/`
- /state: `http://127.0.0.1:8765/state`
- Success fact: `albion_tutorial_step_sticks_and_stones_completed`
