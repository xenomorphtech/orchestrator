#!/usr/bin/env python3
"""Orchestrator web dashboard — campaigns, cycles, memory, briefings.

Serves a read-only view over harness CLI output, analysis/* files, and the
operator's auto-memory tree. Pure SSR, Tailwind CDN, no JS framework.

Run:  python3 /home/sdancer/orchestrator/web/app.py  (binds 127.0.0.1:3030)
"""
from __future__ import annotations
import hashlib, json, os, re, secrets, subprocess, urllib.request
from collections import deque
from pathlib import Path
from datetime import datetime
from flask import Flask, abort, jsonify, make_response, redirect, render_template_string, request, url_for
from itsdangerous import BadSignature, URLSafeTimedSerializer
from markupsafe import escape

HARNESS = "/home/sdancer/orchestrator/harness"
# Direct SpacetimeDB SQL endpoint (same server/db the harness CLI talks to). Used to
# fetch the *newest* episodes: the harness `episodes --limit N` runs `... LIMIT N` with
# no ORDER BY, and SpacetimeDB returns rows in ascending rowid order, so the CLI yields
# the OLDEST N. SpacetimeDB v2.1 supports neither ORDER BY nor MAX(), so we binary-search
# the max id via `WHERE id >= X LIMIT 1` probes, then fetch a bounded id-window.
HARNESS_SERVER = os.environ.get("HARNESS_SERVER", "http://127.0.0.1:3000")
HARNESS_DATABASE = os.environ.get("HARNESS_DATABASE", "orchestrator-harness")
SQL_URL = f"{HARNESS_SERVER}/v1/database/{HARNESS_DATABASE}/sql"
ANALYSIS = Path("/home/sdancer/orchestrator/analysis")
BRIEFINGS = Path("/home/sdancer/orchestrator/briefings")
MEMORY = Path("/home/sdancer/.claude/projects/-home-sdancer-orchestrator/memory")
TALK_LOG = ANALYSIS / "talk.jsonl"
TALK_CHANNELS = ANALYSIS / "talk_channels"
GENERAL_TALK_CHANNEL = "general"
CHANNEL_SLUG_RE = re.compile(r"^[a-z0-9-]{1,32}$")
DASHBOARD_DEFAULT_PASSWORD = "welcome to my w0rld"
DASHBOARD_PASSWORD = os.environ.get("DASHBOARD_PASSWORD", DASHBOARD_DEFAULT_PASSWORD)
PASSWORD_SALT = "dashboard-salt-v1:"
SECRET_KEY_PATH = Path("/home/sdancer/orchestrator/.dash_secret_key")
AUTH_COOKIE_NAME = "dash_auth"
AUTH_COOKIE_MAX_AGE = 30 * 24 * 3600


def hash_password(password: str) -> str:
    return hashlib.sha256(f"{PASSWORD_SALT}{password}".encode()).hexdigest()


PASSWORD_HASH = hash_password(DASHBOARD_PASSWORD)


def load_or_create_secret_key(path: Path = SECRET_KEY_PATH) -> str:
    if path.exists():
        key = path.read_text().strip()
        if not key:
            raise RuntimeError(f"{path} is empty")
        path.chmod(0o600)
        return key
    key = secrets.token_hex(32)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as handle:
        handle.write(key)
    path.chmod(0o600)
    return key

app = Flask(__name__)
app.secret_key = load_or_create_secret_key()
AUTH_SERIALIZER = URLSafeTimedSerializer(app.secret_key, salt="dashboard-auth-cookie-v1")


def run(*args, timeout: int = 15) -> str:
    try:
        r = subprocess.run([HARNESS, *args], capture_output=True, text=True, timeout=timeout)
        return r.stdout
    except Exception as e:
        return f"(error: {e})"


def parse_table(text: str) -> list[dict]:
    """Parse the harness CLI's pipe-delimited table format."""
    lines = [ln for ln in text.splitlines() if ln.strip()]
    if not lines:
        return []
    header_line = lines[0]
    if "+" in header_line or set(header_line.strip()) <= {"-", "+"}:
        return []
    cols = [c.strip() for c in header_line.split("|")]
    rows = []
    for ln in lines[1:]:
        if set(ln.strip()) <= {"-", "+"}:
            continue
        # tail like "(N rows)" — skip
        if re.match(r"^\(\d+ rows?\)$", ln.strip()):
            continue
        parts = [p.strip() for p in ln.split("|")]
        if len(parts) != len(cols):
            continue
        rows.append(dict(zip(cols, parts)))
    return rows


def parse_json_cell(s: str):
    if not s or s == "(none)" or s == "{}":
        return None
    try:
        return json.loads(s)
    except Exception:
        return s


# ─── data accessors (cached briefly per-request) ────────────────────────────

def sql_query(query: str, timeout: int = 10) -> list[dict]:
    """Run read-only SQL against the harness SpacetimeDB and return list[dict]."""
    req = urllib.request.Request(
        SQL_URL, data=query.encode(), method="POST",
        headers={"Content-Type": "text/plain"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        payload = json.loads(resp.read().decode())
    if not payload:
        return []
    result = payload[0]
    cols = [
        (e["name"]["some"] if isinstance(e.get("name"), dict) else e["name"])
        for e in result["schema"]["elements"]
    ]
    return [dict(zip(cols, row)) for row in result["rows"]]


_MAX_EPISODE_ID = {"id": 0}


def _episode_id_exists_from(x: int) -> bool:
    return bool(sql_query(f"SELECT id FROM episodes WHERE id >= {x} LIMIT 1"))


def max_episode_id() -> int:
    """Largest episode id present. Cached; cheap (≈1 probe) in steady state."""
    cached = _MAX_EPISODE_ID["id"]
    if cached and not _episode_id_exists_from(cached + 1):
        return cached
    lo, hi = max(cached, 1), max(cached * 2, 2)
    while _episode_id_exists_from(hi):  # exponential upper bound
        lo, hi = hi, hi * 2
    while lo < hi:                      # binary-search largest present id
        mid = (lo + hi + 1) // 2
        if _episode_id_exists_from(mid):
            lo = mid
        else:
            hi = mid - 1
    _MAX_EPISODE_ID["id"] = lo
    return lo


def episodes(limit: int = 50):
    # newest `limit` episodes, newest first. See SQL_URL note for why the CLI can't.
    try:
        top = max_episode_id()
        if top <= 0:
            return []
        window = limit * 5 + 100  # generous for id gaps (deleted episodes)
        rows = sql_query(
            "SELECT id, created_at, summary, agent_statuses_json, "
            "actions_taken_json, goal_progress_json FROM episodes "
            f"WHERE id > {top - window}"
        )
        rows.sort(key=lambda r: int(r["id"]))   # ascending by id
        rows = rows[-limit:]                     # newest `limit` (still ascending)
        rows.reverse()                           # newest first
    except Exception:
        # fall back to the CLI (oldest-N, reversed) so the page still renders
        rows = parse_table(run("episodes", "--limit", str(limit)))
        rows.reverse()
    for r in rows:
        for k in ("agent_statuses_json", "actions_taken_json", "goal_progress_json"):
            r[k] = parse_json_cell(r.get(k, "")) or {}
    return rows


def goals():
    rows = parse_table(run("goals"))
    rows.sort(key=lambda r: (r.get("status") != "active", -int(r.get("priority", "0") or 0)))
    return rows


def agents():
    rows = parse_table(run("agents"))
    return rows


def facts(limit: int = 80):
    rows = parse_table(run("facts"))
    rows.reverse()
    return rows[:limit]


def services():
    rows = parse_table(run("services"))
    return rows


def paths_portfolio():
    p = ANALYSIS / "paths.json"
    if not p.exists():
        return None
    try:
        return json.loads(p.read_text())
    except Exception:
        return None


def list_md(dir_: Path) -> list[dict]:
    if not dir_.is_dir():
        return []
    out = []
    for f in sorted(dir_.glob("*.md")):
        try:
            st = f.stat()
            head = f.read_text(errors="replace").splitlines()
            title = next((ln.lstrip("# ").strip() for ln in head if ln.strip()), f.name)
            out.append({
                "name": f.name,
                "title": title[:120],
                "size": st.st_size,
                "mtime": datetime.fromtimestamp(st.st_mtime),
            })
        except Exception:
            continue
    out.sort(key=lambda x: x["mtime"], reverse=True)
    return out


def read_md(dir_: Path, name: str) -> str | None:
    if "/" in name or ".." in name:
        return None
    p = dir_ / name
    if not p.exists() or not p.is_file():
        return None
    return p.read_text(errors="replace")


def now_iso() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def signed_auth_value() -> str:
    return AUTH_SERIALIZER.dumps(PASSWORD_HASH)


def is_authed(req) -> bool:
    token = req.cookies.get(AUTH_COOKIE_NAME, "")
    if not token:
        return False
    try:
        return AUTH_SERIALIZER.loads(token, max_age=AUTH_COOKIE_MAX_AGE) == PASSWORD_HASH
    except BadSignature:
        return False


def sanitize_channel_slug(name: str | None) -> str | None:
    raw = (name or "").strip().lower().encode("ascii", "ignore").decode("ascii")
    if not raw:
        return None
    slug = re.sub(r"[^a-z0-9]+", "-", raw)
    slug = re.sub(r"-{2,}", "-", slug).strip("-")
    slug = slug[:32].strip("-")
    if not slug or not CHANNEL_SLUG_RE.fullmatch(slug):
        return None
    return slug


def talk_channel_path(channel: str) -> Path:
    slug = sanitize_channel_slug(channel) or GENERAL_TALK_CHANNEL
    return TALK_CHANNELS / f"{slug}.jsonl"


def ensure_talk_channels() -> None:
    TALK_CHANNELS.mkdir(parents=True, exist_ok=True)
    general_path = talk_channel_path(GENERAL_TALK_CHANNEL)
    if general_path.exists():
        return
    if TALK_LOG.exists():
        try:
            general_path.write_text(TALK_LOG.read_text(errors="replace"))
            return
        except Exception:
            pass
    general_path.touch(exist_ok=True)


def list_talk_channels() -> list[str]:
    ensure_talk_channels()
    channels = {GENERAL_TALK_CHANNEL}
    for path in TALK_CHANNELS.glob("*.jsonl"):
        slug = sanitize_channel_slug(path.stem)
        if slug:
            channels.add(slug)
    return [GENERAL_TALK_CHANNEL] + sorted(ch for ch in channels if ch != GENERAL_TALK_CHANNEL)


def talk_entries(channel: str = GENERAL_TALK_CHANNEL, limit: int = 200) -> list[dict]:
    ensure_talk_channels()
    talk_path = talk_channel_path(channel)
    talk_path.touch(exist_ok=True)
    rows = deque(maxlen=limit)
    try:
        with talk_path.open(errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except Exception:
                    continue
                if not isinstance(row, dict):
                    continue
                rows.append(row)
    except Exception:
        return []
    return list(rows)


def append_talk_entry(channel: str, sender: str, text: str, reply_to: str | None = None) -> dict:
    ensure_talk_channels()
    talk_path = talk_channel_path(channel)
    payload = {
        "ts": now_iso(),
        "from": sender,
        "text": text,
    }
    if reply_to:
        payload["reply_to"] = reply_to
    with talk_path.open("a") as f:
        f.write(json.dumps(payload, ensure_ascii=True) + "\n")
    return payload


def notify_orchestrator_pane(
    text: str,
    channel: str = GENERAL_TALK_CHANNEL,
    prefix: str | None = None,
) -> None:
    try:
        ts = now_iso()
        slug = sanitize_channel_slug(channel) or GENERAL_TALK_CHANNEL
        payload = prefix if prefix is not None else f"[/talk#{slug} @ {ts}] {text}"
        if prefix is not None and text:
            payload = f"{prefix} {text}"
        subprocess.Popen(
            [HARNESS, "send", "orchestrator", payload],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            close_fds=True,
        )
    except Exception:
        pass


def talk_admin_controls(channel: str) -> str:
    slug = sanitize_channel_slug(channel) or GENERAL_TALK_CHANNEL
    clear_confirm = json.dumps(f"Clear all messages in #{slug}?")
    parts = [
        '<form method="post" action="/talk/clear" style="display:inline" '
        f"onsubmit='return confirm({clear_confirm});'>"
        f'<input type="hidden" name="c" value="{escape(slug)}">'
        '<button type="submit" class="ml-1 text-xs text-zinc-500 hover:text-amber-400" '
        'title="clear">clr</button>'
        "</form>"
    ]
    if slug != GENERAL_TALK_CHANNEL:
        delete_confirm = json.dumps(f"Delete channel #{slug}?")
        parts.append(
            '<form method="post" action="/talk/delete" style="display:inline" '
            f"onsubmit='return confirm({delete_confirm});'>"
            f'<input type="hidden" name="c" value="{escape(slug)}">'
            '<button type="submit" class="ml-1 text-xs text-zinc-500 hover:text-red-500" '
            'title="delete">&times;</button>'
            "</form>"
        )
    return "".join(parts)


# ─── templates ──────────────────────────────────────────────────────────────

BASE = """<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{{ title }} — orchestrator</title>
<script src="https://cdn.tailwindcss.com"></script>
{% if refresh %}<meta http-equiv="refresh" content="{{ refresh }}">{% endif %}
<style>
 .mono{font-family:ui-monospace,Menlo,Consolas,monospace}
 .ellip{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%}
 pre{white-space:pre-wrap;word-break:break-word}
 table tr:hover{background:rgba(255,255,255,.04)}
 .badge{display:inline-block;padding:.1rem .4rem;border-radius:.2rem;font-size:.7rem;line-height:1rem}
 .b-active{background:#16a34a;color:#fff}
 .b-pending{background:#eab308;color:#000}
 .b-cancelled,.b-done{background:#525252;color:#fff}
 .b-progressing{background:#0ea5e9;color:#fff}
 .b-stalled{background:#dc2626;color:#fff}
</style>
</head>
<body class="bg-zinc-950 text-zinc-200 mono text-sm">
<nav class="bg-zinc-900 border-b border-zinc-800 px-4 py-2 flex gap-4 items-center">
 <a href="/" class="font-bold text-zinc-100">orchestrator</a>
 <a href="/" class="hover:text-white">dashboard</a>
 <a href="/cycles" class="hover:text-white">cycles</a>
 <a href="/goals" class="hover:text-white">goals</a>
 <a href="/paths" class="hover:text-white">paths</a>
 <a href="/agents" class="hover:text-white">agents</a>
 <a href="/facts" class="hover:text-white">facts</a>
<a href="/services" class="hover:text-white">services</a>
<a href="/memory" class="hover:text-white">memory</a>
<a href="/briefings" class="hover:text-white">briefings</a>
<a href="/talk" class="hover:text-white">talk</a>
 <a href="/logout" class="hover:text-white">logout</a>
 <span class="ml-auto text-zinc-500 text-xs">{{ now }}</span>
</nav>
<main class="p-4 max-w-7xl mx-auto">
{{ body|safe }}
</main></body></html>"""


def render(title: str, body: str, refresh: int = 0) -> str:
    return render_template_string(BASE, title=title, body=body, refresh=refresh,
                                  now=datetime.now().strftime("%Y-%m-%d %H:%M:%S"))


def render_login(error: str | None = None) -> str:
    body = ['<section class="mx-auto max-w-md pt-16">']
    body.append('<div class="rounded border border-zinc-800 bg-zinc-900 p-6 shadow-2xl shadow-black/30">')
    body.append('<h1 class="mb-2 text-2xl text-zinc-100">Dashboard Login</h1>')
    body.append('<p class="mb-5 text-sm text-zinc-500">Enter the dashboard password to access the talk panel and operator views.</p>')
    if error:
        body.append(f'<div class="mb-4 rounded border border-red-900/60 bg-red-950/60 px-3 py-2 text-sm text-red-200">{escape(error)}</div>')
    body.append(
        '<form method="post" action="/login" class="space-y-4">'
        '<div>'
        '<label for="password" class="mb-2 block text-xs uppercase tracking-[0.2em] text-zinc-500">Password</label>'
        '<input id="password" type="password" name="password" autocomplete="current-password" '
        'class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100 '
        'placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none">'
        '</div>'
        '<button type="submit" class="w-full rounded bg-sky-600 px-4 py-2 text-sm text-white hover:bg-sky-500">Enter dashboard</button>'
        '</form>'
    )
    body.append('</div>')
    body.append('</section>')
    return render("login", "\n".join(body))


# ─── routes ─────────────────────────────────────────────────────────────────

@app.before_request
def require_dashboard_auth():
    if request.path.startswith("/login") or request.path.startswith("/static"):
        return None
    if is_authed(request):
        return None
    return redirect(url_for("login_view"), code=302)


@app.route("/login", methods=["GET", "POST"])
def login_view():
    if request.method == "POST":
        form_pw = request.form.get("password", "")
        if hash_password(form_pw) == PASSWORD_HASH:
            response = make_response(redirect(url_for("talk_view", c=GENERAL_TALK_CHANNEL), code=303))
            response.set_cookie(
                AUTH_COOKIE_NAME,
                signed_auth_value(),
                max_age=AUTH_COOKIE_MAX_AGE,
                httponly=True,
                samesite="Lax",
                secure=False,
            )
            return response
        return render_login(error="Wrong password")
    if is_authed(request):
        return redirect(url_for("talk_view", c=GENERAL_TALK_CHANNEL), code=302)
    return render_login()


@app.route("/logout")
def logout_view():
    response = make_response(redirect(url_for("login_view"), code=302))
    response.delete_cookie(AUTH_COOKIE_NAME, httponly=True, samesite="Lax")
    return response


@app.route("/")
def dashboard():
    eps = episodes(limit=8)
    gs = goals()
    active_goals = [g for g in gs if g.get("status") == "active"]
    paths = paths_portfolio()
    svcs = services()

    body = []
    body.append('<h1 class="text-2xl mb-4 text-zinc-100">Dashboard</h1>')

    # active campaigns
    body.append('<section class="mb-6">')
    body.append('<h2 class="text-lg text-zinc-100 mb-2">Active campaigns ({} of {})</h2>'.format(
        len(active_goals), len(gs)))
    if active_goals:
        body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                    '<th class="py-1 pr-3">goal</th><th>prio</th><th>title</th></tr></thead><tbody>')
        for g in active_goals:
            body.append(
                f'<tr><td class="pr-3 align-top"><a class="text-sky-400 hover:underline" href="/goal/{g["goal_key"]}">{g["goal_key"]}</a></td>'
                f'<td class="pr-3 align-top">{g.get("priority","")}</td>'
                f'<td class="align-top">{g["title"]}</td></tr>'
            )
        body.append('</tbody></table>')
    else:
        body.append('<div class="text-zinc-500">(no active goals)</div>')
    body.append('</section>')

    # path portfolio summary
    if paths and paths.get("goals"):
        body.append('<section class="mb-6">')
        body.append('<h2 class="text-lg text-zinc-100 mb-2">Path portfolio</h2>')
        body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                    '<th class="pr-3 py-1">goal</th><th class="pr-3">metric</th>'
                    '<th class="pr-3">progress</th><th>paths</th></tr></thead><tbody>')
        for gk, g in paths["goals"].items():
            cur = g.get("current", "?")
            tgt = g.get("target", "?")
            n = len(g.get("paths", []))
            statuses = ", ".join(p.get("status", "?") for p in g.get("paths", []))
            body.append(f'<tr><td class="pr-3 align-top">{gk}</td>'
                        f'<td class="pr-3 align-top">{g.get("metric_name","")}</td>'
                        f'<td class="pr-3 align-top">{cur}/{tgt}</td>'
                        f'<td class="align-top">{n} ({statuses})</td></tr>')
        body.append('</tbody></table></section>')

    # recent cycles
    body.append('<section class="mb-6">')
    body.append('<h2 class="text-lg text-zinc-100 mb-2">Recent cycles '
                '<a href="/cycles" class="text-xs text-sky-400">(all →)</a></h2>')
    body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                '<th class="pr-3 py-1">id</th><th class="pr-3">when</th>'
                '<th>summary</th></tr></thead><tbody>')
    for e in eps:
        body.append(f'<tr><td class="pr-3 align-top text-zinc-500">{e["id"]}</td>'
                    f'<td class="pr-3 align-top text-xs text-zinc-400">{e["created_at"][:19]}</td>'
                    f'<td class="align-top"><a class="hover:text-white" href="/cycle/{e["id"]}">{e["summary"][:180]}</a></td></tr>')
    body.append('</tbody></table></section>')

    # services
    if svcs:
        body.append('<section class="mb-6">')
        body.append('<h2 class="text-lg text-zinc-100 mb-2">Services</h2>')
        body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                    '<th class="pr-3 py-1">service</th><th class="pr-3">type</th>'
                    '<th class="pr-3">status</th><th>target</th></tr></thead><tbody>')
        for s in svcs:
            cls = "text-emerald-400" if s.get("last_status") == "healthy" else "text-amber-400"
            body.append(f'<tr><td class="pr-3 align-top">{s.get("service_name","")}</td>'
                        f'<td class="pr-3 align-top">{s.get("service_type","")}</td>'
                        f'<td class="pr-3 align-top {cls}">{s.get("last_status","")}</td>'
                        f'<td class="align-top text-xs text-zinc-500">{s.get("check_target","")}</td></tr>')
        body.append('</tbody></table></section>')

    return render("dashboard", "\n".join(body), refresh=30)


@app.route("/cycles")
def cycles_list():
    n = 200
    eps = episodes(limit=n)
    body = [f'<h1 class="text-2xl mb-4">Cycles <span class="text-sm text-zinc-500">(latest {len(eps)})</span></h1>']
    body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                '<th class="pr-3 py-1">id</th><th class="pr-3">when</th>'
                '<th class="pr-3">agents</th><th class="pr-3">actions</th>'
                '<th>summary</th></tr></thead><tbody>')
    for e in eps:
        as_ = e.get("agent_statuses_json") or {}
        at = e.get("actions_taken_json") or []
        body.append(f'<tr><td class="pr-3 align-top text-zinc-500">{e["id"]}</td>'
                    f'<td class="pr-3 align-top text-xs text-zinc-400">{e["created_at"][:19]}</td>'
                    f'<td class="pr-3 align-top text-xs text-zinc-500">{len(as_) if isinstance(as_,dict) else "?"}</td>'
                    f'<td class="pr-3 align-top text-xs text-zinc-500">{len(at) if isinstance(at,list) else "?"}</td>'
                    f'<td class="align-top"><a class="hover:text-white" href="/cycle/{e["id"]}">{e["summary"]}</a></td></tr>')
    body.append('</tbody></table>')
    return render("cycles", "\n".join(body))


@app.route("/cycle/<int:cid>")
def cycle_detail(cid):
    try:
        rows = sql_query(
            "SELECT id, created_at, summary, agent_statuses_json, "
            "actions_taken_json, goal_progress_json FROM episodes "
            f"WHERE id = {int(cid)}"
        )
        e = rows[0] if rows else None
        if e:
            for k in ("agent_statuses_json", "actions_taken_json", "goal_progress_json"):
                e[k] = parse_json_cell(e.get(k, "")) or {}
    except Exception:
        e = next((x for x in episodes(limit=500) if str(x["id"]) == str(cid)), None)
    if not e:
        abort(404)
    body = [f'<h1 class="text-2xl mb-1">Cycle {e["id"]}</h1>']
    body.append(f'<div class="text-zinc-500 text-xs mb-3">{e["created_at"]}</div>')
    body.append(f'<div class="bg-zinc-900 border border-zinc-800 rounded p-3 mb-3"><pre>{e["summary"]}</pre></div>')
    body.append('<div class="grid grid-cols-3 gap-3">')
    for k, label in [("agent_statuses_json","Agents"),("actions_taken_json","Actions"),("goal_progress_json","Goal progress")]:
        v = e.get(k)
        body.append('<div class="bg-zinc-900 border border-zinc-800 rounded p-3">')
        body.append(f'<div class="text-xs text-zinc-500 mb-1">{label}</div>')
        body.append(f'<pre class="text-xs">{json.dumps(v, indent=2) if v else "(none)"}</pre>')
        body.append('</div>')
    body.append('</div>')
    body.append('<div class="mt-4"><a class="text-sky-400 hover:underline" href="/cycles">← all cycles</a></div>')
    return render(f"cycle {cid}", "\n".join(body))


@app.route("/goals")
def goals_list():
    gs = goals()
    body = ['<h1 class="text-2xl mb-4">Goals</h1>']
    body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                '<th class="pr-3 py-1">status</th><th class="pr-3">prio</th>'
                '<th class="pr-3">key</th><th>title</th></tr></thead><tbody>')
    for g in gs:
        cls = {"active":"b-active","pending":"b-pending","done":"b-done","cancelled":"b-cancelled"}.get(g.get("status",""),"b-pending")
        body.append(f'<tr><td class="pr-3 align-top"><span class="badge {cls}">{g["status"]}</span></td>'
                    f'<td class="pr-3 align-top text-zinc-500">{g["priority"]}</td>'
                    f'<td class="pr-3 align-top"><a class="text-sky-400 hover:underline" href="/goal/{g["goal_key"]}">{g["goal_key"]}</a></td>'
                    f'<td class="align-top">{g["title"]}</td></tr>')
    body.append('</tbody></table>')
    return render("goals", "\n".join(body))


@app.route("/goal/<key>")
def goal_detail(key):
    gs = goals()
    g = next((x for x in gs if x.get("goal_key") == key), None)
    if not g:
        abort(404)
    body = [f'<h1 class="text-2xl mb-2">{g["goal_key"]}</h1>']
    body.append(f'<div class="mb-2"><span class="badge b-{g["status"]}">{g["status"]}</span> '
                f'<span class="text-zinc-500 text-xs">prio {g["priority"]}</span></div>')
    body.append(f'<div class="mb-3 text-zinc-300">{g["title"]}</div>')
    if g.get("detail") and g["detail"] != "(none)":
        body.append(f'<div class="bg-zinc-900 border border-zinc-800 rounded p-3 mb-3"><pre>{g["detail"]}</pre></div>')
    body.append('<div class="grid grid-cols-2 gap-3 text-xs">')
    for k in ("success_fact_key","completion_report","created_at","updated_at"):
        body.append(f'<div class="bg-zinc-900 border border-zinc-800 rounded p-2">'
                    f'<div class="text-zinc-500">{k}</div>'
                    f'<div>{g.get(k,"")}</div></div>')
    body.append('</div>')
    return render(f"goal {key}", "\n".join(body))


@app.route("/paths")
def paths_view():
    p = paths_portfolio()
    body = ['<h1 class="text-2xl mb-4">Path portfolio</h1>']
    if not p:
        body.append('<div class="text-zinc-500">(empty / unreadable analysis/paths.json)</div>')
    else:
        for gk, g in p.get("goals", {}).items():
            body.append(f'<section class="mb-5 bg-zinc-900 border border-zinc-800 rounded p-3">')
            body.append(f'<h2 class="text-lg mb-1">{gk}</h2>')
            body.append(f'<div class="text-xs text-zinc-500 mb-2">{g.get("metric_name","")} '
                        f'— current {g.get("current","?")}/{g.get("target","?")} '
                        f'— last move {g.get("last_move_at","?")}</div>')
            paths = g.get("paths", [])
            if not paths:
                body.append('<div class="text-zinc-500 text-xs">(no paths)</div>')
            else:
                body.append('<table class="w-full text-xs"><thead><tr class="text-zinc-500 text-left">'
                            '<th class="pr-2">name</th><th class="pr-2">status</th><th class="pr-2">stall</th>'
                            '<th class="pr-2">worker</th><th>hypothesis / falsification</th></tr></thead><tbody>')
                for pt in paths:
                    cls = "b-progressing" if pt.get("status")=="progressing" else "b-stalled"
                    body.append(f'<tr><td class="pr-2 align-top">{pt.get("name","")}</td>'
                                f'<td class="pr-2 align-top"><span class="badge {cls}">{pt.get("status","")}</span></td>'
                                f'<td class="pr-2 align-top">{pt.get("stall_counter",0)}</td>'
                                f'<td class="pr-2 align-top text-zinc-400">{pt.get("worker","")}</td>'
                                f'<td class="align-top text-zinc-300">'
                                f'<div>{pt.get("hypothesis","")}</div>'
                                f'<div class="text-zinc-500">{pt.get("falsification","")}</div></td></tr>')
                body.append('</tbody></table>')
            body.append('</section>')
    return render("paths", "\n".join(body))


@app.route("/agents")
def agents_list():
    a = agents()
    body = ['<h1 class="text-2xl mb-4">Agents</h1>']
    if not a:
        body.append('<div class="text-zinc-500">(no agents registered)</div>')
    else:
        body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                    '<th class="pr-3 py-1">name</th><th class="pr-3">kind</th><th class="pr-3">workdir</th>'
                    '<th>description</th></tr></thead><tbody>')
        for ag in a:
            body.append(f'<tr><td class="pr-3 align-top">{ag.get("agent_name","")}</td>'
                        f'<td class="pr-3 align-top text-xs text-zinc-500">{ag.get("kind","")}</td>'
                        f'<td class="pr-3 align-top text-xs">{ag.get("workdir","")}</td>'
                        f'<td class="align-top">{ag.get("description","")}</td></tr>')
        body.append('</tbody></table>')
    return render("agents", "\n".join(body))


@app.route("/facts")
def facts_list():
    fs = facts()
    body = ['<h1 class="text-2xl mb-4">Facts <span class="text-sm text-zinc-500">(latest)</span></h1>']
    body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                '<th class="pr-3 py-1">when</th><th class="pr-3">key</th>'
                '<th>value</th></tr></thead><tbody>')
    for f in fs:
        body.append(f'<tr><td class="pr-3 align-top text-xs text-zinc-500">{f.get("created_at","")[:19]}</td>'
                    f'<td class="pr-3 align-top">{f.get("fact_key","")}</td>'
                    f'<td class="align-top text-zinc-300">{f.get("fact_value","")}</td></tr>')
    body.append('</tbody></table>')
    return render("facts", "\n".join(body))


@app.route("/services")
def services_list():
    s = services()
    body = ['<h1 class="text-2xl mb-4">Services</h1>']
    body.append('<table class="w-full"><thead><tr class="text-zinc-500 text-left">'
                '<th class="pr-3 py-1">name</th><th class="pr-3">type</th>'
                '<th class="pr-3">status</th><th class="pr-3">last poll</th><th>target</th></tr></thead><tbody>')
    for sv in s:
        cls = "text-emerald-400" if sv.get("last_status") == "healthy" else "text-amber-400"
        body.append(f'<tr><td class="pr-3 align-top">{sv.get("service_name","")}</td>'
                    f'<td class="pr-3 align-top text-xs text-zinc-500">{sv.get("service_type","")}</td>'
                    f'<td class="pr-3 align-top {cls}">{sv.get("last_status","")}</td>'
                    f'<td class="pr-3 align-top text-xs text-zinc-500">{sv.get("last_polled_at","")[:19]}</td>'
                    f'<td class="align-top text-xs">{sv.get("check_target","")}</td></tr>')
    body.append('</tbody></table>')
    return render("services", "\n".join(body))


@app.route("/memory")
@app.route("/memory/<name>")
def memory_view(name=None):
    if name is None:
        files = list_md(MEMORY)
        body = [f'<h1 class="text-2xl mb-4">Memory <span class="text-sm text-zinc-500">({len(files)})</span></h1>']
        # always show MEMORY.md index first
        idx = read_md(MEMORY, "MEMORY.md")
        if idx:
            body.append('<details open class="mb-4 bg-zinc-900 border border-zinc-800 rounded p-3">'
                        '<summary class="cursor-pointer text-zinc-100">MEMORY.md (index)</summary>'
                        f'<pre class="text-xs mt-2">{idx}</pre></details>')
        body.append('<ul class="space-y-1">')
        for f in files:
            if f["name"] == "MEMORY.md":
                continue
            body.append(f'<li><a class="text-sky-400 hover:underline" href="/memory/{f["name"]}">{f["name"]}</a> '
                        f'<span class="text-zinc-500 text-xs">— {f["title"]} ({f["size"]}B, {f["mtime"].strftime("%Y-%m-%d %H:%M")})</span></li>')
        body.append('</ul>')
        return render("memory", "\n".join(body))
    content = read_md(MEMORY, name)
    if content is None:
        abort(404)
    body = [f'<h1 class="text-xl mb-2">{name}</h1>',
            f'<div class="bg-zinc-900 border border-zinc-800 rounded p-3"><pre class="text-xs">{content}</pre></div>',
            '<div class="mt-3"><a class="text-sky-400 hover:underline" href="/memory">← all memory</a></div>']
    return render(name, "\n".join(body))


@app.route("/briefings")
@app.route("/briefings/<name>")
def briefings_view(name=None):
    if name is None:
        files = list_md(BRIEFINGS)
        body = [f'<h1 class="text-2xl mb-4">Briefings <span class="text-sm text-zinc-500">({len(files)})</span></h1>']
        body.append('<ul class="space-y-1">')
        for f in files:
            body.append(f'<li><a class="text-sky-400 hover:underline" href="/briefings/{f["name"]}">{f["name"]}</a> '
                        f'<span class="text-zinc-500 text-xs">— {f["title"]} ({f["mtime"].strftime("%Y-%m-%d %H:%M")})</span></li>')
        body.append('</ul>')
        return render("briefings", "\n".join(body))
    content = read_md(BRIEFINGS, name)
    if content is None:
        abort(404)
    body = [f'<h1 class="text-xl mb-2">{name}</h1>',
            f'<div class="bg-zinc-900 border border-zinc-800 rounded p-3"><pre class="text-xs">{content}</pre></div>',
            '<div class="mt-3"><a class="text-sky-400 hover:underline" href="/briefings">← all briefings</a></div>']
    return render(name, "\n".join(body))


@app.route("/talk", methods=["GET", "POST"])
def talk_view():
    channel = sanitize_channel_slug(request.args.get("c")) or GENERAL_TALK_CHANNEL
    if request.method == "POST":
        sender = request.form.get("from", "user").strip() or "user"
        text = request.form.get("text", "").strip()
        if text:
            append_talk_entry(channel, sender, text)
            if sender == "user":
                try:
                    notify_orchestrator_pane(text, channel=channel)
                except Exception:
                    pass
            return redirect(url_for("talk_view", c=channel), code=303)

    channels = list_talk_channels()
    channel_store = f"analysis/talk_channels/{channel}.jsonl"
    body = ['<h1 class="text-2xl mb-4">Talk</h1>']
    body.append('<div class="flex flex-col gap-4 lg:flex-row">')
    body.append('<aside class="w-full shrink-0 lg:w-48">')
    body.append('<section class="bg-zinc-900 border border-zinc-800 rounded p-4">')
    body.append(
        f'<form action="/talk/new?c={escape(channel)}" method="post" class="mb-4 flex gap-2">'
        '<input type="text" name="name" maxlength="64" placeholder="new channel" '
        'class="min-w-0 flex-1 rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100 '
        'placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none">'
        '<button type="submit" class="rounded bg-zinc-800 px-3 py-2 text-sm text-zinc-100 hover:bg-zinc-700">+</button>'
        '</form>'
    )
    body.append('<nav class="space-y-1">')
    for slug in channels:
        link_cls = (
            "bg-sky-600/20 border-sky-500 text-sky-200"
            if slug == channel else
            "border-transparent text-zinc-300 hover:border-zinc-700 hover:bg-zinc-950/70 hover:text-white"
        )
        controls = talk_admin_controls(slug)
        body.append(
            '<div class="flex items-center">'
            '<a href="/talk?c={}" class="block min-w-0 flex-1 rounded border px-3 py-2 text-sm {}">#{}</a>'
            "{}"
            "</div>".format(escape(slug), link_cls, escape(slug), controls)
        )
    body.append('</nav>')
    body.append('</section>')
    body.append('</aside>')
    body.append('<section class="min-w-0 flex-1 bg-zinc-900 border border-zinc-800 rounded p-4">')
    header_controls = talk_admin_controls(channel)
    body.append(
        '<div class="mb-4 flex flex-wrap items-center justify-between gap-3">'
        '<div>'
        f'<div class="flex items-center"><h2 class="text-lg text-zinc-100">#{escape(channel)}</h2>{header_controls}</div>'
        f'<div class="text-xs text-zinc-500">Channel thread backed by {escape(channel_store)}.</div></div>'
        '<div class="text-xs text-zinc-500">Ctrl+Enter sends. Enter inserts a newline.</div>'
        '</div>'
    )

    entries = talk_entries(channel=channel, limit=200)
    # Marker for the incremental poller: total message count currently rendered.
    # talk_since() compares against this `n` to send only newer messages.
    initial_count = len(entries)
    empty_cls = " hidden" if entries else ""
    body.append(
        '<div id="talk-empty" class="text-zinc-500 mb-4{}">(no messages yet)</div>'.format(empty_cls)
    )
    body.append('<div id="talk-messages" class="space-y-3 mb-5">')
    if entries:
        for entry in entries:
            sender = str(entry.get("from", "") or "worker")
            ts = str(entry.get("ts", "") or "")
            text = escape(str(entry.get("text", "") or ""))
            reply_to = str(entry.get("reply_to", "") or "")
            badge = "bg-amber-500 text-black"
            if sender == "user":
                badge = "bg-sky-500 text-white"
            elif sender == "orchestrator":
                badge = "bg-emerald-500 text-white"
            indent = " pl-6" if reply_to else ""
            reply_meta = f'<span class="ml-2 text-zinc-600">reply_to {escape(reply_to)}</span>' if reply_to else ""
            body.append(
                '<article class="border border-zinc-800 rounded bg-zinc-950/70 p-3{}">'
                '<div class="flex items-center gap-2 mb-2">'
                '<span class="inline-block rounded px-2 py-0.5 text-xs {}">{}</span>'
                '<span class="text-zinc-500 text-xs">{}</span>{}'
                '</div>'
                '<pre class="text-sm text-zinc-200 whitespace-pre-wrap">{}</pre>'
                '</article>'.format(indent, badge, escape(sender), escape(ts), reply_meta, text)
            )
    body.append('</div>')

    body.append(
        f'<form method="post" action="/talk?c={escape(channel)}" class="border-t border-zinc-800 pt-4">'
        '<input type="hidden" name="from" value="user">'
        '<label for="talk-text" class="block text-xs text-zinc-500 mb-2">Post a message</label>'
        '<textarea id="talk-text" name="text" rows="3" '
        'class="w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100 '
        'placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none" '
        'placeholder="Ask the orchestrator something..."></textarea>'
        '<div class="mt-3 flex items-center justify-between gap-3">'
        '<div class="text-xs text-zinc-500">Drafts are stored per channel in sessionStorage.</div>'
        '<button type="submit" class="rounded bg-sky-600 px-4 py-2 text-sm text-white hover:bg-sky-500">Send</button>'
        '</div>'
        '</form>'
    )
    body.append('</section>')
    body.append('</div>')
    body.append(
        """<script>
        (function () {
          const channel = __CHANNEL_JSON__;
          let seen = __INITIAL_COUNT__;
          const ta = document.querySelector('textarea[name="text"]');
          const list = document.getElementById('talk-messages');
          const empty = document.getElementById('talk-empty');

          // ---- draft persistence + Ctrl+Enter send (unchanged behavior) ----
          if (ta) {
            const KEY = 'talk_draft_v2:' + channel;
            const saved = sessionStorage.getItem(KEY);
            if (saved && !ta.value) ta.value = saved;
            ta.addEventListener('input', function () {
              sessionStorage.setItem(KEY, ta.value);
            });
            ta.addEventListener('keydown', function (e) {
              if (e.ctrlKey && e.key === 'Enter') {
                e.preventDefault();
                if (ta.form && typeof ta.form.requestSubmit === 'function') {
                  ta.form.requestSubmit();
                  return;
                }
                if (ta.form) ta.form.submit();
              }
            });
            if (ta.form) {
              ta.form.addEventListener('submit', function () {
                sessionStorage.removeItem(KEY);
              });
            }
          }

          // Defuse any legacy meta-refresh that might still be present.
          document.querySelectorAll('meta[http-equiv="refresh"]').forEach(function (meta) {
            meta.remove();
          });

          // ---- incremental append (no full-page reload) ----
          function badgeClass(sender) {
            if (sender === 'user') return 'bg-sky-500 text-white';
            if (sender === 'orchestrator') return 'bg-emerald-500 text-white';
            return 'bg-amber-500 text-black';
          }
          function buildMessage(entry) {
            const sender = String(entry.from || 'worker');
            const ts = String(entry.ts || '');
            const text = String(entry.text || '');
            const replyTo = String(entry.reply_to || '');
            const article = document.createElement('article');
            article.className = 'border border-zinc-800 rounded bg-zinc-950/70 p-3' + (replyTo ? ' pl-6' : '');
            const head = document.createElement('div');
            head.className = 'flex items-center gap-2 mb-2';
            const badge = document.createElement('span');
            badge.className = 'inline-block rounded px-2 py-0.5 text-xs ' + badgeClass(sender);
            badge.textContent = sender;
            const tsEl = document.createElement('span');
            tsEl.className = 'text-zinc-500 text-xs';
            tsEl.textContent = ts;
            head.appendChild(badge);
            head.appendChild(tsEl);
            if (replyTo) {
              const r = document.createElement('span');
              r.className = 'ml-2 text-zinc-600';
              r.textContent = 'reply_to ' + replyTo;
              head.appendChild(r);
            }
            const pre = document.createElement('pre');
            pre.className = 'text-sm text-zinc-200 whitespace-pre-wrap';
            pre.textContent = text;
            article.appendChild(head);
            article.appendChild(pre);
            return article;
          }
          function nearBottom() {
            const doc = document.documentElement;
            return (window.innerHeight + window.scrollY) >= (doc.scrollHeight - 80);
          }
          let polling = false;
          async function poll() {
            if (polling || !list) return;
            polling = true;
            try {
              const res = await fetch('/talk/' + encodeURIComponent(channel) + '/since?n=' + seen, {
                headers: {'Accept': 'application/json'},
                credentials: 'same-origin',
              });
              if (!res.ok) return;
              const data = await res.json();
              if (!Array.isArray(data.messages) || data.messages.length === 0) {
                if (typeof data.count === 'number') seen = data.count;
                return;
              }
              const stick = nearBottom();
              for (const entry of data.messages) {
                list.appendChild(buildMessage(entry));
              }
              if (empty) empty.classList.add('hidden');
              seen = (typeof data.count === 'number') ? data.count : (seen + data.messages.length);
              if (stick) window.scrollTo(0, document.documentElement.scrollHeight);
            } catch (e) {
              /* transient network error: keep marker, retry next tick */
            } finally {
              polling = false;
            }
          }
          window.setInterval(poll, 3000);
        })();
        </script>""".replace("__CHANNEL_JSON__", json.dumps(channel)).replace("__INITIAL_COUNT__", json.dumps(initial_count))
    )
    return render("talk", "\n".join(body), refresh=0)


@app.route("/talk/<channel>/since")
def talk_since(channel: str):
    """Incremental diff endpoint: return only messages whose absolute line
    index is >= the client's last-seen count `n`. Used by the talk page to
    append new messages without a full-page reload."""
    slug = sanitize_channel_slug(channel) or GENERAL_TALK_CHANNEL
    try:
        since_n = int(request.args.get("n", "0"))
    except (TypeError, ValueError):
        since_n = 0
    if since_n < 0:
        since_n = 0
    # talk_entries() caps at `limit`; pass a large limit so `count` reflects the
    # true tail position and the client marker stays consistent.
    entries = talk_entries(channel=slug, limit=100000)
    total = len(entries)
    new_entries = entries[since_n:] if since_n < total else []
    return jsonify({"channel": slug, "count": total, "messages": new_entries})


@app.route("/talk/new", methods=["POST"])
def talk_new_channel():
    current = sanitize_channel_slug(request.args.get("c")) or GENERAL_TALK_CHANNEL
    channel = sanitize_channel_slug(request.form.get("name"))
    if not channel:
        return redirect(url_for("talk_view", c=current), code=303)
    ensure_talk_channels()
    talk_channel_path(channel).touch(exist_ok=True)
    return redirect(url_for("talk_view", c=channel), code=303)


@app.route("/talk/clear", methods=["POST"])
def talk_clear_channel():
    channel = sanitize_channel_slug(request.form.get("c") or request.args.get("c")) or GENERAL_TALK_CHANNEL
    ensure_talk_channels()
    talk_channel_path(channel).open("w").close()
    notify_orchestrator_pane(
        f"truncated analysis/talk_channels/{channel}.jsonl",
        channel=channel,
        prefix=f"[/talk#{channel} ADMIN clear]",
    )
    return redirect(url_for("talk_view", c=channel), code=303)


@app.route("/talk/delete", methods=["POST"])
def talk_delete_channel():
    channel = sanitize_channel_slug(request.form.get("c") or request.args.get("c")) or GENERAL_TALK_CHANNEL
    if channel == GENERAL_TALK_CHANNEL:
        return "cannot delete the default channel", 400
    try:
        talk_channel_path(channel).unlink()
    except FileNotFoundError:
        pass
    notify_orchestrator_pane(
        f"removed analysis/talk_channels/{channel}.jsonl",
        channel=channel,
        prefix=f"[/talk#{channel} ADMIN delete]",
    )
    return redirect(url_for("talk_view", c=GENERAL_TALK_CHANNEL), code=303)


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=3030, debug=False, use_reloader=False)
