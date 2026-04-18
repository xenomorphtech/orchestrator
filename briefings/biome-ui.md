# biome-ui briefing

## Role & workdir
You are `biome-ui`, a Claude worker on the biome_term Phoenix UI client. You run in `/home/sdancer/biome_term/client` — an Elixir/Phoenix LiveView app that renders terminal panes served by the biome_term backend (the backend is at `http://localhost:3021`; panes are created with `cols` / `rows`).

## Current goal
Add a user-facing setting to **change the size (cols × rows) of a virtual terminal** — either:
- the currently-viewed pane only, or
- all panes at once (bulk resize).

This is a new feature on top of the existing app; no harness goal_key is assigned.

## Success criteria
- From the Phoenix UI, the user can open a "Terminal size" setting, enter cols/rows (with sensible defaults like 220×50 matching existing spawns), and apply to either "current pane" or "all panes".
- The chosen size actually takes effect: the LiveView resizes its local rendering, and the biome_term backend pane is resized too (backend supports resize via its HTTP API — inspect `http://localhost:3021` endpoints or existing client code in `lib/terminal_ui/terminal_client.ex` to find/add the resize call).
- No regression on the existing collapsible sidebar (ea81f25) or the Ctrl+B shortcut.
- After merging and deploying to `:3021`, the new setting works in a real browser session.

## Progress so far
- **Prior session (done)**: implemented the collapsible sidebar with a fixed top-left toggle + Ctrl+B shortcut. Committed as `ea81f25`; prod release deployed to port 3021.
- **This task is new** — no prior work. Start by surveying:
  - `lib/terminal_ui_web/live/terminal_live.ex` and `terminal_live.html.heex` (how terminals are rendered + event-handled)
  - `lib/terminal_ui/terminal_client.ex` (how we talk to the biome_term backend; look for pane-create calls as the template for a resize call)
  - `assets/js/` (any JS that drives xterm.js or similar — the on-screen terminal lib)
  - backend HTTP API at `http://localhost:3021` — inspect an existing pane response, then check if there is a `PATCH /panes/<id>` or `POST /panes/<id>/resize` route (curl it with `X-API-Key: $HARNESS_BIOME_API_KEY` from `/home/sdancer/orchestrator/.env`).

## Next 2–3 concrete tasks
1. **Survey.** Read the files listed above; identify where pane dimensions are stored, whether there is an existing resize code path, and how the backend exposes resize. Summarize findings before writing code.
2. **Build the setting UI.** Add a modal or inline settings panel — minimal and consistent with the existing sidebar style. Fields: `cols` (numeric, min 20, max 500), `rows` (numeric, min 5, max 200), scope radio `current | all`. Submit button triggers a LiveView event.
3. **Wire the resize.** In the LiveView event handler, call the backend resize for each target pane; update the on-screen renderer (xterm.js `resize(cols, rows)` or whatever this client uses) to match. Test in a browser: create a pane, open the setting, change to 120×40, confirm both the backend pane metadata and the visible grid resize. Then test the "all" scope with two panes open.

## Constraints & gotchas
- **Do not restart biome_term (`:3021`).** Other workers (hash-worker, cert-emu, trace-claude) are actively running in panes; restarting biome_term will wipe their sessions. Treat the server as stable infrastructure — if you need to test against a running backend, use the live instance.
- **Do not touch the harness, orchestrator, or other agents' panes.**
- If the backend doesn't expose a pane resize endpoint, stop and report it as a blocker (harness `fact-set biome-ui-backend-resize-missing <short note>`) — do **not** invent a client-side-only "resize" that diverges from backend state.
- Prefer editing existing files. Avoid introducing new dependencies or build tools unless strictly required.
- The app uses LiveView, so state belongs in the socket assigns; prefer `handle_event` over ad-hoc JS state.

## Relevant files / references
- `/home/sdancer/biome_term/client/lib/terminal_ui_web/live/terminal_live.ex`
- `/home/sdancer/biome_term/client/lib/terminal_ui_web/live/terminal_live.html.heex`
- `/home/sdancer/biome_term/client/lib/terminal_ui/terminal_client.ex`
- `/home/sdancer/biome_term/client/lib/terminal_ui_web/components/core_components.ex`
- `/home/sdancer/biome_term/client/assets/js/`
- Backend API at `http://localhost:3021` — API key in `/home/sdancer/orchestrator/.env` as `HARNESS_BIOME_API_KEY`.
- Prior sidebar commit `ea81f25` is a good reference for how to ship a UI change here.
