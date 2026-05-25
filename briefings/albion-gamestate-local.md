# albion-gamestate-local — Real /state for acct_3 closed-loop

## Role & workdir
Codex worker. Workdir: **`/home/sdancer/albion-gamestate-local`** (fresh `local-substrate` branch off albion-gamestate-service `df89dc4`).

## Current goal
- **goal_key**: `albion_action_loop` (umbrella)
- **success_fact_key**: `albion_gamestate_local_live`
- **success metric (this phase)**: a closed-loop dispatch on acct_3 produces a measurable change in `self.x`/`self.z` reported by `http://localhost:8765/state`, with all of: (a) the gamestate service running as a systemd user unit, (b) emit.py pointed at `localhost:8765` instead of any stub, (c) at least one audited right-click dispatch followed by `Δ(self.x,self.z) ≠ (0,0)` in the next `/state` poll within 5s.

## Already achieved (do NOT re-falsify — read these before starting)

| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L1 | `/home/sdancer/albion-pcap-decode/analysis/op_code_tables.md` + `move_field_layout.md` | Photon framing + Move event x/z byte offsets | ✅ DONE |
| L2 | `/home/sdancer/albion-pcap-decode/analysis/player_position_stream.csv` (2256 rows) | offline pcap → position stream proven | ✅ DONE |
| L3 | `albion-gamestate-service` repo: scripts/units/README | HTTP /state daemon shipped for vast.ai (ssh8.vast.ai:14838 → :8765); rotates pcap; tails via `--capture-glob` | ✅ DONE |
| L4 | acct_3 Albion (PID 19970) live UDP traffic to `193.169.238.124:5056` | substrate is emitting decryptable Photon | ✅ VERIFIED (`ss -tunap` 2026-05-25T07:04) |
| L5 | `emit.py --gamestate-url` already accepts URL → polls HTTP `/state` | client wiring exists | ✅ DONE (closed-loop already proven against stub in `albion-action-emitter-local`) |
| L6 | substrate self-heals via `albion-acct3-watchdog.service` (detector v2 multi-crop) | acct_3 stays in_zone across camera moves | ✅ DONE 2026-05-25T04:58Z |
| L7 | xdotool right-click ground-move produces displacement on acct_3 | dispatch primitive PROVEN | ✅ DONE 2026-05-25 (`albion_action_emitter_shipped`) |

**You are NOT reinventing.** The decoder works. The HTTP daemon works. The dispatcher works. The substrate is self-healing. You're wiring the pieces together for the LOCAL substrate.

## Hypothesis
The vast.ai-targeted gamestate-service (tcpdump on a rotating ring → Rust decoder → JSON over HTTP) can be redeployed against acct_3's local Photon UDP egress (`193.169.238.124:5056` from PID 19970), exposing `localhost:8765/state` with a populated `self.zone.name + self.x + self.z` that updates within ~1s of a right-click ground-move dispatch.

## Falsification (mechanism-scoped — read [[falsify-mechanism-not-path]] before any *_blocked.md)
- **Mechanism under test**: tcpdump-on-eth0 + offline-style Rust decoder + HTTP /state daemon (passive-pcap class).
- **Falsified iff**: after deployment, a right-click dispatch at a known-good ground tile is observable in the audit JSONL but the *next 10 polls of localhost:8765/state* show identical `(self.x, self.z)`.
- **Untried siblings (must enumerate before path-close)**: photon_tap.so LD_PRELOAD with `-DDISABLE_SEND_HOOKS` (per `[[albion-send-hooks-break-client]]` — recv-side only), eBPF kprobe on `udp_sendmsg`/`udp_recvmsg`, kernel module pcap tap, memory-read of Unity gamestate via `/proc/19970/mem` + cached field offsets. List ≥3 in any *_blocked.md.

## Known risk — session mask
The Move event payload XORs current x/z with an 8-byte session mask (see `move_field_layout.md`). This mask is per-session. The vast.ai pcap_decode worker resolved it for that capture; whether the same recovery works for a fresh local session is OPEN. If decoder produces non-numeric or oscillating x/z, mask recovery is the blocker — search prior work under `/home/sdancer/dark-december-move-decode/`, `/home/sdancer/dark-december-coord-param-disambig/`, `/home/sdancer/dark-december-xor-key-recover/` before treating as new.

## Tasks 1 — DONE (prior turn)
- `scripts/deploy_local.sh` written, installs to `/opt/albion-gamestate-local/`, captures on eth0.
- `decode_lib.py` extended for DLT_EN10MB (root cause of 0-events first pass).
- Live smoke (07:15Z): /state reported `events_processed=5`, real `request:Move` positions decoded; right-click at (1120,300) on acct_3 produced z drift from -91.83 → -86.91 (Δz=4.9). Closed-loop chain proven.
- Smoke artifact: `analysis/task1_local_smoke_2026-05-25.md`. Decoded sample: `analysis/local_smoke_events_2026-05-25.jsonl` (76KB).
- ⚠️ Daemon (tcpdump + HTTP service) DIED on worker turn-end per `[[worker-artifact-isolation]]`. Must install proper systemd unit before further work.
- ⚠️ `zone.name = null` because mid-session restart missed the `response:Join`. Bootstrap fix needed in Task 2.

## Next 3 concrete tasks (~60min total)

### Task 2A (~15min) — systemd USER units (CRITICAL — daemon currently DEAD)

1. Create `~/.config/systemd/user/albion-gamestate-local-capture.service` running `/opt/albion-gamestate-local/start_capture.sh` (tcpdump), `Restart=always`, runs as user sdancer with sudo permission for tcpdump (test if needed).
2. Create `~/.config/systemd/user/albion-gamestate-local-service.service` running `/opt/albion-gamestate-local/start_gamestate.sh` (HTTP daemon on 127.0.0.1:8765), `Restart=always`, `After=albion-gamestate-local-capture.service`.
3. Use `XDG_RUNTIME_DIR=/run/user/1001 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1001/bus systemctl --user daemon-reload && systemctl --user enable --now ...` (same pattern as `albion-acct3-watchdog.service`).
4. Verify both `is-active`, `/state` curl returns events.

### Task 2B (~15min) — zone.name bootstrap

Once the daemon is running under systemd (so it survives across right-click probes), the next acct_3 zone-transition (loading screen, character logout/login via watchdog, etc.) will provide a `response:Join` whose payload the existing service should already know how to consume. Option A: rely on that natural session boundary — leave the daemon running and document the trigger. Option B: write a one-shot `bootstrap_zone.py` that reads the most recent join from the pcap ring archives (if any pcap before mid-session restart has the join). Option C: parse zone name from a side channel (Photon `Join` request payload, or a Unity log file if Albion writes one). Pick whichever path you can confirm in 15min.

### Task 3 (~20min) — End-to-end closed-loop verdict + fact

1. With the daemon under systemd: fire 1 right-click dispatch at a known-good tile on acct_3 via the proven emit.py path (`runuser -l sdancer -c 'DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority xdotool ...'`), capture 10 polls of `/state` over 10s spanning the dispatch.
2. Verify (a) pre-dispatch `(self.x, self.z)` stable, (b) post-dispatch Δ ≠ (0,0).
3. Commit `analysis/gamestate_local_verdict_2026-05-25.md` with: systemd unit listings, sample `/state` JSON pre + post, Δ(x,z) numerical value, dispatch audit row, and known-zone-name status.
4. Set fact: `harness fact-set albion_gamestate_local_live "localhost:8765 HTTP /state under systemd (capture+service user units); closed-loop Δ(x,z)=<value> via right-click on acct_3 Veldra1203; zone.name=<value-or-pending>; artifact analysis/gamestate_local_verdict_2026-05-25.md"`.

## Commit-or-falsify contract (per [[briefing-commit-or-falsify-contract]])
- 90min hard cap. Else `analysis/gamestate_local_partial_2026-05-25.md` with whatever you have + ≥3 untried mechanisms.
- 15min heartbeat → `analysis/heartbeat.log`.
- `/tmp/abort_albion-gamestate-local` → commit partial + exit.

## Constraints (HARD)
- **NEVER touch acct3-albion.service / acct3-xtigervnc.service** (substrate).
- **NEVER touch `albion-acct3-watchdog.service`** (substrate keepalive).
- **NEVER deploy to /opt/albion-gamestate/** (that's the vast.ai production path — different system, different DB; would clobber the working prod deploy).
- **NEVER overwrite `/home/sdancer/albion-gamestate-service/` working files** — your worktree is `/home/sdancer/albion-gamestate-local/` on branch `local-substrate`.
- **NEVER LD_PRELOAD photon_tap.so with send-side hooks** (per `[[albion-send-hooks-break-client]]`). If you go that route, build with `-DDISABLE_SEND_HOOKS`.
- **NEVER bind to 0.0.0.0:8765 here** — that conflicts with the production daemon's port convention. Bind explicitly to 127.0.0.1.
- tcpdump on eth0 requires capability or root. Check whether the user can `tcpdump -i <iface> -w ...` without sudo; if not, use `sudo` (yes, that's OK locally) or `setcap cap_net_raw,cap_net_admin+eip` on the binary.
- Do NOT commit pcap files to git (they're large + contain session data). Add `*.pcap` to `.gitignore`.

## Memory references
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure, ≥3 untried siblings.
- `[[albion-send-hooks-break-client]]` — send-side photon_tap breaks client; recv-side OK with caveats.
- `[[check-existing-decoder-before-re]]` — sweep dark-december sibling worktrees before re-doing decode work.
- `[[macromanage-workers]]` — you self-discover the deploy details; orchestrator names goal + verification artifact.

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-gamestate-local.md`
- This worktree: `/home/sdancer/albion-gamestate-local/` (branch `local-substrate`)
- Reference repo (DO NOT modify): `/home/sdancer/albion-gamestate-service/`
- Reference decoder + Move layout: `/home/sdancer/albion-pcap-decode/analysis/{op_code_tables.md,move_field_layout.md,player_position_stream.csv}`
- Reference closed-loop client: `/home/sdancer/albion-action-emitter-local/emit.py` (do not modify it; you create a local launcher script that targets http://localhost:8765/state)
- Dark-december mask work (search if you hit session-mask issues): `/home/sdancer/dark-december-{move-decode,coord-param-disambig,xor-key-recover,move-kuprobe-cipher-state}/`
- Substrate: acct_3 Albion PID 19970, peer `193.169.238.124:5056`
- Harness: `/home/sdancer/orchestrator/harness`
- Fact to set on success: `albion_gamestate_local_live`
