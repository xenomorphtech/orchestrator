# albion-tutorial-skip-menu — H1 from K=6 planner: menu/world-map escape from tutorial

## URGENT — 60-min cap, EV=highest of the 3 fresh hypotheses
The K=6 planner audit at 11:37Z identified your hypothesis as **the cheapest and most strategically valuable** path forward. Two prior closures (XTest + vncdo) are CONTAMINATED (clicked Veldra's feet, not the NPC). Your job: bypass the NPC entirely via menu/world-map UI.

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-skip-menu`** (branch `tutorial-skip-menu`).

## Current goal
- **goal_key**: `albion_tutorial_complete_local`
- **success_fact_key**: `albion_tutorial_skip_via_menu` (NEW)
- **success metric**: `zone.name` in /state transitions from "The Lighthouse" / `@TUTORIALSINGLE@<uuid>` to a NON-tutorial zone (e.g., a starter city like Thetford, OR any open-world zone). Quest panel "First Steps - Talk to the survivor (0/1)" disappears or changes.

## Hypothesis
Albion's Options/Settings menu (Esc → Settings → Gameplay) OR world-map context-menu (M / right-click on minimap) exposes a **Skip Tutorial** / **Leave Tutorial Zone** / **Travel** affordance. Menus are designed to accept synthesized input (no anti-cheat reason to filter them). XTest IS proven for menus (per `[[albion-login-substrate]]` — login buttons + general keys work on Xtigervnc :3).

## Falsification (mechanism-scoped per [[falsify-mechanism-not-path]])
- **Mechanism class**: menu-driven UI traversal via xdotool key presses (Esc, arrows, Enter, Tab) and left-clicks on menu items.
- **Falsified iff**: Esc menu enumerated to depth-3 + minimap right-click context enumerated, NO "skip" / "leave" / "travel" / "exit tutorial" affordance found. Document with screenshot of every menu page reached.
- **Untried siblings if falsified**: hotkey "M" for map → world map → click destination, F1-F12 keyboard shortcuts (game help keybindings), chat-command `/leave` or `/travel` typed into chat (need chat-input).

## Substrate state (verified by planner)
- Veldra1203 in The Lighthouse, world ~(3.7, -31), wandering due to action-loop
- Quest panel still 0/1 "Talk to the survivor"
- Action-loop running — STOP it before menu exploration (Esc + menu clicks need stationary substrate)

## Tasks (60-min budget)

### T0 (~3min) — Stop action-loop + baseline screenshot
1. `XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus systemctl --user stop albion-action-loop`
2. Fresh screenshot: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-skip-menu/analysis/t0_pre_menu.png'`
3. Touch `/tmp/heartbeat_albion-tutorial-skip-menu`

### T1 (~25min) — Esc / Settings menu traversal
1. Press Esc: `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool key Escape'`. Screenshot `t1_esc.png`.
2. Inspect menu. Look for items: "Settings", "Options", "Help", "Tutorial", "Skip", "Travel", "Map", "Exit", "Logout".
3. For each promising menu item, mousemove + click (real-press) to navigate INTO. Screenshot after each click as `t1_menu_<n>.png`. Depth ≤3.
4. If you find any "Skip Tutorial" / "Leave" / "Travel to..." → click it. Check /state.zone.name change.

### T2 (~20min) — Map / minimap context-menu
1. Press M (map open): `xdotool key m`. Screenshot `t2_map_open.png`.
2. If map opens, look for travel destinations. Right-click on a destination if visible. Screenshot post-action.
3. Alternative: right-click on the minimap area (bottom-right corner, native coords ~(1700, 900) per UI conventions). Look for context menu with travel option.

### T3 (~10min) — Verdict + restart action-loop
1. **If zone changed**: 
   - `harness fact-set albion_tutorial_skip_via_menu "menu-driven escape via <Esc/Settings/Travel/etc> at depth N; zone transitioned from @TUTORIALSINGLE@... to <new>; artifact analysis/tutorial_skip_verdict_2026-05-25.md"`
   - Commit `analysis/tutorial_skip_verdict_2026-05-25.md` with menu navigation path + pre/post zone names.
2. **If no skip found**: 
   - Commit `analysis/tutorial_skip_menu_blocked_2026-05-25.md` with: every menu page screenshot, full enumeration of menu items found, conclusion that no skip affordance exists at depth 3.
3. **Always**: restart action-loop: `XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus systemctl --user start albion-action-loop`.

## Commit-or-falsify contract
- 60min hard cap. 55min mark: write whatever verdict/blocked you have.
- 10min heartbeat: `touch /tmp/heartbeat_albion-tutorial-skip-menu` after each menu step.
- `/tmp/abort_albion-tutorial-skip-menu` → commit partial + exit.

## Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- You MAY stop albion-action-loop.service during work; MUST restart on exit.
- xdotool key Escape is FINE for menus (it's NOT a quit-dialog trigger when used to navigate; only matters if you press it repeatedly into the dialog).
- If a "Quit/Exit Game" dialog appears, press Esc to dismiss (the SAME key) or click Cancel; do NOT click Quit.
- NEVER click "Logout", "Quit", "Exit Game", "Yes" on logout confirmations.
- xdotool via `sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- Screenshots: ALWAYS absolute paths in sudo-runuser.
- Per `[[albion-login-substrate]]`: Xtigervnc :3 accepts XTEST for buttons + general keys — this is the substrate XTest IS known to work on. The XTest failure with NPC sprite-click was likely due to stale coords, NOT XTest filtering.

## Memory references
- `[[falsify-mechanism-not-path]]`, `[[albion-login-substrate]]`, `[[albion-tutorial-unskippable-by-design]]` (RETRACT this if you find skip — it was a 2026-05-20 conclusion from before the bonus tutorial step 2 advance was observed)

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-skip-menu.md`
- Worktree: `/home/sdancer/albion-tutorial-skip-menu/` (branch `tutorial-skip-menu`)
- Planner output (READ first): see episode 11:37 + `last_planner_cycle_2026-05-25-11` fact
- /state: `http://127.0.0.1:8765/state`
- Harness: `/home/sdancer/orchestrator/harness`
- Success fact: `albion_tutorial_skip_via_menu`
