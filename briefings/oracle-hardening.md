# oracle-hardening — Production-harden the remote NMSS cert oracle service

## Role & workdir
You are a deploy/ops Codex worker. Workdir: `/home/sdancer/nmss-emu-oracle-hardening/`.

## Current goal / sub-goal
- **goal_key**: `nmss_cert_oracle_production_hardening` (new this cycle 163; previously planner-suggested)
- **sub_goal_key**: `oracle-hardening`

## Current oracle state (concrete facts to ground you)
- Live python process: `root@162.244.80.97`, PID 29708, since 2026-05-11, command `python3 /root/oracle-service/server.py --bind 127.0.0.1 --port 9876`
- Listens on **127.0.0.1:9876** (localhost-only on the remote ARM box — NOT publicly reachable)
- Code lives at `/root/oracle-service/server.py` (on the remote) + `/home/sdancer/nmss-emu-oracle-service/oracle_service/` (local dev copy)
- 5/5 cert smoke test PASSES; cycle-29 deliverable is sound
- Per-cert latency 17-19s (snapshot-load-heavy, NOT optimizable without rearchitecting)
- The harness service monitor reports it "unhealthy" because there's NO SYSTEMD UNIT — only the raw python process. This is the primary hardening gap.

## Success criteria — production_readiness_score / 4 (binary subgoals)
A working `oracle-service.service` unit on the remote ARM box such that:

1. **systemd unit installed**: `systemctl is-active oracle-service` returns `active` on the remote. Unit at `/etc/systemd/system/oracle-service.service` with `Restart=always` + `RestartSec=10s`.
2. **Auto-restart verified**: kill the python PID, wait 15s, confirm `systemctl is-active oracle-service` is `active` again AND the new PID differs from the old.
3. **Metrics endpoint returns**: `curl -s http://127.0.0.1:9876/metrics` returns Prometheus-style text (at minimum: `oracle_certs_served_total`, `oracle_request_duration_seconds`). Modify `server.py` to add this endpoint.
4. **Smoke 5/5 still passes**: re-run `python3 /root/oracle-service/scripts/smoke_5x.py` after hardening; expect `5/5 PASS` (no regression).

Success = all 4 pass. Set fact `nmss_cert_oracle_hardened_4_of_4` with the systemd unit content + smoke result.

## DO NOT scope
- Do NOT attempt p99-latency optimization. The 17-19s/cert is algorithm-bound (snapshot load) — that's a separate work item with its own metric.
- Do NOT add TLS or auth — the service is localhost-bound on the remote; no external attack surface.
- Do NOT change the cert algorithm. Pure-Rust path from `/home/sdancer/nmss-emu/cert-rust-repro` MUST remain 5/5.

## Execution flow — DO NOT EXIT BETWEEN STEPS
Per memory rule: prior workers exited after "Task 1 complete". Treat steps 1-5 as ONE atomic execution. Do not summarize and stop; run all 5 and only then return.

**Step 1** — SSH to `root@162.244.80.97` and read `/root/oracle-service/server.py`. Note the imports, the cert handler, the bind setup. Decide how to add a `/metrics` HTTP handler that exposes 2-3 Prometheus counters.

**Step 2** — Edit `server.py` (in-place on the remote, with `scp` first to back up to `server.py.bak`) to add:
- `oracle_certs_served_total` counter (increments on each successful cert response)
- `oracle_request_duration_seconds` histogram (record per-cert latency)
- `/metrics` endpoint that emits both in Prometheus text format
- Keep the existing `/cert` handler unchanged.

**Step 3** — Write the systemd unit `/etc/systemd/system/oracle-service.service` on the remote with:
```
[Unit]
Description=NMSS cert oracle service
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/oracle-service
ExecStart=/usr/bin/python3 /root/oracle-service/server.py --bind 127.0.0.1 --port 9876
Restart=always
RestartSec=10
StandardOutput=append:/var/log/oracle-service.log
StandardError=append:/var/log/oracle-service.log

[Install]
WantedBy=multi-user.target
```

**Step 4** — Cut over: stop the legacy PID 29708, `systemctl daemon-reload`, `systemctl enable --now oracle-service`. Verify `systemctl is-active oracle-service` is `active` and the new PID is up.

**Step 5** — Run all 4 success checks in sequence:
- A: `ssh root@162.244.80.97 'systemctl is-active oracle-service'` → expect `active`
- B: auto-restart test — `ssh root@162.244.80.97 'pid=$(systemctl show -p MainPID --value oracle-service); kill -9 $pid; sleep 15; new_pid=$(systemctl show -p MainPID --value oracle-service); [ "$new_pid" != "$pid" ] && [ "$new_pid" != "0" ] && echo OK || echo FAIL'`
- C: metrics endpoint — `ssh root@162.244.80.97 'curl -s http://127.0.0.1:9876/metrics'` should show `oracle_certs_served_total{` lines
- D: smoke 5/5 — `ssh root@162.244.80.97 'cd /root/oracle-service && python3 scripts/smoke_5x.py'` should show 5/5 PASS

Write artifact `/home/sdancer/nmss-emu-oracle-hardening/analysis/oracle_hardening_2026-05-15.md` with all 4 check outputs + the systemd unit file content + the new `server.py` diff. Set fact `nmss_cert_oracle_hardened_4_of_4` (success) OR `nmss_cert_oracle_hardened_N_of_4` (partial with gap analysis).

Print `ORACLE_HARDENED_DONE` on the final line.

## Constraints & gotchas
- **Hard 500 MB memory budget.** Pure dev/ops work, no large-binary analysis.
- **Remote SSH credentials**: see `/home/sdancer/orchestrator/.env` (ARM64_HOST=162.244.80.97, ARM64_USER=root, ARM64_PASSWORD=Something12). Use `sshpass -p "$ARM64_PASSWORD" ssh ...` OR existing ssh-agent.
- **DO NOT TOUCH** the cert algorithm or `/home/sdancer/nmss-emu/cert-rust-repro/`. Read-only deploy work.
- **Backup before overwrite**: always `cp server.py server.py.bak` on the remote before any edit.
- **No `--no-verify`** equivalent if git is involved.
- **systemd unit reload order matters**: edit unit file → `systemctl daemon-reload` → `systemctl enable --now`.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-oracle-hardening/`
- remote oracle code: `root@162.244.80.97:/root/oracle-service/`
- local oracle dev copy: `/home/sdancer/nmss-emu-oracle-service/oracle_service/`
- harness service monitor (currently 14-fail): `oracle-service-arm` in `harness services`
- fact keys: `nmss_cert_oracle_hardened_4_of_4` (success) / `nmss_cert_oracle_hardened_N_of_4` (partial)
- success-fact-key: `nmss_cert_oracle_hardened_4_of_4`
