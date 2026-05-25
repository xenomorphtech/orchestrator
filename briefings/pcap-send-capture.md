# pcap-send-capture — restore send-side Photon capture WITHOUT crashing Albion

## Role & workdir
Codex worker at `/home/sdancer/vastai-albion-pcap-send` (worktree, branch `pcap-send-capture`, off `vastai-albion@14b7ace`).

## Goal
User directive via /talk channel `vastai-albion-web` at 2026-05-21T15:00:16+02:00:
> retry trying to get send packets until it works, also, enter world after login before considering it a success

We just **confirmed** ([[albion-send-hooks-break-client]]) that in-process `sendto`/`send`/`sendmsg`/`writev` LD_PRELOAD overrides cause Albion to wedge/crash after login. So we cannot use that mechanism. We need a send-side capture path that adds **zero** in-process latency on the Photon ACK path.

The chosen mechanism is **pcap on the Photon UDP port** — read packets straight off the wire, decode the Photon frame, emit JSONL records in the same format the existing ingester produces (with `is_recv=0` for send direction). Zero hook in the game process.

## Already achieved (do not re-do, do not re-falsify)
| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| 1 | photon_tap.c commit `ce50cb2` + deployed .so on vast.ai | send-family LD_PRELOAD overrides removed; recv-only build is stable | vast.ai user `albion` | ✅ DONE |
| 2 | Albion PID 1256835 alive 20+ min in 25-player zone | post-login wedge fixed when send hooks absent | vast.ai | ✅ DONE |
| 3 | Fact `albion_send_hooks_were_crash_cause_2026_05_21` set | hypothesis confirmed | harness facts | ✅ DONE |
| 4 | Memory `[[albion-send-hooks-break-client]]` written | reusable knowledge | orchestrator memory | ✅ DONE |

Your job is **NOT** to revisit the LD_PRELOAD path. Your job is to add a parallel pcap-based send capture pipeline that lives alongside the working recv-only LD_PRELOAD setup.

## Success criteria
**All four must hold.** Verification gate is explicit per user.

1. A new daemon (Rust or Python — your choice) is running on vast.ai capturing UDP traffic to Albion's Photon ports, decoding Photon frames, and appending records to the session JSONL stream that `gamestate_service.py` already consumes. The records MUST be format-compatible with the LD_PRELOAD ingester's existing output (look at `albion-frida-capture/ingest/` and the existing JSONL frames at `/var/log/albion-frida-sessions/session-*.jsonl` on vast.ai for the exact schema). Use `is_recv: 0` to mark send-direction frames.
2. **Albion is still alive AND in-zone for >5 minutes after the pcap daemon starts.** `pgrep -af Albion-Online` returns a PID with ETIME >5:00. `/state` shows >0 players and self.zone is non-None (because gamestate_service's self.zone derivation reads send-side `request:JoinZone` frames — which your daemon now feeds it). If self.zone is still None after 5 min in zone, the format match is broken — fix the schema, not the daemon.
3. `/state.events_processed` is advancing at a higher rate than the recv-only baseline (~5/sec when active), and the additional events include send-direction frames (check by counting JSONL lines tagged `is_recv:0` vs `is_recv:1`).
4. Restart Albion via the existing launch wrapper to confirm clean cold-start coexistence between the LD_PRELOAD recv shim and the pcap daemon. **No crash within 5 min of post-restart login.**

If your daemon causes Albion to crash post-login, you've recreated the old failure mode with a different mechanism. Falsify your approach and try path 2 from the orchestrator's 5-paths list (lock-free async LD_PRELOAD with deferred forward via eventfd).

## Mandatory anti-patterns
- **NEVER** put the daemon in Albion's process (LD_PRELOAD/inject/ptrace). Use a separate process. The whole point of pcap is that the game doesn't know it's watched.
- **NEVER** re-enable the LD_PRELOAD send hooks. Leave `-DDISABLE_SEND_HOOKS` in place on the photon_tap.so build.
- **DO NOT** touch `gamestate_service.py`'s parsing logic — it already eats the JSONL format. Just emit records that match what the LD_PRELOAD-based ingester would have written. Read `albion-frida-capture/ingest/main.rs` (or equivalent) and one live session JSONL line to learn the schema.
- **DO NOT** rebuild the entire ingester pipeline. The cleanest design: pcap daemon → emit JSONL lines straight into the same session log file the LD_PRELOAD ingester writes to, OR feed the existing ingester via its Unix socket (look for `/var/run/albion-ingest.sock` or similar in `start_all.sh`).
- **DO NOT** kill the LD_PRELOAD .so. It's the recv-side capture — still needed.
- **DO NOT** declare success based on PID liveness alone. The user explicitly said "enter world after login before considering it a success." That means /state.self.zone must be non-None AND game must be playable.

## Next concrete tasks
1. **Substrate prep on vast.ai.** SSH in (`ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`), check that `tcpdump`/`libpcap` are installed (`which tcpdump; ldconfig -p | grep libpcap`). If not, `apt-get install -y tcpdump libpcap-dev`. Confirm Albion is alive and find the Photon UDP ports: `ss -tunlp | grep Albion-Online` AND `ss -tunp | grep Albion-Online` (look for ESTAB UDP rows; Albion typically uses ports in the 5055-5057 range, possibly higher game-server ports — record the actual peer ports).
2. **Pick the schema.** scp one live JSONL frame from `/var/log/albion-frida-sessions/session-*.jsonl` on vast.ai and inspect the fields. The LD_PRELOAD ingester writes records like `{"ts": <epoch_us>, "is_recv": 1, "fd": <int>, "peer_port": <u16>, "payload_b64": "..."}` (verify exact keys). Match this schema for your send-direction records with `is_recv: 0`.
3. **Build the daemon.** Rust preferred (workspace already has `photon-decoder-rs` and `albion-session-tail` references). Write a small binary that uses the `pcap` crate to live-capture, filters `udp and (src port 5055 or src port 5056 or src port matching server ports)` for outbound (src = albion machine), and appends matching JSONL lines to the active session log. Keep it bounded — no unbounded memory growth; cap in-flight queue at 4096 frames; drop with a warning log if over.
4. **Deploy as a systemd unit (or systemd-equivalent on vast.ai container — see `[[worker-artifact-isolation]]`, vast.ai has no systemd so use a tmux session or nohup setsid with a PID file at `/var/run/photon-pcap.pid`).** Start it. Confirm it's running and writing.
5. **Verify gate #2 (the user's explicit gate).** Wait until `/state.self.zone` is non-None AND `/state.players` count is >0 AND Albion PID has ETIME >5:00 after pcap daemon start. Take a screenshot of https://albion.orch.run/ showing self.zone populated. Save to `analysis/pcap_send_verify_screenshot.png`.
6. **Restart-test.** `pkill -f Albion-Online`, wait for it to die, relaunch via `/opt/albion-frida-capture/spawn_preload.sh`, watch pcap daemon stay alive AND new Albion process come up clean AND no crash for 5+ min.
7. **Report.** Append a milestone line to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` with: pcap daemon PID, JSONL output path, self.zone observed value, Albion uptime at success moment, screenshot path. Frame as a single line of JSON.

## Schema reference: existing JSONL ingester
Read `albion-frida-capture/ingest/` source in the worktree to learn:
- exact field names (`ts`, `is_recv`, `fd`, `payload_b64` likely)
- timestamp units (epoch micros? nanos? ISO?)
- payload encoding (base64 of the raw UDP datagram? or photon body only?)
- any framing (length-prefixed lines? one frame per line?)

If the ingester takes input via Unix socket (look in `start_all.sh` and the spawn wrapper), the cleanest design is to make your pcap daemon connect to that socket and push frames there — then the ingester does the JSONL serialization for you with guaranteed format match.

## Substrate facts
- vast.ai SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- vast.ai is a Docker container — **no systemd**. Use nohup setsid + PID file for daemonization.
- Session JSONL: `/var/log/albion-frida-sessions/session-*.jsonl` on vast.ai
- LD_PRELOAD .so (don't touch): `/opt/albion-frida-capture/preload/photon_tap.so`
- Spawn wrapper: `/opt/albion-frida-capture/spawn_preload.sh`
- Live dashboard: https://albion.orch.run/ → :8765 on vast.ai
- Current state: Albion PID 1256835, in-zone with 25 players, send hooks disabled

## Reporting cadence
Append milestone events (not heartbeats) to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`. Report-worthy:
- pcap daemon up and writing
- self.zone first appears in /state (means the schema match worked)
- restart-test pass
- failure (with falsification: what specifically broke)

## Memory pointers
- `[[albion-send-hooks-break-client]]` — why in-process hooks are off the table
- `[[albion-client-wedge-class]]` — the wedge class this avoids
- `[[macromanage-workers]]` — pick your own tactics
- `[[worker-artifact-isolation]]` — vast.ai has no systemd; daemons survive turn-end via nohup setsid
