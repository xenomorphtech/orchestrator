# Rust Dashboard Route Inventory

Source read in full: `dashboard/src/main.rs`; shared data access lives in `dashboard/src/data.rs`.

## Global Auth And Shared Data

- Auth gate: every protected Axum handler redirects to `/login` unless `dash_auth` validates against the local dashboard secret key and equals `sha256("dashboard-salt-v1:" + DASHBOARD_PASSWORD)`.
- Password: `DASHBOARD_PASSWORD` env or default `welcome to my w0rld`; cookie max age 30 days, httponly, `SameSite=Lax`, not secure.
- Secret key: `/home/sdancer/orchestrator/.dash_secret_key`, created `0600` if missing.
- Harness CLI: `/home/sdancer/orchestrator/harness`; table parser splits pipe-delimited headers/rows, skips separator lines and `(N rows)`.
- SQL: POST text SQL to `{HARNESS_SERVER}/v1/database/{HARNESS_DATABASE}/sql`; env overrides, then `/home/sdancer/orchestrator/harness.toml`, then defaults.
- Files: `/home/sdancer/orchestrator/briefings/*.md`, `/home/sdancer/.claude/projects/-home-sdancer-orchestrator/memory/*.md`, uploaded artifact bytes under `/home/sdancer/orchestrator/analysis/dashboard_uploads/`. Talk conversations/messages, artifact index rows, and path portfolio data come from the harness DB.
- Base template: Rust `render_page`; Tailwind CDN, nav links, optional meta refresh, body HTML assembled by handlers.

## Routes

- `GET /login`: if authed redirects to `/talk?c=general`; otherwise renders login form. Data calls: cookie validation only.
- `POST /login`: form `password`; on match sets `dash_auth` cookie and redirects to `/talk?c=general`; on mismatch renders login form with error. Data calls: password hash + secret signer.
- `GET /logout`: deletes `dash_auth` cookie and redirects to `/login`. Data calls: none.
- `GET /`: dashboard page with refresh 30. Data calls: `episodes(8)` via SQL newest-id search with CLI fallback, `goals()` via `harness goals`, `paths_portfolio()` from the harness DB `paths` table, `services()` via `harness services`. Template sections: active campaigns, path portfolio summary, recent cycles, services.
- `GET /cycles`: latest 200 cycles. Data calls: `episodes(200)` via SQL with CLI fallback. Template: table id/when/summary/frontier/stall; id and summary link to `/cycle/<id>`.
- `GET /cycle/<cid>`: cycle detail, `cid` integer path. Data calls: SQL by id with fallback to `episodes(500)`. Template: full cycle description plus JSON pre blocks for agents/actions/goal progress. Returns 404 if missing.
- `GET /goals`: goals list. Data calls: `harness goals`, sorted active first then priority descending. Template: status badge, priority, key link, title.
- `GET /goaltree`: goal forest dashboard shell. Data calls: `harness goal-tree --json`; renders `goal_tree.v2` roots recursively with nested child goals/sub-goals. Sub-goal nesting comes from `metadata.parent_sub_goal_key`; dependency fields are displayed as dependency metadata, not visual containment. Workstreams are not part of the goal-tree contract.
- `GET /goal/<key>`: goal detail. Data calls: `harness goals`, exact `goal_key` match. Template: badge, priority, title, optional detail, success/completion/created/updated grid. Returns 404 if missing.
- `GET /paths`: path portfolio. Data calls: harness DB `paths` table plus goal/anchor metadata. Template: one section per goal with metric/current/target/last move and per-path table.
- `GET /agents`: agents list. Data calls: `harness agents`. Template: table name/kind/workdir/description or empty notice.
- `GET /facts`: latest 80 facts. Data calls: `harness facts`, reversed and truncated. Template: table created/key/value.
- `GET /services`: services list. Data calls: `harness services`. Template: table name/type/status/last poll/target.
- `GET /memory`: memory index. Data calls: `list_md(MEMORY)` and `read_md(MEMORY, "MEMORY.md")`. Template: optional open MEMORY.md details block and file list sorted by mtime descending.
- `GET /memory/<name>`: memory detail. Data calls: guarded markdown read from `MEMORY`; rejects `/` and `..`. Template: raw markdown in pre plus back link. Returns 404 if missing.
- `GET /briefings`: briefing index. Query input `show=active|archived|all` default active. Data calls: `harness briefing-list`, `briefing-list --only-archived`, or `briefing-list --archived`; normalized `(none)` cells. Template: tabs, grouped by category, table name/goal/tags/updated with archived chip.
- `GET /briefings/<name>`: briefing detail. Path may end in `.md`; base name used. Data calls: `harness briefing-list --archived`, `harness briefing-get <base>`, fallback to `/home/sdancer/orchestrator/briefings/<base>.md`. Template: metadata chips plus escaped markdown in pre. Returns 404 if missing.
- `GET /artifacts`: artifact upload/list page. Data calls: SQL `artifacts`; upload form posts multipart files with optional context text; list links downloads by SHA-256.
- `POST /artifacts/upload`: multipart `file`, optional multipart `context`, optional query `c` talk slug. Stores bytes under `analysis/dashboard_uploads/`, records `artifact_upsert` with context in `metadata_json`, optionally appends a system talk message and nudges the talk worker.
- `POST /artifacts/context`: form `sha256`, `context`, optional `c` talk slug. Resolves indexed artifact by content hash and updates its `metadata_json.context` via `artifact_upsert`.
- `GET /artifacts/raw/<sha256>`: validates SHA-256, resolves indexed artifact path, ensures path is under `analysis/dashboard_uploads/`, and downloads bytes.
- `GET /talk`: talk UI for query `c` conversation slug, default `general`. Data calls: DB-backed `facts` rows with source types `dashboard-talk-conversation` / `dashboard-talk-message`, plus recent `artifacts`. Template: conversation sidebar, context-aware create form, clear/archive forms, messages, post form, artifact upload/list, polling JavaScript. No meta refresh.
- `POST /talk`: form `from`, `text`; query `c` conversation. Data calls: `fact_set` for message storage; if sender is `user`, registers/sends the per-conversation `codex_app_server` worker with a 3600-second timeout and appends the worker's final assistant text when the turn completes. Redirects `303` back to conversation.
- `GET /talk/<channel>/since`: incremental JSON endpoint. Query `n` last-seen count. Data calls: DB-backed `talk_entries(slug, 100000)`. Output JSON `{channel,count,messages}` using absolute rendered count and entries from `n`.
- `POST /talk/new`: query `c` current conversation, form `name`, optional `goal`, optional `context`. Data calls: `fact_set` for conversation metadata, `agent-add --kind codex_app_server`, background `harness send` with context packet. Redirects `303` to new or current conversation.
- `POST /talk/clear`: form or query `c`. Data calls: `fact_set` to advance conversation `clear_after`. Redirects `303` to conversation.
- `POST /talk/delete`: form or query `c`. Rejects `general` with 400. Data calls: `fact_set` to archive conversation metadata. Redirects `303` to general.
- `GET /api/cycles`: JSON latest cycles.
- `GET /api/goals`: JSON goals.
- `GET /api/goal/<key>`: JSON goal detail.
- `GET /api/paths`: JSON path portfolio from the DB `paths` table.
- `GET /api/agents`: JSON agents.
- `GET /api/facts`: JSON latest facts.
- `GET /api/services`: JSON services.
- `GET /api/memory`: JSON memory index.
- `GET /api/briefings`: JSON briefing index.
- `GET /api/adherence`: JSON path adherence signals.
- `GET /api/progress`: JSON aggregate progress.
- `GET /api/goaltree/<goal_key>`: JSON `goal_tree.v2` single-root forest for the requested goal. Shape: `{schema:"goal_tree.v2", roots:[{type:"goal", children:[...]}], facts:[...]}`.
- `POST /api/goaltree/<goal_key>/tick`: append a goal-tree tick.
