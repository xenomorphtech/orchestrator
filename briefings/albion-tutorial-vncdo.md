# albion-tutorial-vncdo — RFB pointer click on survivor NPC (mechanism #1 from adversarial)

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-tutorial-vncdo`** (branch `tutorial-vncdo`).

## Single-mechanism mandate (15min hard cap)
The XTest input-injection mechanism class was exhausted (5 probes failed). Adversarial-pair enumeration ranked **vncdo RFB pointer click as EV=9 top-1**: it changes event provenance from XTest-flagged to raw RFB-server-emitted, the cleanest difference.

Your ONLY job this turn: **execute vncdo RFB click on the survivor NPC and verify quest tick.** 1 mechanism, ~5 attempts max, then commit verdict.

## Substrate truth (verified 11:22 by orchestrator)
- Veldra1203 in The Lighthouse, world ~(10.5, -26.75); action-loop is producing autonomous Δz drift
- NPC was at native ~(432, 730-790) per t1_npc_location.txt prior to action-loop restart
- Action-loop IS active now and may have shifted Veldra's position relative to NPC — re-screenshot to confirm

## The probe (1 command, then verify)

### T0 (~2min) — Stop action-loop + fresh pre-screenshot
```
XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus systemctl --user stop albion-action-loop
sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-vncdo/analysis/t4_pre.png'
```
Inspect t4_pre.png — confirm NPC visible. If NPC drifted out, do ONE right-click at (220,250) via xdotool to re-reveal then re-screenshot (only ONE — don't recurse).

### T1 (~5min) — vncdo RFB click + verify
```
# Move + click in one vncdo invocation (per adversarial recipe)
vncdo -s localhost:3 move 432 750 click 1
sleep 2
sudo runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-tutorial-vncdo/analysis/t4_m1_post.png'
```
Compare t4_pre.png vs t4_m1_post.png for: (a) dialog overlay, (b) quest panel text change ("Talk to the survivor (0/1)" → other), (c) any visual confirmation.

If success: advance dialog with additional vncdo clicks OR `vncdo -s localhost:3 key Return` until quest text changes.

If no change after first click: retry at y=730, 760, 790 in turn (bracket the NPC sprite, same as turn-3 did with xdotool). Stop after 4 total vncdo clicks if none ticks.

### T2 (~3min) — Verdict + fact + restart action-loop
**If success**: 
```
/home/sdancer/orchestrator/harness fact-set albion_tutorial_step_talk_to_survivor_advanced "vncdo RFB click at (X,Y) :3 advanced quest from 'Talk to the survivor 0/1' to '<new text>'; artifact analysis/tutorial_advance_vncdo_verdict_2026-05-25.md; revival of tutorial-advance (XTest mechanism-dropped) via RFB provenance change"
```
+ commit `analysis/tutorial_advance_vncdo_verdict_2026-05-25.md` with pre/post sha256, coord, quest text diff.

**If failure (all 4 vncdo clicks no quest tick)**: commit `analysis/tutorial_advance_vncdo_blocked_2026-05-25.md` mechanism-scoped: "RFB pointer click via vncdo at NPC sprite (432, [730,750,760,790]) — no quest tick. Untried siblings: uinput keyboard (mechanism #2 from adversarial), AT-SPI Action.DoAction, Frida IL2CPP hook on OnPointerClick, LD_PRELOAD libX11 shim, photon-packet inject."

**Always**: restart action-loop on exit:
```
XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus systemctl --user start albion-action-loop
```

## Commit-or-falsify contract
- 15min hard cap. 12-min mark: write whatever verdict/blocked you have.
- Touch `/tmp/heartbeat_albion-tutorial-vncdo` at start AND after each vncdo attempt.
- `/tmp/abort_albion-tutorial-vncdo` → commit partial + exit.

## Constraints (HARD)
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- You MAY stop albion-action-loop.service during the click phase; MUST restart on exit.
- `vncdo` binary is at the standard location (verified present by adversarial worker). If it errors "connection refused", check `nc -z localhost 5903` — VNC port should be live.
- NEVER spam Escape.
- Screenshots: ALWAYS absolute paths inside sudo-runuser.
- **Do NOT redo XTest variants** (xdotool LEFT, xdotool key e/f/space). All falsified per partial.md.

## Memory references
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure
- `[[unity-real-press-required]]` — historical note; RFB has different semantics
- `[[albion-tutorial-clickclass]]` — LEFT-click on NPC = quest tick mechanism (XTest path failed, RFB tries the same logical action via different provenance)

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-tutorial-vncdo.md`
- Worktree: `/home/sdancer/albion-tutorial-vncdo/`
- Adversarial enumeration (READ first): `/home/sdancer/albion-tutorial-adv-enum/analysis/tutorial-advance_adversarial_alternatives.md`
- Prior partial.md: `/home/sdancer/albion-tutorial-advance/analysis/tutorial_advance_partial_2026-05-25.md`
- Prior NPC coords: `/home/sdancer/albion-tutorial-advance/analysis/t1_npc_location.txt`
- Harness: `/home/sdancer/orchestrator/harness`
- Fact on success: `albion_tutorial_step_talk_to_survivor_advanced`
- VNC endpoint: `localhost:3` (TCP 5903)
