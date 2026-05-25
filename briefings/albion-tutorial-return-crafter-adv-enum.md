# albion-tutorial-return-crafter-adv-enum — adversarial-pair for H11 mechanism-dropped

## URGENT — 30-min HARD CAP, ENUMERATION ONLY (no execution)

H11 was marked mechanism-dropped 2026-05-25 ~16:40Z by worker `albion-tutorial-return-crafter`. Closure: `/home/sdancer/albion-tutorial-return-crafter/analysis/tutorial_return_crafter_blocked_2026-05-25.md`. Worker stuck at locomotion dead-end (-1.75,-41.25) with Δ=+13.46 east, -15.77 south remaining to NPC (11.71,-57.02). 9 east/SE right-click ground moves stalled; 1 LEFT-click on candidate sprite (1310,170) didn't open dialogue.

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-return-crafter-adv-enum`** (branch `tutorial-return-crafter-adv-enum`).

## Mandate

Read the original verdict at `/home/sdancer/albion-tutorial-return-crafter/analysis/tutorial_return_crafter_blocked_2026-05-25.md` first. Then enumerate ≥3 untried mechanisms across these three classes:

### Class A: Locomotion alternatives (worker exhausted right-click ground in narrow band)
The worker named 3 untried siblings: (1) cardinal stuck-break clicks (N/W/S), (2) visual re-localization from final_screen.png, (3) lower walkable lane visual targeting. Validate these + propose ≥3 more:
- Keyboard WASD locomotion (Albion supports it?)
- Minimap-click destination
- Multi-hop: click halfway → re-click further (path-around-obstacle)
- Click on NPC sprite directly to auto-walk (Albion convention)
- Alternative angle (NW then SE)
- /follow chat command if Crafter is visible

### Class B: NPC interaction alternatives (only 1 sprite-click tried)
Worker tried LEFT-click at (1310,170). Untried sprite siblings worker named: (1310,150), (1310,200). Add ≥3 more:
- Right-click on NPC sprite
- Double-click on NPC sprite
- Mousedown-hold-mouseup (per `[[unity-real-press-required]]`)
- LEFT-click with `--repeat 2` for Unity UI button drift
- Keyboard interact key (E, F, T, Space) at NPC ring
- Quest tracker click in HUD (if visible)

### Class C: Higher-substrate (skip locomotion entirely)
- Frida hook to teleport Veldra coords directly
- Game-console / chat-command for `/teleport` or `/return`
- Click on quest panel "Return to Wounded Crafter" entry to auto-path
- NPC indicator on minimap to set waypoint

Enumerate ≥3 from Class C.

## Output format (REQUIRED)

Write `/home/sdancer/albion-tutorial-return-crafter-adv-enum/analysis/albion-tutorial-return-crafter_adversarial_alternatives.md` with the same 3-class table structure as the previous adv-enum (see `[[albion-tutorial-dodge-adv-enum]]` output for format). Include Verdict section with: net-new count, most-recommended next-path, recommended next-path name.

## Constraints (HARD)
- **NO EXECUTION.** Read + write only.
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- 30-min hard cap.

## Files
- Original verdict: `/home/sdancer/albion-tutorial-return-crafter/analysis/tutorial_return_crafter_blocked_2026-05-25.md`
- Worker's probe artifacts: `/home/sdancer/albion-tutorial-return-crafter/analysis/probe_*.png`, `npc_try_170.png`, `final_screen.png`
- Worktree: `/home/sdancer/albion-tutorial-return-crafter-adv-enum/`
- Output: `/home/sdancer/albion-tutorial-return-crafter-adv-enum/analysis/albion-tutorial-return-crafter_adversarial_alternatives.md`
