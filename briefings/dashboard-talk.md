# dashboard-talk — add a chat panel to the orchestrator web UI

## Role & workdir
Codex worker, workdir `/home/sdancer/orchestrator`. **Important**: the file you'll edit (`web/app.py`) is **UNTRACKED** in this repo — do NOT create a git worktree, do NOT branch. Edit the live file in place; no commit needed.

## Current goal / sub-goal
- **goal_key**: `dashboard_talk_panel`
- **sub_goal_key**: `talk-route-in-web-app-py`

## User request (verbatim)
> "add to our http://term.orch.run:3030/ dashboard a place to talk, where text turns/answers are replied to there"

Interpretation: a two-way chat panel on the dashboard where (a) the user can post a message and (b) the orchestrator's cycle outputs / replies appear inline. Each "turn" is one message; replies thread underneath.

## Success criteria
- New page `/talk` reachable from the top nav of `http://term.orch.run:3030/`
- POST form accepts user text → appends to a JSONL store
- GET renders the full chat thread chronologically (oldest first, newest at the bottom near the input box) with sender, timestamp, and the text body
- The orchestrator can append its own replies to the same JSONL store and they render in-line
- Page auto-refreshes (meta-refresh 10s is fine — match existing SSR pattern)
- Dashboard process restarted with the new route live; `curl -s http://127.0.0.1:3030/talk` returns 200 with the expected form
- Done = visible at `http://term.orch.run:3030/talk` AND the nav bar shows a "talk" link.

## Concrete tasks (do in order)

1. **Read the existing app** — `/home/sdancer/orchestrator/web/app.py` (481 lines, Flask SSR, Tailwind CDN). Note the `BASE` template, the `render(title, body, refresh)` helper, and the route style (each route returns `render(...)`). The nav bar is hardcoded in `BASE`.

2. **Add the JSONL store helper.** Use `/home/sdancer/orchestrator/analysis/talk.jsonl` (alongside `paths.json`, `falsified.md`, etc.). Each line: `{"ts": "<iso8601>", "from": "user"|"orchestrator"|"<worker-name>", "text": "<the message>", "reply_to": "<ts of message being replied to, optional>"}`. Append-only. Create the file if missing.

3. **Add the `/talk` route.**
   - `GET /talk` — read the JSONL (last N=200 entries to bound load), render newest-at-bottom thread + a POST form.
     - Each turn: small left-margin badge for sender (color-coded: user=sky-500, orchestrator=emerald-500, worker=amber-500), timestamp in zinc-500 text-xs, text in pre-wrap.
     - Reply threading: if `reply_to` is set, indent that turn slightly (left padding).
     - Auto-refresh: `refresh=10` (10-second meta-refresh) so orchestrator replies surface promptly.
     - The POST form sits at the bottom: `<form method=post action=/talk>` with a `<textarea name=text rows=3>` and a submit button. Tailwind classes to match the existing dark theme.
   - `POST /talk` — read `request.form['text']`, validate non-empty, append a line to talk.jsonl with `ts=now_iso, from="user", text=<the text>`, redirect to `/talk` (302).
   - For Flask form handling: add `from flask import request, redirect, url_for` to the imports.

4. **Update the nav bar.** In the `BASE` template constant, add `<a href="/talk" class="hover:text-white">talk</a>` between "briefings" and the timestamp span. Keep the visual flow consistent.

5. **Restart the dashboard process.** The dashboard runs as a bare `python3 app.py` (PID 2229097, no systemd unit). To restart:
   ```bash
   # SIGTERM the running process, then re-launch detached.
   pkill -f '/home/sdancer/orchestrator/web/app.py' || true
   sleep 1
   cd /home/sdancer/orchestrator && nohup python3 web/app.py >/tmp/orchestrator_dash.log 2>&1 &
   sleep 2
   curl -sS -o /dev/null -w 'HTTP %%{http_code}\n' http://127.0.0.1:3030/talk
   ```
   Expect HTTP 200 on the smoke test. Tail `/tmp/orchestrator_dash.log` if anything looks wrong.

6. **Quick interactive smoke test.** From the workdir:
   ```bash
   curl -sS -X POST http://127.0.0.1:3030/talk -d 'text=hello%20from%20smoke%20test' -L | grep -c 'hello from smoke test'
   ```
   Expect ≥1 (the message appears on the rendered page).

7. **Document.** Write a short verdict at `/home/sdancer/orchestrator/analysis/dashboard_talk_verdict.md` (≤30 lines) describing the schema of talk.jsonl, the route map, and how the orchestrator should pick up unreplied user messages on its next `/orchestrate` tick (read jsonl, find latest user msg with `from=="user"`, decide whether to reply via `fact-set` + an append helper). **Final line `DASHBOARD_TALK_DONE`.**

8. **Set fact** `dashboard_talk_panel_live_2026_05_18 = true` via `/home/sdancer/orchestrator/harness fact-set ...`.

## Constraints & gotchas

- **Untracked file.** `web/` is not in git. Do NOT `git worktree add` — there's nothing in the index to checkout. Edit `web/app.py` directly. No commit step.
- **No new dependencies.** Stick to Flask + stdlib. The existing app uses Tailwind CDN — don't add a JS framework. SSR + meta-refresh is the house style.
- **Bound the JSONL read.** Last 200 entries is plenty; reading the entire file every render would degrade later. Use `collections.deque(open(...), maxlen=200)` or seek-to-end then read backward.
- **Sanitize user input.** Use Jinja's autoescape (Flask default in render_template_string) — DO NOT bypass with `|safe` on user text. Only `body|safe` for orchestrator-rendered HTML is allowed, per the existing pattern.
- **Don't break the existing `body|safe` pattern.** Compose the talk page body as an HTML string then pass to `render(...)`. For user text inside that HTML, do explicit escaping: `from markupsafe import escape; escape(user_text)`.
- **Auto-refresh trade-off.** The existing dashboard refreshes every 30s; for chat, 10s is acceptable. Keep meta-refresh — do NOT add JS/WebSocket. SSR is the constraint.
- **Atomic append.** Use `open(path, 'a')` with a single `f.write(line + "\n")` — appends on POSIX are atomic for small payloads.

## Relevant files / references

- `/home/sdancer/orchestrator/web/app.py` — the Flask app (481 lines, single file)
- `/home/sdancer/orchestrator/analysis/talk.jsonl` — to be created
- `/home/sdancer/orchestrator/analysis/dashboard_talk_verdict.md` — write your verdict here
- Existing pattern reference: routes `/cycles`, `/goals`, `/facts` in `web/app.py` show the `body[].append(...)` + `render(title, body, refresh)` style
- Dashboard URL: `http://term.orch.run:3030/` (or `http://127.0.0.1:3030/` from this host)
