# vastai-albion-synthetic — verify Frida-equiv pipeline with synthetic input

## Role & workdir
Codex worker (codex_app_server kind, no pane) on a parallel divergence path. Workdir: `/home/sdancer/vastai-albion-synthetic` (worktree branched off `eeb2dea` as `synthetic-validator`).

## Why this path exists
The sibling worker `vastai-albion-sonnet` (Claude pane, workdir `/home/sdancer/vastai-albion`) has spent 5+ orchestrator cycles trying to drive **live Albion-under-LD_PRELOAD** to authenticated Photon traffic. The hook proven firing (cycle 2667: 1000 recvfrom calls intercepted), but `matched=0` persists across cycles. Symptoms include Albion stuck at 24MB RSS in `do_poll` under the preload — strong signal that Albion's anti-cheat is silently neutralizing the preload (refusing to authenticate, or refusing to fetch resources).

This path attacks the **verification problem** from a different angle: **prove the pipeline works end-to-end with synthetic input**, regardless of whether Albion-under-AC ever cooperates.

## Goal
**Frida-equivalent capture pipeline working AND verifiable, demonstrated by an end-to-end test that does NOT depend on live Albion-under-preload reaching authenticated Photon traffic.**

## Verification artifact (the ONLY thing that means "done")
Either:
- (A) A synthetic Photon-port UDP emitter on the vast.ai box generates frames that traverse the full pipeline (preload hook → ingester socket → session log JSONL → gamestate-service consumer → `/state` HTTP) and `curl http://localhost:8765/state` (or the public tunnel) returns the synthesized entity state, OR
- (B) An end-to-end integration test in this worktree under `tests/` that exercises the same chain with a controlled fake source and asserts the pipeline emits expected frames at each stage.

The artifact MUST exercise: hook → ingester IPC → session log writer → consumer parse → `/state` surface. Skipping any layer is insufficient.

## Plan-of-record
`/home/sdancer/vastai-albion-synthetic/albion-frida-capture/docs/DESIGN.md` describes the full architecture (inherited from cycle 2655 design). The synthetic path adds **synthetic Photon-port emitter** as a first-class data source alongside the live-Albion path.

## Operating doctrine (macromanage)
- **You choose tactics.** Whether to use a UDP socket on 5055 + a recvfrom-emitting subprocess under preload, an in-process bypass that writes directly to the session-log JSONL file, an integration test under `tests/`, or any combination — your call. Pick the fastest path to a verification artifact.
- **Don't fix Albion-under-preload.** That's `vastai-albion-sonnet`'s lane. Your job is to verify the pipeline independent of Albion.
- **Do not touch `/home/sdancer/vastai-albion`** (the sibling worktree) or `/var/log/albion-frida-sessions/` files that are open by sibling processes. Use distinct session-log filenames if you write live frames (e.g. `synthetic-*.jsonl`).
- **Report only on**: (a) verification success with the curl output proving synthetic frames flow through, or (b) a blocker that genuinely needs orchestrator-level resources.

## Substrate facts
- vast.ai SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Preload .so deployed: `/opt/albion-frida-capture/preload/photon_tap.so` (already loaded by sibling — DON'T overwrite during their active session; use a clone under `/opt/albion-frida-synthetic/` if you need to alter behaviour)
- Ingester socket: `/run/albion-frida.sock` (sibling owns; bring up your own if needed under `/run/albion-frida-synthetic.sock`)
- gamestate-service: `/opt/albion-gamestate/gamestate_service.py` on port 8765 (sibling expects this; run a second instance on port 8766 if you need to test consumer changes)
- Existing session log dir: `/var/log/albion-frida-sessions/` (read-only for you unless creating distinct filenames)
- vast.ai container has **no systemd** (PID 1 = sh). Use `nohup setsid` + PID files for daemons.

## Anti-patterns (don't)
- Don't try to "help" `vastai-albion-sonnet` solve the Albion-under-preload problem. Different path, different worker.
- Don't open status tables asking "want me to do X?" — execute.
- Don't draft another DESIGN.md. Read the existing one, build on it.

## Memory pointers
- `[[macromanage-workers]]` — orchestrator doctrine for this path
- `[[worker-artifact-isolation]]` — daemons on vast.ai survive turn-end (no systemd cgroup death like Hetzner)
- `[[ld_preload_hook_proven_firing_2026_05_21]]` — fact: hook works, blocker is downstream

## Reporting cadence
Append milestone events to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` as `{"ts","from":"vastai-albion-synthetic","text":"<event>"}`.
