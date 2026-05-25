# albion-action-emitter-local — Ground-click→move probe (turn-7) on now-stable substrate

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-action-emitter-local`** (existing fork, codebase + audit machinery already in place).

## Current goal
- **goal_key**: `albion_action_loop`
- **success_fact_key**: `albion_action_emitter_shipped`
- **success metric (this phase)**: ≥1 audited dispatch that produces an observable character-position change between pre/post screenshots on acct_3 in `The Lighthouse`.

## Already achieved (do NOT re-falsify)

| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L1–L3 | acct_3 onboarded, charcreate, in-zone (`Veldra1203`) | account chain works | ✅ DONE |
| L4 | `analysis/p1_5_palette_rewrite_verdict_2026-05-25.md` + `t6_r_post.png` (sha256 `47e0b8f41bb8`) | **R-key dispatch produces visible activation ring + cooldown** (hotbar channel CLOSED) | ✅ DONE |
| L4-neg | same verdict — `click button=1 (LEFT) x=1240 y=690 duration=80ms` produced NO displacement | LEFT-click at this coord NOT-supported for move | ✅ DOCUMENTED |
| L5 | `albion-acct3-watchdog.service` ACTIVE (PID 256173, in_zone heartbeats dist 3–10, threshold 12) | **substrate now self-healing**: watchdog re-logs in within ~60s on drop | ✅ DONE 2026-05-25T05:53Z |
| L5-prev | turn-6 substrate-blocked (`t7_blocker_login_modal.png` sha256 `62f01c0d3ce4`) | reason no longer in force (substrate guaranteed in_zone by watchdog) | ✅ UNBLOCKED |

**Hotbar (key dispatch) channel is PROVEN.** What's open: ground-click→character-position-change.

## Hypothesis to test this turn
Albion's standard movement convention is **right-click on terrain → walk-to-point**. A `mousedown 3 → 150ms → mouseup 3` (Unity real-press per `[[unity-real-press-required]]`) at a clean ground-tile coord, dispatched against the active Albion window on `DISPLAY=:3`, will move `Veldra1203` such that comparing `pre.png` and `post5s.png` shows a non-trivial character-sprite displacement.

## Falsification (mechanism-scoped — read [[falsify-mechanism-not-path]] before any *_blocked.md)
- **Mechanism under test**: `xdotool mousedown 3 / 150ms / mouseup 3` (right-click button real-press, input-injection class) via `runuser -l sdancer -c 'DISPLAY=:3 xdotool ...'`.
- **Falsified iff**: probed at ≥3 distinct clean ground tiles, neither button 3 (right-click) nor button 1 (left-click) at any of them produces visible displacement, AND the dispatch audit logs `dispatch_result=ok` for each.
- **Untried siblings (must enumerate before closing)**: `vncdotool pointer mousedown/mouseup` (PROVEN for click-clicks elsewhere), evdev `/dev/uinput` mouse events, AT-SPI pointer events, XTest cookie protocol direct. Name ≥3 untried in any *_blocked.md.

## Substrate facts (PRE-VERIFIED — do NOT re-test)

| Fact | Source |
|---|---|
| acct_3 Albion alive on Xtigervnc :3, `Veldra1203` in `The Lighthouse` | watchdog heartbeats, every 60s |
| Substrate self-heals on login drop within ~60s | `/home/sdancer/albion-acct3-watchdog/var/log/relogin.jsonl` |
| Tutorial quest panel ("The First Step — Click on the ground to move and find a way off the beach") visible top-right | turn-6+ screenshots |
| Hotbar y > ~580; quest panel x > ~440; minimap bottom-right; HP top-left | turn-6 inspection |
| emit.py / policy.py / audit JSONL pipeline already exists in this worktree | prior turns |

## Next 2 concrete tasks (~60min total)

### Task 1 (~25min) — Right-click ground-move probe at ≥3 tiles

1. Capture `analysis/t7_pre.png` (`DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root`; fall back to `xwd -id $(xdotool getactivewindow)` if root returns blank).
2. From that screenshot, pick **3 distinct ground tiles** that satisfy: not under hotbar (y < 580), not under quest panel (x < 440), not under minimap, not under HP bar, not on/near Veldra's sprite. Record `(x,y)` in `analysis/t7_target_tiles.txt`.
3. For each tile, in sequence: pre.png → dispatch via `runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool mousemove --window <wid> X Y mousedown 3; sleep 0.15; xdotool mouseup 3'` (`--window` to keep focus on Albion) → wait 5s → post.png. Audit row per emit.py format. Artifacts `t7_tileN_{pre,post}.png` for N=1,2,3.
4. Per tile: `t7_tileN_diff.png` via `compare -metric AE` or Pillow.

### Task 2 (~20min) — Commit verdict

Write `analysis/p1_5_groundclick_move_verdict_2026-05-25.md`:
- per-tile: target xy, audit row, pre/post sha256 prefix, AE diff count, qualitative observation (sprite moved? camera panned? quest tick? nothing?)
- conclusion: one of
  - "right-click ground-move SUPPORTED — tile N produced displacement from (xs,ys) to ~(xs',ys')"
  - "right-click NOT-SUPPORTED at 3/3 tiles — repeat Task 1 with button=1 at same tiles for parity, then continue to Task 3"
- IF supported: `harness fact-set albion_action_emitter_shipped "Local closed-loop ground-click→move verified via xdotool right-click real-press at tile (X,Y); artifact analysis/p1_5_groundclick_move_verdict_2026-05-25.md"`.

### Task 3 (~15min) — ONLY if 3/3 right + 3/3 left both produce no movement

Do NOT close the path. Per `[[falsify-mechanism-not-path]]`, this falsifies the MECHANISM, not the path. Write `analysis/p1_5_groundclick_move_mechanism_blocked.md` enumerating ≥3 untried siblings (vncdotool, evdev `/dev/uinput`, AT-SPI). Do NOT set the success fact — orchestrator picks next mechanism.

## Commit-or-falsify contract (per [[briefing-commit-or-falsify-contract]])
- 60min hard cap. Else write `analysis/p1_5_groundclick_partial_2026-05-25.md`.
- 15min heartbeat → `analysis/heartbeat.log`.
- `/tmp/abort_albion-action-emitter-local` → commit partial + exit.

## Constraints (HARD)
- **NEVER restart acct_3 Albion.** Watchdog handles drops.
- **NEVER touch `albion-acct3-watchdog.service` / `/home/sdancer/albion-acct3-watchdog/`** — substrate keepalive.
- **NEVER touch `/home/sdancer/albion-action-emitter/`** — vast.ai reference, dormant.
- **NEVER log credentials.** If login screen appears mid-turn, STOP and wait ≤120s for watchdog re-log.
- **NEVER spam Escape** (`[[xdotool_unity_albion_blocked]]` retraction).
- **NEVER relaunch acct_1 / acct_2** — P2, deferred.
- xdotool via `runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`.
- Screenshot: `DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root <file>.png`.

## Memory references
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure, ≥3 untried siblings.
- `[[unity-real-press-required]]` — mousedown / 150ms / mouseup (synth click discarded).
- `[[albion-login-substrate]]` — Xtigervnc :3 accepts XTEST.
- `[[xdotool_unity_albion_blocked]]` — RETRACTED; xdotool works.
- `[[albion-tutorial-clickclass]]` — tutorial QUEST-tick events need LEFT; plain MOVE on RIGHT. Test right first.
- `[[macromanage-workers]]` — self-discover coords.

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-action-emitter-local.md`
- Prior turn-5 verdict (rotation ✅, LEFT-click NO): `analysis/p1_5_palette_rewrite_verdict_2026-05-25.md`
- Prior turn-6 blocker (substrate, now resolved): `analysis/p1_5_rightclick_move_substrate_blocked_2026-05-25.md`
- Audit JSONL examples: `analysis/audit/`
- Watchdog log (read-only): `/home/sdancer/albion-acct3-watchdog/var/log/{relogin,heartbeat}.jsonl`
- Harness: `/home/sdancer/orchestrator/harness`
