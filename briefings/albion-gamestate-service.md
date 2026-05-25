# albion-gamestate-service — add "Actions" tab showing emitter audit log

## Role & workdir
Codex worker (codex_app_server, same durable thread from earlier turns). Local workdir: `/home/sdancer/albion-gamestate-service`. Live deploy: `/opt/albion-gamestate/gamestate_service.py` on vast.ai container, reachable via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
Add a new tab on the public Albion dashboard at `https://albion.orch.run/` that displays the most recent action emitter audit-log entries with auto-refresh. Closes the observability loop on the stage-3 action emitter (the 6th supervised daemon) added in prior cycles.

## Success criteria
1. `https://albion.orch.run/` shows a new tab labelled "Actions" (or "Action Log") alongside the existing Overview / Screenshot tabs.
2. The tab displays the most recent ~20-50 audit-log entries in reverse-chronological order. Each row shows at minimum: timestamp (HH:MM:SS), `policy_branch`, `action.type`+`action.key` (or pretty form), `dispatch_latency_ms`, `dispatch_result`.
3. Auto-refresh every 1-2 seconds (cache-busted query string is fine; same pattern as the screenshot tab).
4. Aggregate **branch counts** for the rolling window are visible at the top of the tab (e.g. `login_screen: 12 · in_zone: 0 · no_state: 0 · idle: 1`).
5. **No regression**: `/state` still HTTP 200, `/vnc/index.html` still 200, `/screenshot.png` still 200, action-emitter daemon still alive (its audit log keeps growing — your tab is a passive reader).
6. Milestone posted to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` with: tab URL/anchor, curl-evidence on the new endpoint (e.g. `GET /actions.json` returning JSON with non-empty `entries`), and confirmation `/state /vnc /screenshot.png` all still 200.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| 1 | `gamestate_service.py` running on 127.0.0.1:8767, fronted by nginx, serving the dashboard | Dashboard backend is live + healthy | vast.ai | ✅ DONE |
| 2 | Screenshot tab @ `#screenshot` + `/screenshot.png` HTTP 200 image/png 4.15MB | Inline-HTML tab pattern already established in this dashboard | vast.ai | ✅ DONE |
| 3 | nginx `/vnc/` route with HEAD→GET shim | nginx route layer is extensible | vast.ai | ✅ DONE |
| 4 | Action emitter writes JSONL to `/var/log/albion-action-emitter/actions-<YYYY-MM-DD>.jsonl` with fields `ts`, `policy_branch`, `action`, `dispatch_latency_ms`, `dispatch_completion_ms`, `dispatch_result` | Audit-log producer side is established and verified | vast.ai | ✅ DONE |

## Tasks (figure out the details from live code; do NOT ask me to pre-fetch)

### Task 1 — Plan the file-reader (concise)
The dashboard service (`gamestate_service.py`) currently runs on `127.0.0.1:8767` under `albion-supervise`. The audit log file is owned by user `albion` and lives at `/var/log/albion-action-emitter/actions-<today>.jsonl`. Check:
- Which user runs `gamestate_service.py` (probably `root` or `albion`; check `ps`).
- Whether that user can read the audit log directory + file. If not, fix via group membership or `chmod g+r` rather than running gamestate as root.
- Whether the directory has a yesterday/today rollover problem — handle it (glob `actions-*.jsonl` and merge last N entries sorted by `ts`, or just always read today's file and accept midnight rollover gap).

### Task 2 — Implement
Add a route `/actions.json` to `gamestate_service.py` that:
- Reads the last N (~50, configurable via query param `?limit=N`, cap at 500) JSONL entries from the audit log.
- Returns JSON: `{"entries": [...], "branch_counts": {"login_screen": N1, "in_zone": N2, ...}, "newest_ts": "<iso>", "audit_log_path": "..."}`.
- Cache for ~200ms to avoid hammering the file on N concurrent viewers.
- Returns `Content-Type: application/json` with `Cache-Control: no-store`.

Add the frontend tab — same pattern as the screenshot tab. Polled fetch + DOM render. Don't pull in heavy frameworks; vanilla JS + a small table is enough.

### Task 3 — Deploy + verify
Sync to `/opt/albion-gamestate/`. Signal the supervisor to restart only the gamestate child (preserve the supervisor wrapper PID). Verify externally:
- `curl -sk https://albion.orch.run/actions.json | jq '{newest_ts, branch_counts, entry_count: (.entries|length)}'` returns non-empty entries.
- Open the dashboard, confirm the Actions tab renders the audit rows + the branch_counts header.
- `curl -sIk` for `/state`, `/vnc/index.html`, `/screenshot.png` all still 200.

### Task 4 — Milestone
Append to `vastai-albion-web.jsonl` one JSON line with: tab URL+anchor, curl evidence for `/actions.json` (entry_count, branch_counts, newest_ts), 4 other endpoint 200-checks.

## Constraints & gotchas
- **Don't restart the action-emitter daemon.** You're the reader; the writer must keep running uninterrupted. Permission issues are fixed via filesystem ACL or group membership, NOT by restarting the emitter as a different user.
- **Don't disrupt /state /vnc/ /screenshot.png.** Only the gamestate child restarts; nginx + cloudflared + KasmVNC + Albion + photon-pcap + frida-ingest + action-emitter ALL stay up.
- **Don't `pkill -9`.** Supervisor signal patterns only.
- **Modular boundary preserved.** Don't merge the action-emitter code into gamestate_service. The interface is the JSONL file on disk.
- **No new pip deps.** Stdlib only (the inline-HTML dashboard already avoids template engines).
- **No HEAD-handling regression** on `/vnc/` (the prior `proxy_method GET` nginx rule must stay).
- **Bound resource use.** Reading the last 50 lines of a JSONL file is a `seek-from-end + tail` operation, not a `read whole file`. Keep it cheap.
- **License**: this is your own Python code; no GPL imports.

## Relevant files / references
- Local: `/home/sdancer/albion-gamestate-service/scripts/gamestate_service.py` (or wherever the inline HTML lives — confirm).
- Live: `/opt/albion-gamestate/gamestate_service.py` on container.
- Audit log path: `/var/log/albion-action-emitter/actions-<YYYY-MM-DD>.jsonl`.
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`.
- Memory: `[[albion-vastai-daemon-stack]]`, `[[albion-send-hooks-break-client]]`.

## Reporting
Either (a) milestone with curl proof of `/actions.json` non-empty + 4 endpoint 200-checks, OR (b) precise blocker description (permission denied on log read, etc.). Not "I think it works".
