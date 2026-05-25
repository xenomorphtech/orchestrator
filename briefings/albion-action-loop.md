# albion-action-loop — Autonomous action loop on acct_3

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-action-loop`** (branch `action-loop` off `albion-action-emitter-local` `4597ad5`).

## Current goal
- **goal_key**: `albion_action_loop`
- **success_fact_key**: `albion_action_loop_autonomous_running`
- **success metric (this phase)**: a systemd USER unit that runs emit.py against `localhost:8765/state` for ≥5 continuous minutes and produces ≥3 distinct `Δ(x,z) > 1.0` movement events in `/state.last_move_ts` history. Substrate must remain in_zone after the run (watchdog still happy; no client wedge).

## Already achieved (do NOT re-falsify)

| Level | Artifact / fact | What it verifies | Status |
|---|---|---|---|
| L1 | `albion_action_emitter_shipped` | xdotool right-click ground-move dispatch produces Veldra1203 displacement | ✅ DONE |
| L2 | `albion_acct_3_watchdog_running` (detector_v2) | substrate self-healing, multi-crop in_zone classifier | ✅ DONE |
| L3 | `albion_gamestate_local_live` | HTTP /state at 127.0.0.1:8765 returns live `zone.name + self.{x,z}` under systemd | ✅ DONE |
| L4 | `emit.py --gamestate-url <url>` already exists | client polls /state + dispatches via xdotool | ✅ DONE (this worktree) |
| L5 | closed-loop Δ(x,z) = (2.00, 5.53) measured 07:33 | end-to-end chain proven | ✅ DONE |

You are NOT inventing a new pipeline. You are running the proven `emit.py --gamestate-url http://localhost:8765/state` continuously as a systemd USER unit and observing that the loop produces visible character movement over time.

## Hypothesis
The existing `emit.py` (in this worktree, also at `/home/sdancer/albion-action-emitter-local/emit.py`) — when pointed at the live `localhost:8765/state` and configured with a navigation-oriented `policy.py` decision branch — will autonomously produce Δ(x,z) movements on acct_3, observable via successive `/state.self` polls. systemd USER supervision (mirroring the gamestate-local pattern) keeps the loop alive across worker turns.

## Falsification (mechanism-scoped — read [[falsify-mechanism-not-path]] before any *_blocked.md)
- **Mechanism under test**: `emit.py --gamestate-url <real /state>` continuous loop with current `policy.py` (decision class).
- **Falsified iff**: after 5min runtime, `/state.last_move_ts` advances ≤2 times AND no Δ(x,z) > 1.0 is recorded — meaning either policy never decides to walk, or every walk command lands at an already-satisfied destination.
- **Untried siblings (must enumerate before path-close)**: random-walk policy with bounded ground-tile sampling, goal-directed policy toward zone-exit, oscillating-tile policy, time-driven heartbeat policy (walk every Ns regardless of state). List ≥3 in any *_blocked.md.

## Substrate facts (PRE-VERIFIED — do NOT re-test)
- acct_3 Albion alive on Xtigervnc :3, `Veldra1203` in `The Lighthouse`.
- /state populated: zone.name="The Lighthouse", self.name="Veldra1203", self.x, self.z, last_move_ts.
- emit.py supports `--gamestate-url` flag (line 285 of `emit.py`).
- 3 systemd USER units already active: `albion-acct3-watchdog.service`, `albion-gamestate-local-{capture,service}.service`.

## Task 1 — DONE (prior turn)
- `bin/run_loop.sh` written with runuser-tolerant launcher.
- `policy.py` + `config/policy.yaml` extended for in_zone random-tile dispatch on jittered interval.
- 65s live verification: `z: -75.16 → -70.57` (Δz=4.6); `/state.last_move_ts` advanced. Loop end-to-end PROVEN.
- ⚠️ systemd USER unit NOT yet installed — albion-action-loop.service inactive. Continue with Task 2.

## Next concrete task (~20min)

### Task 1B (vestigial — DONE) — policy + launcher
1. Read this worktree's `emit.py` + `policy.py`. Identify the `in_zone` branch's current movement decision. If it's just "do nothing when in_zone" (likely — that was for closed-loop probe), patch the policy so it issues a periodic right-click move action at a randomly-selected clean ground tile (avoid HUD overlays — see `[[albion-tutorial-clickclass]]`-aware tile selection). Keep it simple: every 20–30s pick a tile, dispatch.
2. Write `bin/run_loop.sh` that launches emit.py with the right env: `runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority python3 emit.py --gamestate-url http://localhost:8765/state ...'`. Verify locally for ≥60s; expect ≥1 audit row + ≥1 /state update.

### Task 2 (~20min) — systemd USER unit + 5min verification + verdict

1. Install `~/.config/systemd/user/albion-action-loop.service` (mirror watchdog/gamestate-local pattern; user bus at `/run/user/1001/bus`; Restart=always; After=albion-gamestate-local-service.service).
2. `systemctl --user enable --now albion-action-loop.service`. Verify active.
3. Sample `/state` every 30s for 5min. Record each (ts, x, z) into `analysis/action_loop_5min_poll_2026-05-25.jsonl`. Compute distinct Δ(x,z) > 1.0 events.
4. Commit `analysis/action_loop_verdict_2026-05-25.md` with: unit listing, 5min poll trace summary (min/max/distinct Δ counts), watchdog post-run heartbeat (substrate still in_zone), one screenshot pre/post.
5. Set fact: `harness fact-set albion_action_loop_autonomous_running "albion-action-loop.service ACTIVE under user bus; 5min run produced <N> Δ(x,z)>1.0 events; substrate in_zone post-run; artifact analysis/action_loop_verdict_2026-05-25.md"`.

## Commit-or-falsify contract
- 45min hard cap. Else `analysis/action_loop_partial_2026-05-25.md` with ≥3 untried policy sibling alternatives.
- 15min heartbeat → `analysis/heartbeat.log`.
- `/tmp/abort_albion-action-loop` → commit partial + exit.

## Constraints (HARD)
- **NEVER touch acct3-albion.service / acct3-xtigervnc.service** (substrate).
- **NEVER touch `albion-acct3-watchdog.service` or its files** (substrate keepalive).
- **NEVER touch `albion-gamestate-local-*` services or their `/opt/albion-gamestate-local/` files** (data source).
- **NEVER touch `/home/sdancer/albion-action-emitter-local/` working files** — your worktree is `/home/sdancer/albion-action-loop/` on branch `action-loop`. emit.py / policy.py in YOUR worktree are forks you can modify freely.
- **NEVER spam Escape** (`[[xdotool_unity_albion_blocked]]` retracted but Escape still toggles quit-dialog).
- **NEVER relaunch acct_1 / acct_2** — single-account scope.
- **NEVER bind a second daemon to 127.0.0.1:8765** — the gamestate service already owns it.
- Tile coords must avoid HUD: hotbar y > ~580, quest panel x > ~440, minimap bottom-right, HP top-left. Safe ground roughly (200–440, 100–560).
- If your policy starts producing Δ(x,z) = 0 for 3 consecutive dispatches at the same tile, that's a *satisfied-destination* signal — pick a tile further from current self.x/self.z.

## Memory references
- `[[falsify-mechanism-not-path]]` — closure mechanism-scoped, ≥3 untried siblings.
- `[[unity-real-press-required]]` — mousedown/sleep/mouseup, not synth click.
- `[[albion-tutorial-clickclass]]` — tutorial QUEST-tick gates need LEFT; plain MOVE on RIGHT.
- `[[macromanage-workers]]` — self-discover policy details.
- `[[worker-artifact-isolation]]` — daemons launched by a worker die at turn-end; use systemd USER units.

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-action-loop.md`
- Worktree: `/home/sdancer/albion-action-loop/` (branch `action-loop`)
- Reference emit.py + policy.py (in your worktree, freely modifiable): `./emit.py`, `./policy.py`
- /state: `http://127.0.0.1:8765/state`
- Prior closed-loop verdict (read for context): `/home/sdancer/albion-gamestate-local/analysis/gamestate_local_verdict_2026-05-25.md`
- Prior ground-move verdict: `/home/sdancer/albion-action-emitter-local/analysis/p1_5_groundclick_move_verdict_2026-05-25.md`
- Watchdog state (read-only for verification): `/home/sdancer/albion-acct3-watchdog/var/log/heartbeat.jsonl`
- Harness: `/home/sdancer/orchestrator/harness`
- Fact to set on success: `albion_action_loop_autonomous_running`
