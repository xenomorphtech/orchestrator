# albion-strace-startup — runtime syscall fingerprint of Albion client startup

## Role & workdir
Fresh Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-strace-startup`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **observe Albion-Online process startup via strace to capture EVERY file opened, network endpoint contacted, and config knob read** — revealing seams that are invisible to pure static-binary RE. Specifically targeting: (a) the actual runtime URL Albion opens for `accountportal` mode (cycle-3281 finding hypothesized this), (b) any token/refresh state file paths the static audit missed, (c) any env-var consumption pattern that the wrapper audit missed.

## Big-picture rationale
Six autonomous-bypass classes falsified (input × 2, prior-session-mining, prefs-flip, launcher-CLI, UnityPlayer-RE, accountportal-web). The cycle-3281 accountportal verdict surfaced a specific hypothesis: *"the Albion client launches a runtime-generated parameterized browser URL"* — and the cycle-3286 photon-fuzz Task 1 verdict shows the auth wire path uses an encryption stage. Both suggest there's runtime state we haven't observed. **strace captures it directly.**

This is pure observation. NO Frida, NO ptrace-modify, NO instrumentation. Just `strace -f -e openat,read,connect,sendto,execve` on a fresh Albion launch. Output goes to a log file; we then mine it.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | Static RE complete on launcher + UnityPlayer + GameAssembly + boot.config + wrapper env (cycles 3263-3268) | No auth seam at the cheap static layer | ✅ DONE |
| 2 | `/home/sdancer/albion-accountportal-cdp/analysis/accountportal_flow_verdict.md` (c3281) | Bare `/launcher/` URL returns 404 JSON; runtime URL must come from client | ✅ DONE |
| 3 | `/home/sdancer/albion-photon-login-fuzz/analysis/auth_request_schema.md` (c3286) | Photon auth uses encryption-setup before credential exchange; auth request opaque in pcap | ✅ DONE |
| 4 | `albion-token-watcher.service` armed and polling | Will capture autonomously on any future zone-state flip | ✅ DONE |

## Success criteria
1. **Capture proof**: `strace_startup.log` produced from a controlled Albion restart, covering the first 60s of startup (login form → would-be 2FA stage). At least 100k lines, with `openat`, `connect`, `sendto` syscalls visible.
2. **URL discovery**: greps for `accountportal`, `oauth`, `account.albion`, `albiononline.com/` reveal the EXACT runtime URL Albion opens for accountportal mode (if any). Save matches to `analysis/runtime_urls.txt`.
3. **File enumeration**: every `openat` path under `/home/albion/`, `/tmp/`, or with `token`/`refresh`/`session`/`auth` in the name. Save to `analysis/runtime_file_accesses.txt`.
4. **Endpoint inventory**: every `connect()` target (IP+port), every DNS `sendto`. Save to `analysis/runtime_endpoints.txt`. Cross-reference with known Albion endpoints (`loginserver.live.albion.zone:5055`, `assets.albiononline.com`, etc.) — anything new is a finding.
5. **Verdict**: `analysis/strace_verdict.md` with Achievement-levels-+-gaps. If a runtime URL is identified that we can hit directly, that's a NEW PATH. If only known endpoints surface, the path closes cleanly with "no hidden seam at runtime layer either."

## Tasks (sequential, ~2h estimated)

### Task 1 — sense baseline state
1. SSH to container. Confirm Albion-Online running (`pidof Albion-Online`). Capture current `/state` zone=null baseline.
2. Verify token-watcher is `active`: `systemctl status albion-token-watcher.service`. If NOT active, STOP — escalate.

### Task 2 — controlled restart with strace attached
1. Stop only `albion-client` tmux session (per recipe): `tmux kill-session -t albion-client`.
2. Wait for the Albion-Online process to terminate (`while pidof Albion-Online; do sleep 1; done`).
3. Launch Albion via the SAME wrapper chain that's normally used, BUT prefix with strace:
   ```bash
   strace -f -e trace=openat,read,connect,sendto,execve -o /tmp/strace_startup.log /usr/local/bin/run-albion-client &
   ```
   (Or inject strace into the right wrapper layer — adjust as needed. The goal is to capture EVERY syscall the Albion-Online + UnityPlayer make during the first 60s.)
4. Wait 60s.
5. Compress and pull the log: `gzip /tmp/strace_startup.log` → scp back to `/home/sdancer/albion-strace-startup/analysis/strace_startup.log.gz`.
6. Restore original wrapper if you modified anything. Verify 5 other production daemons still healthy.

### Task 3 — mine the strace
1. `gunzip -k strace_startup.log.gz`. Total line count?
2. `grep -iE "accountportal|oauth|account\.albion|portal\.albion|albiononline\.com" strace_startup.log` → save to `analysis/runtime_urls.txt`.
3. `grep "openat" strace_startup.log | grep -iE "token|refresh|session|auth|secret|cookie" | head -100` → save to `analysis/runtime_file_accesses.txt`.
4. `grep "connect(" strace_startup.log | grep -oE "AF_INET[^)]+" | sort -u` → save to `analysis/runtime_endpoints.txt`.
5. `grep "execve" strace_startup.log` → any subprocess that Albion launches (e.g., a browser).
6. Look for these specific suspect patterns:
   - Any HTTPS URL string (search for `https://` substrings in read() / sendto() data — strace might dump payload bytes)
   - Any subprocess execve targeting a browser (firefox, chromium, xdg-open) — that's the accountportal-handoff if it exists
   - Any file opened with `O_CREAT|O_WRONLY` containing JSON or base64 token-shaped content
   - Any UNIX socket dispatches that look like inter-process auth handoff

### Task 4 — verdict
Write `analysis/strace_verdict.md` with Achievement-levels-+-gaps framing:
- Level 1: strace log captured (line count, file size)
- Level 2: URL grep returned candidate accountportal URLs
- Level 3: file-access grep revealed new candidate token file paths
- Level 4: endpoint grep revealed new candidate auth servers
- Level 5: a SPECIFIC actionable next-step path identified (e.g., "Albion opens https://account.albiononline.com/launcher?id=<runtime-state> — we can recreate that URL", OR "no new endpoint surfaces, hidden-runtime-seam class closed")

If Level 5 produces a fresh hypothesis with specific actionable info → fact `albion_strace_runtime_seam_2026_05_22=<url-or-path>` + propose follow-on briefing. If Level 4-only with no new endpoints → close path cleanly.

## Constraints & gotchas
- **NO Frida, NO ptrace-modify** — strace is observe-only (it uses ptrace internally but only for monitoring, no memory writes). This is well within constraints.
- **You MUST restart Albion ONCE** for this to work — strace can't attach to a running process and observe its early startup syscalls. The restart should be controlled (tmux kill-session albion-client, then re-launch with strace prefix). Other 4 daemons stay alive.
- **photon_tap.so MUST stay `-DDISABLE_SEND_HOOKS`** per `[[albion-send-hooks-break-client]]`. Do not modify the LD_PRELOAD chain.
- **Token-watcher must remain armed throughout** — verify active before AND after Albion restart.
- **DO NOT modify Unity binaries or wrappers other than the one-time strace-prefixed launch**. Restore original spawn_preload.sh / run-albion-client immediately after capture.
- **Memory budget**: strace log of Albion's first 60s is probably ~50-200MB uncompressed. Process via streaming greps (`grep ... | head`), NEVER load entire log into Python set. Memory cap 4GB per analysis step.
- **One worker per path**: you own this work alone. `albion-photon-login-fuzz` is on a parallel path (Photon encryption RE) — don't touch its worktree.
- **Production daemons stay healthy**: cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest. Only Albion-Online process gets restarted.

## Relevant files / references
- Live container: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Wrapper chain: `/usr/local/bin/run-albion-client` → `/opt/albion-frida-capture/spawn_preload.sh` → `./Albion-Online`
- Existing audit verdicts:
  - `/home/sdancer/albion-token-capture/analysis/unityplayer_seams_audit.md`
  - `/home/sdancer/albion-token-capture/analysis/launcher_args_audit.md`
  - `/home/sdancer/albion-accountportal-cdp/analysis/accountportal_flow_verdict.md`
  - `/home/sdancer/albion-photon-login-fuzz/analysis/auth_request_schema.md`
- Sister-path watcher (DO NOT TOUCH): `albion-token-watcher.service`
- Memory pointers: `[[albion-send-hooks-break-client]]`, `[[albion-vastai-daemon-stack]]`, `[[orchestrator-role]]`, `[[macromanage-workers]]`, `[[no-frida]]`, `[[albion-2fa-container-rotation]]`.

## Reporting
Concise progress at each task boundary. If a runtime URL is captured → fact + milestone. If only known endpoints surface → close path cleanly + suggest next-best (egress-IP-pinning needs VPS resource ask, or wait on parallel photon-fuzz Task 2 verdict). Achievement-levels-+-gaps framing throughout. Hard cap 2h.
