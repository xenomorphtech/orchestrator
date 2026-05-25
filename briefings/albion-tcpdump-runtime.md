# albion-tcpdump-runtime — passive network sniff during Albion startup

## Role & workdir
Fresh Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-tcpdump-runtime`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **capture EVERY outbound packet from Albion-Online during a controlled 60-90s startup window via tcpdump** — bypassing the anti-debug detection that blocked the cycle-3294 strace path. Target: identify (a) the runtime URL Albion opens for accountportal mode (cycle-3281 hypothesis), (b) endpoints beyond the known `loginserver.live.albion.zone:5055`, (c) any HTTPS/HTTP outbound to account/oauth endpoints.

## Big-picture rationale
Cycle-3295 finding: Albion's `BEClient_x64.so` reads `/proc/self/status TracerPid` and zenity-aborts when ptrace is attached. ALL ptrace-based observation tools (strace/gdb/ltrace/frida-attach) are structurally blocked. **tcpdump is purely passive** — it sniffs packets at the kernel/interface layer WITHOUT touching the target process. No ptrace, no LD_PRELOAD, no /proc/self/status footprint. Albion cannot detect tcpdump.

This is the only viable runtime-observation class remaining for this campaign after 7 falsified attack classes.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `/home/sdancer/albion-strace-startup/analysis/strace_verdict.md` (c3295) | strace path blocked by Albion anti-debug — DO NOT retry strace | ✅ DONE |
| 2 | Memory `feedback_albion_antidebug_ptrace.md` | Mechanism: `/proc/self/status TracerPid` check in BEClient_x64.so | ✅ DONE |
| 3 | `/home/sdancer/albion-accountportal-cdp/analysis/accountportal_flow_verdict.md` (c3281) | accountportal mode likely needs a runtime-generated URL that the client opens; NOT visible in static binary RE alone | ✅ DONE |
| 4 | `/home/sdancer/albion-photon-login-fuzz/analysis/auth_request_schema.md` (c3286) | Photon wire framing captured from existing frida-tap logs; Photon login server is `loginserver.live.albion.zone:5055` UDP | ✅ DONE |
| 5 | `albion-token-watcher.service` armed on LOCAL orchestrator host | Captures any future zone-change autonomously | ✅ DONE |

## Success criteria
1. **Capture proof**: `albion_startup.pcap` produced from a controlled tcpdump on the container during Albion-Online's first 60-90s post-restart. ≥100 packets captured.
2. **Endpoint inventory**: every unique (dest_ip, dest_port) tuple Albion contacts. Cross-reference with known `loginserver.live.albion.zone:5055`. **NEW endpoints are findings**.
3. **HTTPS URL inventory**: any TLS SNI hostname Albion connects to. SNI is visible plaintext in TLS ClientHello — extract via `tshark -Y 'tls.handshake.type==1' -T fields -e tls.handshake.extensions_server_name`. This will reveal accountportal/oauth/account.albiononline URLs if they exist.
4. **Packet-level decode**: for the Photon UDP stream (port 5055), extract command sequences to confirm whether the encryption stage we captured in cycle-3286 matches what's wire-side. This cross-validates the photon-fuzz path's schema.
5. **Verdict**: `analysis/tcpdump_verdict.md` with Achievement-levels-+-gaps. Level 5 = a SPECIFIC runtime URL or endpoint identified that we can hit directly.

## Tasks (sequential, ~2h hard cap)

### Task 1 — sense substrate + verify tcpdump available
1. SSH to container. Confirm `tcpdump --version` works (Ubuntu 22.04 has it pre-installed in most images).
2. Confirm Albion-Online running: `pidof Albion-Online`.
3. Confirm token-watcher active on LOCAL orchestrator host: `systemctl is-active albion-token-watcher.service` on THIS host (NOT the vast.ai container). Should return `active`. If not, escalate.
4. Find the network interface Albion uses for egress. On vast.ai it's likely `eth0` (or `docker0` if container is bridged). Run `ip route show default | head -2` and `ip -o link show | head -10` to identify.
5. Capture baseline `/state` zone=null + ev counter.

### Task 2 — controlled restart with tcpdump capturing
1. Start tcpdump in background BEFORE the restart:
   ```bash
   tcpdump -i <iface> -w /tmp/albion_startup.pcap -s 0 -U \
     'host 127.0.0.1 or host loginserver.live.albion.zone or host albiononline.com or host assets.albiononline.com or net 5.42.0.0/16 or (port 53 or port 80 or port 443 or port 5055 or port 5056)' &
   TCPDUMP_PID=$!
   ```
   (Adjust filter — Photon UDP is on 5055 typically; Albion may also have other endpoints. Better to broaden to catch unknowns: `not arp and not (port 22 or port 3000 or port 3021 or port 6379)` to exclude SSH+harness ports.)
2. Stop only `albion-client` tmux: `tmux kill-session -t albion-client`.
3. Wait for Albion-Online to terminate: `while pidof Albion-Online; do sleep 1; done`.
4. Relaunch Albion via the SAME wrapper chain (normal `/usr/local/bin/run-albion-client`, no strace).
5. Wait 90 seconds.
6. Stop tcpdump: `kill $TCPDUMP_PID` (SIGTERM).
7. Compress + scp back to `/home/sdancer/albion-tcpdump-runtime/analysis/albion_startup.pcap.gz`.

### Task 3 — mine the pcap
1. `gunzip -k albion_startup.pcap.gz`. Total packet count?
2. **Endpoint inventory**:
   ```bash
   tshark -r albion_startup.pcap -T fields -e ip.dst -e tcp.dstport -e udp.dstport \
     | sort -u | head -50
   ```
   → `analysis/runtime_endpoints.txt`.
3. **TLS SNI inventory** (most likely to reveal accountportal URL):
   ```bash
   tshark -r albion_startup.pcap -Y 'tls.handshake.type==1' -T fields -e tls.handshake.extensions_server_name \
     | sort -u
   ```
   → `analysis/runtime_tls_sni.txt`.
4. **HTTP Host headers** (for any plaintext HTTP):
   ```bash
   tshark -r albion_startup.pcap -Y 'http.request' -T fields -e http.host -e http.request.uri \
     | sort -u
   ```
   → `analysis/runtime_http_hosts.txt`.
5. **DNS queries** (also reveals candidate URLs):
   ```bash
   tshark -r albion_startup.pcap -Y 'dns.flags.response==0' -T fields -e dns.qry.name \
     | sort -u
   ```
   → `analysis/runtime_dns_queries.txt`.
6. **Photon UDP stream** for cross-validation with photon-fuzz schema:
   ```bash
   tshark -r albion_startup.pcap -Y 'udp.port==5055' -T fields -e frame.time_relative -e ip.dst -e udp.dstport -e data.data \
     | head -50
   ```
   → `analysis/photon_udp_stream.txt`.

### Task 4 — verdict
Write `analysis/tcpdump_verdict.md` with Achievement-levels-+-gaps:
- Level 1: pcap captured (packet count, file size)
- Level 2: endpoint inventory shows N unique destinations
- Level 3: TLS SNI / HTTP Host inventory reveals (or doesn't) new account/oauth/portal URLs
- Level 4: DNS queries cross-reference confirms (or doesn't) accountportal-class hostnames
- Level 5: A SPECIFIC runtime URL/endpoint is identified that:
  - matches the cycle-3281 "runtime-generated parameterized URL" hypothesis (e.g., `https://account.albiononline.com/...?state=...&device_id=...`), OR
  - is a previously-unknown endpoint we can probe to find auth/exchange-code/oauth

If Level 5 hits → fact `albion_runtime_url_discovered_2026_05_22=<url>` + propose follow-on briefing for hitting that URL directly. If Level 4-only with no novel endpoints (only known login server + assets.albiononline.com) → declare path closed cleanly.

## Constraints & gotchas
- **NO Frida, NO ptrace, NO strace, NO LD_PRELOAD-touching-Albion** — purely passive packet sniff. tcpdump uses AF_PACKET socket, NOT ptrace.
- **photon_tap.so MUST stay `-DDISABLE_SEND_HOOKS`** per `[[albion-send-hooks-break-client]]` — but you're NOT modifying photon_tap. Don't touch the LD_PRELOAD chain.
- **Restart Albion ONCE** (controlled). Restore wrapper IS NOT MODIFIED in this task — you just launch tcpdump AROUND the normal launch sequence.
- **Other 4 production daemons must stay alive**: cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest. Only `albion-client` tmux gets restarted.
- **Token-watcher must remain armed**: verify LOCAL orchestrator host's `albion-token-watcher.service` is active throughout. NOT on the vast.ai container (per cycle-3290 strace correction).
- **Memory budget**: pcap of 90s is probably 50-500MB. Process via streaming tshark commands (`tshark | head`), NEVER load entire pcap into Python set. Compress before scp-back.
- **Filter carefully**: a too-narrow filter might miss novel endpoints. A too-broad filter might capture SSH/harness traffic (yours). Use BPF `not (port 22 or port 3000 or port 3021 or port 6379)` to exclude orchestrator infrastructure.
- **One worker per path**: photon-fuzz worker is on parallel path (Photon protocol encryption-setup RE) — do NOT touch `/home/sdancer/albion-photon-login-fuzz` worktree.

## Relevant files / references
- Live container: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Strace verdict (DO NOT REPRO): `/home/sdancer/albion-strace-startup/analysis/strace_verdict.md`
- Anti-debug memory: `feedback_albion_antidebug_ptrace.md`
- Photon schema (cross-reference): `/home/sdancer/albion-photon-login-fuzz/analysis/auth_request_schema.md`
- Wrapper chain (read, don't modify): `/usr/local/bin/run-albion-client` → `/opt/albion-frida-capture/spawn_preload.sh` → `./Albion-Online`
- Sister-path watcher (DO NOT TOUCH): LOCAL host `albion-token-watcher.service`
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Memory pointers: `[[albion-antidebug-ptrace]]`, `[[albion-send-hooks-break-client]]`, `[[albion-vastai-daemon-stack]]`, `[[orchestrator-role]]`, `[[macromanage-workers]]`.

## Reporting
Concise progress at each task boundary. If a runtime URL is captured → fact + milestone in talk channel. If only known endpoints surface → declare path closed cleanly + suggest next-best. Achievement-levels-+-gaps throughout. Hard cap 2h.
