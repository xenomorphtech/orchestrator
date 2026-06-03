# Flask Route Inventory

Source read in full: `web/app.py` (1188 lines).

## Global Auth And Shared Data

- Auth gate: `before_request` redirects every path except `/login*` and `/static*` to `/login` unless `dash_auth` cookie validates with `URLSafeTimedSerializer(secret_key, salt="dashboard-auth-cookie-v1")` and equals `sha256("dashboard-salt-v1:" + DASHBOARD_PASSWORD)`.
- Password: `DASHBOARD_PASSWORD` env or default `welcome to my w0rld`; cookie max age 30 days, httponly, `SameSite=Lax`, not secure.
- Secret key: `/home/sdancer/orchestrator/.dash_secret_key`, created `0600` if missing.
- Harness CLI: `/home/sdancer/orchestrator/harness`; table parser splits pipe-delimited headers/rows, skips separator lines and `(N rows)`.
- SQL: POST text SQL to `{HARNESS_SERVER}/v1/database/{HARNESS_DATABASE}/sql`; env overrides, then `/home/sdancer/orchestrator/harness.toml`, then defaults.
- Files: `/home/sdancer/orchestrator/analysis/paths.json`, `/home/sdancer/orchestrator/briefings/*.md`, `/home/sdancer/.claude/projects/-home-sdancer-orchestrator/memory/*.md`, talk logs under `/home/sdancer/orchestrator/analysis/talk_channels/*.jsonl`.
- Base template: inline `BASE` via `render_template_string`; Tailwind CDN, nav links, optional meta refresh, `{{ body|safe }}`.

## Routes

- `GET /login`: if authed redirects `302` to `/talk?c=general`; otherwise renders login form template. Data calls: cookie validation only.
- `POST /login`: form `password`; on match sets `dash_auth` cookie and redirects `303` to `/talk?c=general`; on mismatch renders login form with error. Data calls: password hash + secret signer.
- `GET /logout`: deletes `dash_auth` cookie and redirects `302` to `/login`. Data calls: none.
- `GET /`: dashboard page with refresh 30. Data calls: `episodes(8)` via SQL newest-id search with CLI fallback, `goals()` via `harness goals`, `paths_portfolio()` from `analysis/paths.json`, `services()` via `harness services`. Template sections: active campaigns, path portfolio summary, recent cycles, services.
- `GET /cycles`: latest 200 cycles. Data calls: `episodes(200)` via SQL with CLI fallback. Template: table id/when/agent count/action count/summary.
- `GET /cycle/<cid>`: cycle detail, `cid` integer path. Data calls: SQL by id with fallback to `episodes(500)`. Template: summary card plus JSON pre blocks for agents/actions/goal progress. Returns 404 if missing.
- `GET /goals`: goals list. Data calls: `harness goals`, sorted active first then priority descending. Template: status badge, priority, key link, title.
- `GET /goal/<key>`: goal detail. Data calls: `harness goals`, exact `goal_key` match. Template: badge, priority, title, optional detail, success/completion/created/updated grid. Returns 404 if missing.
- `GET /paths`: path portfolio. Data calls: `analysis/paths.json`. Template: one section per goal with metric/current/target/last move and per-path table.
- `GET /agents`: agents list. Data calls: `harness agents`. Template: table name/kind/workdir/description or empty notice.
- `GET /facts`: latest 80 facts. Data calls: `harness facts`, reversed and truncated. Template: table created/key/value.
- `GET /services`: services list. Data calls: `harness services`. Template: table name/type/status/last poll/target.
- `GET /memory`: memory index. Data calls: `list_md(MEMORY)` and `read_md(MEMORY, "MEMORY.md")`. Template: optional open MEMORY.md details block and file list sorted by mtime descending.
- `GET /memory/<name>`: memory detail. Data calls: guarded markdown read from `MEMORY`; rejects `/` and `..`. Template: raw markdown in pre plus back link. Returns 404 if missing.
- `GET /briefings`: briefing index. Query input `show=active|archived|all` default active. Data calls: `harness briefing-list`, `briefing-list --only-archived`, or `briefing-list --archived`; normalized `(none)` cells. Template: tabs, grouped by category, table name/goal/tags/updated with archived chip.
- `GET /briefings/<name>`: briefing detail. Path may end in `.md`; base name used. Data calls: `harness briefing-list --archived`, `harness briefing-get <base>`, fallback to `/home/sdancer/orchestrator/briefings/<base>.md`. Template: metadata chips plus escaped markdown in pre. Returns 404 if missing.
- `GET /talk`: talk UI for query `c` channel slug, default `general`. Data calls: `list_talk_channels()`, `talk_entries(channel, 200)`. Template: channel sidebar, create/clear/delete forms, messages, post form, polling JavaScript. No meta refresh.
- `POST /talk`: form `from`, `text`; query `c` channel. Data calls: append JSONL entry; if sender is `user`, fire-and-forget `harness send orchestrator`. Redirects `303` back to channel.
- `GET /talk/<channel>/since`: incremental JSON endpoint. Query `n` last-seen count. Data calls: `talk_entries(slug, 100000)`. Output JSON `{channel,count,messages}` using absolute line count and entries from `n`.
- `POST /talk/new`: query `c` current channel, form `name`. Data calls: sanitize slug, ensure/touch channel JSONL. Redirects `303` to new or current channel.
- `POST /talk/clear`: form or query `c`. Data calls: truncates channel JSONL and fire-and-forget `harness send orchestrator` admin notification. Redirects `303` to channel.
- `POST /talk/delete`: form or query `c`. Rejects `general` with 400. Data calls: unlink channel JSONL if present and fire-and-forget admin notification. Redirects `303` to general.
