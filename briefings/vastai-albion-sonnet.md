# vastai-albion-sonnet — Frida capture pipeline working + verifiable

## Role & workdir
Claude pane worker on Opus 4.7 xhigh, workdir `/home/sdancer/vastai-albion`. Drive the Frida-based Albion Online capture pipeline to a working+verifiable end state on the vast.ai box.

## Goal
**Frida pipeline working end-to-end AND verifiable.** The current dashboard at https://albion.orch.run/ is fed by the legacy tcpdump pcap stream into `gamestate_service.py`. Replace that data source with Frida hook → session log → gamestate-service consumer.

## Verification artifact (the ONLY thing that means "done")
`curl -s http://localhost:8765/state` (or via the public tunnel) returns entity state that demonstrably came from the Frida ingester, identifiable via one of:
- a `/state.source` field set to `"frida"` (or similar discriminator)
- an event-count delta distinguishable from the tcpdump path
- a debug timestamp/marker injected by the hook process and surfaced in /state

Until that curl output shows Frida-sourced data, the goal is open.

## Plan-of-record
`/home/sdancer/vastai-albion/albion-frida-capture/docs/DESIGN.md` (committed eeb2dea, 383 lines). All architectural decisions live there — refer to it, update it if you learn something that contradicts it.

## Operating doctrine
- **Stay macro.** You own all tactics: frida install, hook attach, ingester transport, session-log format, gamestate-service consumer swap, deployment topology. Don't ask the orchestrator for opcode lookups, repo greps, or design decisions you can resolve by reading code or testing.
- **Report only on**: (a) verification success with the curl output proving Frida-sourced data, or (b) a blocker that genuinely needs orchestrator-level resources (new hardware, credential, third-party API key, physical-device intervention).
- **No tcpdump regression.** The existing pcap pipeline keeps running until the Frida pipeline is verified. Cutover happens in one atomic switch to the consumer's `--source` selector.

## Anti-patterns (don't)
- Don't open a status-table message saying "X not installed, Y not attached, Z doesn't exist" and then ask "want me to do steps 1-3?" — just do them.
- Don't draft more design docs unless DESIGN.md is genuinely missing something concrete. Execution beats more architecture.
- Don't burn turns drafting "next steps" lists for the orchestrator to ack. The directive is in this file.

## Substrate facts
- vast.ai SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Albion process: PID rotates on cold-restart; locate via `pgrep -f Albion-Online` on the box.
- Existing capture: tcpdump → `/var/log/albion-capture/live-*.pcap` (1h rotation, 6 files).
- Existing service: `/opt/albion-gamestate/gamestate_service.py` on port 8765, RSS-capped 1GB.
- Existing dashboard: `GET /` returns 11 KB vanilla-JS dashboard polling `/state` every 2s.
- vast.ai container has **no systemd** (PID 1 = sh). Use `nohup setsid` + PID files for daemons.

## Memory pointers
- `[[albion-client-wedge-class]]` — Photon "Client not responding" wedge after auth; cold-restart not backoff
- `[[albion-substrate]]` — vast.ai is the AO substrate (NOT thered/RK3588)
- `[[worker-artifact-isolation]]` — daemons on vast.ai are not subject to Hetzner cgroup death; nohup setsid is fine there
- `[[macromanage-workers]]` — the orchestrator's directive doctrine for this path

## Reporting cadence
Append milestone events (not heartbeats) to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` as `{"ts","from":"vastai-albion-sonnet","text":"<event>"}`.
