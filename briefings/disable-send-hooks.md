# disable-send-hooks — drop send-side photon_tap, verify post-login crash gone

## Role & workdir
Codex worker at `/home/sdancer/vastai-albion-disable-send-hooks` (worktree, branch `disable-send-hooks`, off `vastai-albion` main commit `14b7ace`).

## Goal
User reports the Albion client crashes after login. Hypothesis (from memory `[[albion-client-wedge-class]]`): the send-side LD_PRELOAD hooks in `photon_tap.so` add enough latency to the client's outgoing Photon ACK path that Photon's "Client not responding" timer fires and the session wedges/crashes shortly after login. **Recv hooks are fine and must stay** — they only observe inbound traffic, no ACK path involved. Only the send-family overrides need to go.

## Success criteria
1. `photon_tap.c` no longer overrides `sendto`/`send`/`sendmsg`/`writev` — glibc resolves them directly. Use a compile-time gate (`#ifndef DISABLE_SEND_HOOKS … #endif` around the four functions) so the change is reversible. Build defaults to **send hooks disabled** for this experiment.
2. Rebuilt `.so` deployed at `/opt/albion-frida-capture/preload/photon_tap.so` on vast.ai. New mtime, file size delta vs prior.
3. Albion process restarted via `spawn_preload.sh` (or however `start_all.sh` brings it up).
4. **The verification.** After user logs in and plays for ≥3 minutes, the Albion process is still alive (same PID as post-launch, no exit) AND `events_processed` on `https://albion.orch.run/state` advances (inbound recv hooks still feeding the ingester — proves the .so is loaded, just send-side is bypassed).
5. Counter sanity: `/tmp/photon_tap.log` on vast.ai shows recv counter lines incrementing but **no `send`/`sendto`/`sendmsg` lines** (those overrides aren't installed, so the debug logger never sees them).

## Substrate facts
- vast.ai SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Source on remote: `/opt/albion-frida-capture/preload/photon_tap.so` (deployed); on local box, source lives at `albion-frida-capture/preload/photon_tap.c` in the worktree.
- Albion runs as user `albion`. Launch wrapper: `/opt/albion-frida-capture/spawn_preload.sh` (sets `LD_PRELOAD` + `PHOTON_TAP_DEBUG=/tmp/photon_tap.log`).
- Gamestate service tunnel: `https://albion.orch.run/state` → `:8765` on vast.ai. Use it to poll `events_processed`.
- The Python gamestate consumer at `/opt/albion-gamestate/gamestate_service.py` is still tailing the session log — recv decoding will continue to work; only send-side decoded frames stop landing in the log.

## Build instructions (worker should derive these from existing repo)
There is no Makefile shipped. The `.so` is typically built with:
```bash
cc -O2 -fPIC -shared -pthread -o photon_tap.so photon_tap.c -ldl
```
on the target host. Build **on vast.ai** (matching glibc/libc ABI) — don't cross-compile from this box. Use scp to push photon_tap.c, build there, restart.

## Anti-patterns (don't)
- Don't `git push` or modify production `/opt/...` files without restarting Albion afterwards — a half-deployed .so will be loaded next launch.
- Don't disable the recv hooks. They're the dashboard's eyes; killing them breaks the whole gamestate service.
- Don't kill the existing Albion process while the user is logged in — coordinate the restart by appending a notification to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` BEFORE killing, asking the user to log out. Wait for their reply (poll the channel) or for the existing Albion process to exit naturally (it crashes — that's the whole point).
- Don't write a wrapper script that toggles send hooks at runtime via env var. Compile-time gate is fine for this experiment.

## Next 2-3 concrete tasks
1. **Edit** `albion-frida-capture/preload/photon_tap.c`: wrap the four send-family override functions (`sendto`, `send`, `sendmsg`, `writev`) in `#ifndef DISABLE_SEND_HOOKS … #endif`. Also wrap the `real_sendto`/`real_send`/`real_sendmsg` initialization in `init()` (lines ~67-70) so unused symbols don't cause warnings — but you can keep them; they're harmless. Commit with message `disable-send-hooks: gate send-family overrides for client-wedge experiment`.
2. **Deploy.** scp the patched `photon_tap.c` to vast.ai (`/tmp/photon_tap.c`), `cc -O2 -fPIC -shared -pthread -DDISABLE_SEND_HOOKS -o /tmp/photon_tap.so /tmp/photon_tap.c -ldl` on the box, atomically replace `/opt/albion-frida-capture/preload/photon_tap.so`, fix ownership (`chown albion:albion`).
3. **Verify the .so is good** without launching Albion: `ldd /opt/albion-frida-capture/preload/photon_tap.so` (no missing symbols), and `nm -D /opt/albion-frida-capture/preload/photon_tap.so | grep -E '^[0-9a-f]+ T (send|sendto|sendmsg|writev)$'` should return **empty** (those symbols are no longer exported as text). Counter-check that `recvfrom`/`recv`/`recvmsg` ARE still exported.
4. **Restart Albion.** Check if Albion is currently running: `pgrep -af Albion-Online | head -3`. If yes, post a message to the talk channel asking user to log out, wait 60s, then `pkill -KILL -f Albion-Online`. Relaunch via the same path used by `start_all.sh` — typically `sudo -u albion /opt/albion-frida-capture/spawn_preload.sh &`.
5. **Verify.** Watch `https://albion.orch.run/state` for `events_processed` to advance over 3-5 min after user logs back in. Watch `/tmp/photon_tap.log` for recv-counter lines (yes) and absence of send-counter lines (correct). If Albion process exits during the window, the hypothesis is FALSIFIED — record that and request user direction. If it stays alive AND ev counter advances, the hypothesis is **supported**: send-side instrumentation was the wedge.
6. **Report.** Append a single milestone line to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`: `{"ts":"<utc>","from":"disable-send-hooks","text":"send hooks disabled — Albion alive Nmin post-login, ev=<value>"}` or the falsified equivalent.

## Constraints & gotchas
- `[[no-frida-libUnreal]]` doesn't apply here — this is LD_PRELOAD on a glibc syscall hook, not Frida.
- `[[albion-client-wedge-class]]` says cold-restart fixes the wedge; that's a hint the wedge state is in-process, not server-side. Confirms our hypothesis: send-side stack manipulation is what nudges the client into the wedge.
- The Python service at `:8765` reads from a JSONL session log. Disabling send-side capture means the log loses outgoing `request:Move` frames — `self.x`/`self.z` will only update via incoming `Move` echoes if present, so the dashboard's "Self position" may degrade slightly. That's acceptable for this experiment.
- Memory cap: codex sandbox is fine; nothing here is memory-heavy.

## Relevant files / references
- Local source: `/home/sdancer/vastai-albion-disable-send-hooks/albion-frida-capture/preload/photon_tap.c`
- Deploy path on remote: `/opt/albion-frida-capture/preload/photon_tap.so`
- Launch wrapper on remote: `/opt/albion-frida-capture/spawn_preload.sh`
- Log path on remote: `/tmp/photon_tap.log`
- Dashboard: `https://albion.orch.run/state` (JSON; key fields: `events_processed`, `self.zone`)
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Worktree branch: `disable-send-hooks` off `vastai-albion@14b7ace`

## Memory pointers
- `[[albion-client-wedge-class]]` — the cold-restart pattern this experiment may explain
- `[[worker-artifact-isolation]]` — vast.ai daemons don't need a Hetzner-style cgroup; nohup setsid is fine
- `[[macromanage-workers]]` — pick your own tactics for atomic .so swap
