# dashboard-auth-gate verdict

- Date: 2026-05-18
- Scope: `web/app.py`, `.gitignore`, repo vendoring of the dashboard app, live service restart
- Auth mode: `/login` password form plus signed `dash_auth` cookie carrying the salted SHA-256 password hash

Implemented:
- Default password constant with `DASHBOARD_PASSWORD` env override
- Startup password hash via `sha256("dashboard-salt-v1:" + password)`
- Persistent secret key in `/home/sdancer/orchestrator/.dash_secret_key` with mode `0600`
- `before_request` gate protecting dashboard and talk routes, allowing `/login` and `/static`
- `GET/POST /login` and `GET /logout`
- Cookie flags: `Max-Age=2592000`, `HttpOnly`, `Path=/`, `SameSite=Lax`, `secure=False`
- `.dash_secret_key` added to `.gitignore`

Verified locally with Flask test client:
- `GET /talk` unauthenticated returns `302 /login`
- `GET /login` returns `200`
- `POST /login` with `welcome to my w0rld` returns `303` and sets `dash_auth`
- Authenticated `GET /talk` returns `200`
- Wrong password re-renders login with `200` and error text
- Signed cookie payload validates to the current `PASSWORD_HASH`

Verified live after `sudo systemctl restart orchestrator-dash.service`:
- `systemctl is-active orchestrator-dash.service` => `active`
- `GET /talk` => `302`
- `GET /login` => `200`
- `POST /login` => `303`
- cookie jar contains one `dash_auth`
- authenticated `GET /talk` => `200`
- wrong-password `POST /login` => `200`
- live `Set-Cookie` header includes `HttpOnly` and `SameSite=Lax`

DASHBOARD_AUTH_GATE_DONE
