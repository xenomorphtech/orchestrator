# Repro Dry-Run Gaps

Date: 2026-06-04

Scope: static/dry-run audit of `clientless-deploy/bootstrap.sh` before any destructive falsification test. Do not OS-reinstall the live sg2 box `139.180.144.61`; use a throwaway target.

## Shellcheck

Command:

```bash
shellcheck clientless-deploy/bootstrap.sh
```

Output after inline fixes:

```text
<no output; exit 0>
```

Also checked:

```bash
bash -n clientless-deploy/bootstrap.sh
```

Result: exit 0.

## Idempotency Notes

| Bootstrap area | Current behavior | Re-run safety |
| --- | --- | --- |
| SSH reachability | Waits for root SSH with `StrictHostKeyChecking=accept-new`. | Safe; no state mutation until SSH is reachable. |
| Apt/rust/claude/cloudflared deps | Installs missing packages/tools; package installs are naturally convergent. | Safe; package manager may upgrade only if apt chooses to satisfy missing package state. |
| User/sudo/key setup | Creates `sdanced` if absent, appends operator key only if missing, rewrites sudoers file. | Safe for intended user; overwrites `/etc/sudoers.d/90-sdanced` by design. |
| Claude creds/skill | Copies host credentials and skill each run. | Convergent, but shares the same Claude auth state across boxes. |
| Pool SSH key/inventory | Copies account key and latest pool inventory when present. | Convergent, but shares the same pool account key across boxes. |
| Clientless source | Rsyncs code into `~/clientless`. | Convergent for code; does not delete `~/clientless/analysis`. |
| Analysis seed | Rsyncs seed files to `~/clientless/analysis.seed`; copies `goal_tree.json` and `STATE.md` into `analysis/` only when absent. | Safe for live state; reruns preserve active goal tree and state. |
| Knowledge base/tools | Rsyncs host `albion-wiki` and `albion/tools` with `--delete`. | Convergent; will remove target files absent from host canonical trees. |
| Harness/dashboard source | Rsyncs source with `--delete`, builds as `sdanced`, writes harness binaries/config. | Convergent; source target dirs are owned by bootstrap. |
| Systemd units | Rewrites fixed unit files and enables them. Starts only if no live unmanaged process/port occupies the role. | Safe on a single target; fixed names need isolation for multiple units on one host. |
| Harness DB publish | If `orchestrator-box` is reachable, skips publish. Otherwise publishes with `--delete-data=never`. | Safe for existing DB; fresh box creates DB. |
| Projection cron | Removed; bootstrap prunes legacy `gt2paths.py` crontab entries. DB paths are written directly by `tick.sh`. | Safe/convergent. |
| Health check | Reads DB, dashboard, units, loop logs. | Non-destructive. |

## Collision Matrix

These are the external resources or fixed names that matter if a second bootstrap target runs at the same time as live sg2.

| Resource | Current bootstrap value | Collision with live sg2? | Isolation needed for throwaway |
| --- | --- | --- | --- |
| Cloudflare public route / tunnel | Existing sg2 tunnel token in `CLOUDFLARE_TUNNEL_TOKEN`; unit name `cloudflared-sg2.service`; description says sg2 tunnel. | Yes. Reusing the same tunnel token can attach another connector to the same Cloudflare tunnel/public hostname and can route test traffic to the wrong box. | Create a distinct Cloudflare tunnel and hostname for the throwaway, e.g. `throwaway-<id>.orch.run`; pass a distinct `CLOUDFLARE_TUNNEL_TOKEN`. Bootstrap should grow `CLOUDFLARED_SERVICE_NAME` and `CLOUDFLARED_TUNNEL_LABEL` or equivalent before parallel tests. |
| Cloudflare API token | `CLOUDFLARE_API_TOKEN` from host `.env`; not accepted as tunnel token unless it has tunnel-token shape. | No direct tunnel collision if ignored, but it is a shared account credential. | Keep as host-only secret. For isolated tunnel creation, use API token only in a separate provisioning step that creates a new tunnel token. |
| Dashboard public hostname | Implicitly sg2 through current Cloudflare tunnel. | Yes, via tunnel token/route. | Use a throwaway hostname and token; do not point it at sg2 route. |
| SpacetimeDB listen port | `127.0.0.1:3001`. | No cross-host collision. Collides only if two deployments share one host/network namespace. | Add `SPACETIME_LISTEN_ADDR`/`SPACETIME_PORT` before same-host tests; for a separate VM this can stay `3001`. |
| Dashboard listen port | Dashboard binary default `0.0.0.0:3030`. | No cross-host collision. Collides only if two deployments share one host/network namespace. | Add dashboard bind/port env support if same-host testing is needed; for a separate VM this can stay `3030`. |
| Harness DB name | `orchestrator-box`. | No cross-host collision because each SpacetimeDB is local. Operationally ambiguous in dashboards/logs across boxes. | Add `HARNESS_DATABASE_LOCAL` override, e.g. `orchestrator-throwaway-<id>`, before multi-box comparisons. |
| Harness server URL | `http://127.0.0.1:3001`. | No cross-host collision. | Tie to `SPACETIME_LISTEN_ADDR` if port is parameterized. |
| Systemd unit names | `spacetimedb-box.service`, `orchestrator-dashboard.service`, `cloudflared-sg2.service`, `clientless-lean-loop.service`. | No cross-host collision. Collides on same host and the `cloudflared-sg2` name is misleading for throwaways. | Add unit name prefix/env, e.g. `UNIT_PREFIX=clientless-throwaway`, before same-host tests; at minimum rename cloudflared unit label for throwaway clarity. |
| Fixed remote user/home | `sdanced`, `/home/sdanced`. | No cross-host collision. Same-host second deployment collides completely. | Use `NONROOT_USER=<test-user>` for same-host tests; separate VM can use `sdanced`. |
| Compatibility tree | `/home/sdancer/orchestrator` on target plus `/home/sdanced/orchestrator`. | No cross-host collision. Same-host second deployment collides. | Parameterize compatibility path or skip it for same-host tests. |
| Clientless workdir | `/home/sdanced/clientless`. | No cross-host collision. Same-host second deployment clobbers code and seed dirs. | Use `NONROOT_USER` or a future `CLIENTLESS_HOME` for same-host tests. |
| Knowledge/tool paths | `/home/sdanced/albion-wiki`, `/home/sdanced/albion/tools`. | No cross-host collision. Same-host second deployment clobbers target mirrors. | Use `NONROOT_USER` or future path envs for same-host tests. |
| Claude credentials | Host `/home/sdancer/.claude/.credentials.json` copied to target. | Possible external auth/session coupling across boxes. | For throwaway, either accept short test reuse or provide `CLAUDE_CREDENTIALS_FILE`/test account credentials. |
| Pool account SSH key | Host `/home/sdancer/.ssh/id_ed25519` copied to target. | Shared access credential; not a port/name collision. | For throwaway safety, allow `POOL_SSH_KEY` override or omit pool key when testing only bootstrap reachability. |
| Pool inventory | Host `analysis/pool_inventory.json` copied to `~/clientless/pool.json`. | Shared pool substrate; two boxes could dispatch work to same pool boxes. | For throwaway, pass a test inventory or empty pool file via `POOL_INVENTORY` override. |
| SSH known-host trust | `StrictHostKeyChecking=accept-new`; operator must run `ssh-keygen -R` after reinstall. | Host-key collision if IP is reused after reinstall. | Always use a new throwaway IP or clear the specific known-host entry before bootstrap. |

## Fresh-Box Manual Touch Gap List

| Gap | Why it still matters | Required action before destructive/throwaway falsification |
| --- | --- | --- |
| Throwaway target selection | Live sg2 must not be OS-reinstalled. | Provision a separate VM and point bootstrap at that IP. Prefer a short-lived Vultr Cloud Compute instance in the same region, or a fresh Vast CPU-like throwaway if root SSH is available. |
| Distinct Cloudflare tunnel | Current sg2 tunnel token is live traffic infrastructure. | Create a throwaway Cloudflare tunnel/hostname and set `CLOUDFLARE_TUNNEL_TOKEN` to that token for the test, or disable cloudflared start for the throwaway until isolation envs exist. |
| DB/service naming clarity | `orchestrator-box` and `cloudflared-sg2.service` are sg2-flavored. | Add env overrides for DB name and cloudflared service/tunnel label before evaluating multi-box dashboards. |
| Claude auth coupling | Bootstrap copies live Claude OAuth credentials. | Decide whether throwaway may share live auth for the short test or supply a throwaway credential file. |
| Pool side effects | Bootstrap seeds real pool access/inventory. | Use an empty/test `pool.json` or disable pool key copy for a pure repro test. |
| Host path dependency | Bootstrap assumes host source paths under `/home/sdancer/...`. | Run from this orchestrator host, or add source path env overrides before testing elsewhere. |

## Recommended Safe Throwaway

Use a separate, short-lived Vultr Cloud Compute Ubuntu VM in Singapore, not `139.180.144.61`. It is close enough to the target OS/network shape for bootstrap validation and avoids touching live sg2. The smallest shared instance is sufficient for early bootstrap phases; Rust builds may be slow but acceptable for a one-off falsification. Current public pricing shows Regular Performance Cloud Compute starting at `$2.50/mo`; Vultr bills Cloud Compute hourly up to 672 hours/month, so the burn rate is roughly `$0.004/hr` before add-ons. Destroy immediately after the test.
