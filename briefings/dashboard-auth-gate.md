# dashboard-auth-gate — password-protect /talk + commit/push dashboard

## Role & workdir
Codex worker, workdir `/home/sdancer/orchestrator`. Editing the running Flask app `web/app.py` (currently untracked — this task includes committing it to the repo).

## Current goal / sub-goal
- **goal_key**: `dashboard_talk_panel`
- **sub_goal_key**: `auth-gate-with-persistent-cookie`

## User request (verbatim)
> "protect this dashboard with a password, set as defautl 'welcome to my w0rld'
> commit and push the dashboard stuff into the orchestrator repo
> ensure the pass is saved as a hashed/salted cookie so don't need to relogin every time"

## Success criteria
- All `/` and `/talk*` routes (and any other interactive routes) require a password.
- `GET /login` shows a single-field password form; `POST /login` validates and sets a persistent signed cookie.
- Default password: `welcome to my w0rld` (configurable via env var `DASHBOARD_PASSWORD`; fall back to default if unset).
- Password stored hashed + salted at REST. Validate by recomputing hash, NOT plain-text compare.
- Auth cookie is a signed token (use Flask's `itsdangerous`/`flask.session` or `URLSafeTimedSerializer`). Cookie lifetime ≥30 days so user does not re-login on browser restart.
- Cookie value contains the *hash of the password*, not the password itself; cookie is also `HttpOnly`, `SameSite=Lax`.
- `web/app.py` committed AND pushed to the orchestrator git remote.
- `orchestrator-dash.service` restarted with new code live; auth gate verified via curl smoke (302 redirect to /login when no cookie; 200 once cookie set).
- Set fact `dashboard_auth_gate_live_2026_05_18=true` with auth-mode summary.
- Verdict at `analysis/dashboard_auth_gate_verdict.md`. Final line `DASHBOARD_AUTH_GATE_DONE`.

## Concrete tasks (do in order)

1. **Read `/home/sdancer/orchestrator/web/app.py`**. Note the existing route surface (`/`, `/talk`, `/talk/new`, `/talk/clear`, `/talk/delete`, etc.) and the `notify_orchestrator_pane()` helper.

2. **Add auth machinery**:
   - Module-level constants: `DASHBOARD_DEFAULT_PASSWORD = "welcome to my w0rld"`, `DASHBOARD_PASSWORD = os.environ.get("DASHBOARD_PASSWORD", DASHBOARD_DEFAULT_PASSWORD)`.
   - Compute `PASSWORD_HASH = hashlib.sha256(("dashboard-salt-v1:" + DASHBOARD_PASSWORD).encode()).hexdigest()` at startup. (Salt is a literal prefix; combined with hash, this is a deterministic-per-password value safe to store in cookies + compare.)
   - `app.secret_key` — generate persistent key by reading/writing `/home/sdancer/orchestrator/.dash_secret_key` (random 32 bytes on first run, mode 600). NEVER hard-code.
   - Helper `is_authed(request) -> bool`: reads cookie `dash_auth`, validates equals current `PASSWORD_HASH`.
   - `before_request` handler: if request.path starts with `/login` or `/static`, allow; else if not `is_authed(request)`, redirect 302 to `/login`.

3. **Routes**:
   - `GET /login` — render a minimal HTML form with single `<input type=password name=password>`. Use same dark-zinc Tailwind style as the rest of the dashboard.
   - `POST /login` — read `request.form.get('password', '')`; if `hashlib.sha256(("dashboard-salt-v1:" + form_pw).encode()).hexdigest() == PASSWORD_HASH`, set cookie `dash_auth=<PASSWORD_HASH>` with `max_age=30*24*3600`, `httponly=True`, `samesite='Lax'`, `secure=False` (the tunnel terminates TLS upstream — don't break local dev), then redirect 303 to `/talk?c=general`. Else re-render login with `error="Wrong password"`.
   - `GET /logout` — clear cookie + redirect to `/login`.

4. **Restart via systemd**:
   ```bash
   sudo systemctl restart orchestrator-dash.service
   systemctl is-active orchestrator-dash.service
   ```

5. **Smoke**:
   ```bash
   # without cookie → redirect to /login
   curl -sS -o /dev/null -w 'GET /talk → %{http_code}\n' http://127.0.0.1:3030/talk     # expect 302
   curl -sS -o /dev/null -w 'GET /login → %{http_code}\n' http://127.0.0.1:3030/login   # expect 200
   # login with correct pass → 303, cookie set
   curl -sS -c /tmp/dash_cookies.txt -o /dev/null -w 'POST /login → %{http_code}\n' \
     -X POST -d 'password=welcome to my w0rld' http://127.0.0.1:3030/login              # expect 303
   grep -c dash_auth /tmp/dash_cookies.txt                                              # expect 1
   # subsequent GET /talk with cookie → 200
   curl -sS -b /tmp/dash_cookies.txt -o /dev/null -w 'GET /talk (cookied) → %{http_code}\n' http://127.0.0.1:3030/talk  # expect 200
   # wrong password → re-renders login (200) with error
   curl -sS -o /dev/null -w 'POST /login wrong → %{http_code}\n' -X POST -d 'password=nope' http://127.0.0.1:3030/login  # expect 200
   ```

6. **Commit + push**:
   ```bash
   cd /home/sdancer/orchestrator
   git status
   git add web/app.py briefings/dashboard-auth-gate.md analysis/dashboard_auth_gate_verdict.md
   # also add the prior dashboard verdicts if untracked — preserve their content as the source of truth
   git add analysis/dashboard_talk_*verdict.md 2>/dev/null || true
   git commit -m "$(cat <<'EOF'
   feat(dashboard): password gate + hashed cookie + commit web app

   - GET/POST /login + GET /logout
   - Password hashed sha256(salt-v1 + pw) at startup
   - Persistent signed cookie dash_auth (30-day lifetime, HttpOnly, SameSite=Lax)
   - Default password configurable via DASHBOARD_PASSWORD env var
   - Vendor previously-untracked web/app.py into repo
   EOF
   )"
   git push  # default remote/branch
   ```
   If `git push` requires auth/permission, **report blocker** — don't bypass; orchestrator will surface to user.

7. **Set fact + verdict**:
   - `harness fact-set dashboard_auth_gate_live_2026_05_18 "password gate + hashed cookie shipped, web/app.py vendored, commit <hash> pushed"`
   - `analysis/dashboard_auth_gate_verdict.md` (≤80 lines, must end with `DASHBOARD_AUTH_GATE_DONE`).

## Constraints & gotchas
- **`web/app.py` is currently untracked** — this task VENDORS it (first commit of the file). Inspect carefully before adding so secrets/keys are not added inadvertently. Specifically, the `.dash_secret_key` file from Task 2 MUST be `.gitignore`d.
- **Add `.dash_secret_key` to `.gitignore`** before any `git add`.
- **DO NOT commit the password** in plain text inside the Python source — the default is a string constant; if the user wants this hidden later they can override via env var. The salt is a literal prefix (`"dashboard-salt-v1:"`) and is fine to ship.
- **Restart only via systemd** — `sudo systemctl restart orchestrator-dash.service`. Never `nohup` (per memory `worker-artifact-isolation`).
- **No new pip dependencies.** Use `hashlib`, `secrets`, `os` from stdlib; Flask already provides cookies via `make_response(...).set_cookie(...)`.
- **Don't break `notify_orchestrator_pane()`** — auth-gate POSTs from already-authed sessions must continue firing notifications.
- **The `before_request` allowlist** must include `/login` (and `/static/*` if any) — otherwise login form itself becomes inaccessible.
- **303 not 302** for POST /login redirect.
- **Cookie should be set on the `/login` POST response, not on subsequent requests.** Validate by `c -c` storing then `-b` reading.

## Relevant files / references
- `/home/sdancer/orchestrator/web/app.py` — Flask app (untracked, to be vendored)
- `/etc/systemd/system/orchestrator-dash.service` — durable systemd unit
- `/home/sdancer/orchestrator/analysis/talk_channels/` — chat store (no schema changes)
- Memory: `[[worker-artifact-isolation]]`, `[[feedback-orchestrator-role]]`
- Prior verdicts: `analysis/dashboard_talk_channel_controls_verdict.md`, `analysis/dashboard_talk_channels_verdict.md`
