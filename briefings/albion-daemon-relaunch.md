# albion-daemon-relaunch — FIX public KasmVNC URL (cloudflared ingress / nginx redirect)

## Role & workdir
Codex worker (codex_app_server, durable thread). Workdir reference: `/home/sdancer/vastai-albion`. Container access: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal
**Fix the public browser route for KasmVNC :2.** Your prior turn declared the cutover done and posted https://albion.orch.run/vnc/index.html — but from an external browser perspective the URL is broken:

```
$ curl -sIk https://albion.orch.run/vnc/index.html
HTTP/2 404
cf-cache-status: DYNAMIC      ← origin returned 404, not Cloudflare cache miss
server: cloudflare

$ curl -sIk https://albion.orch.run/vnc/
HTTP/2 302
location: http://albion.orch.run:8765/vnc/index.html   ← absolute URL to a non-public HTTP port!
```

The 302 reveals nginx is emitting an **absolute redirect to `albion.orch.run:8765`** — that port is not exposed via cloudflared (only `/state` and the existing dashboard ingress are), and even if it were, the protocol is `http` (cleartext) which Cloudflare's HTTPS edge would reject. Users following that redirect end up nowhere.

The 404 on `/vnc/index.html` directly indicates cloudflared's ingress rule for `/vnc/...` is **not routing to your nginx vnc location** — either the cloudflared config never got the rule, or the rule's path-prefix or service URL is wrong.

`/state` works fine (HTTP 200, source=frida, ev=21), so cloudflared tunnel is up. The fix is in (a) cloudflared ingress, and/or (b) nginx redirect behavior.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| 1 | KasmVNC `:2` server running, PID 113635 | Xkasmvnc alive on display :2 with -ac | vast.ai container | ✅ DONE |
| 2 | XFCE session alive on `:2` (113644/113650) | desktop environment running, Albion can attach | vast.ai | ✅ DONE |
| 3 | Albion-Online PID 114813 confirmed `DISPLAY=:2` via /proc/<pid>/environ | game on the new display | vast.ai | ✅ DONE |
| 4 | photon-pcap + frida-ingest + cloudflared + gamestate supervisors all up | 5-daemon stack still alive | vast.ai | ✅ DONE |
| 5 | https://albion.orch.run/state returns HTTP 200 ev=21 from source=frida | gamestate tunnel preserved through cutover | public | ✅ DONE |
| 6 | nginx on container with /vnc location block | proxy in place | vast.ai | ⚠️ MISCONFIGURED (absolute redirect + cloudflared ingress mismatch) |

**Do not tear anything down to fix this.** The 5-daemon stack and KasmVNC :2 itself are working. The fix is purely in nginx config + cloudflared ingress.

## Success criteria
1. `curl -sIk https://albion.orch.run/vnc/index.html` returns **HTTP 200** with `content-type: text/html` from the origin (KasmVNC's noVNC index).
2. `curl -sIk https://albion.orch.run/vnc/` either returns 200 OR returns a 30x with a **relative** Location header (`location: /vnc/index.html` or `location: ./index.html`) that ends up at 200.
3. The 5-daemon stack remains alive: `pgrep -af cloudflared|gamestate_service|Albion-Online|photon-pcap|albion-frida-ingest` returns 5 rows. `/state` still HTTP 200 with ev advancing.
4. Append a milestone to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` confirming the URL works (post the actual HTTP 200 headers as evidence).

## Tasks

### Task 1 — Pinpoint the failure layer
1. `ssh ... 'curl -sI http://localhost:8765/vnc/index.html'` — does **nginx on the container** serve `/vnc/index.html` directly? If 404 here, fix nginx config. If 200 here but 404 from albion.orch.run, fix cloudflared.
2. `ssh ... 'cat /etc/nginx/sites-enabled/*; cat /etc/nginx/conf.d/*.conf; nginx -T 2>&1 | head -200'` — read the **actual** active nginx config and find the `/vnc` location block. Identify:
   - Is `proxy_pass` pointing at the KasmVNC HTTPS port (likely `https://127.0.0.1:8444` or wherever Xkasmvnc -websocketPort landed)?
   - Is there a `proxy_redirect` directive rewriting absolute URLs back to relative? Without that, KasmVNC's own Location headers leak through with internal hostnames.
   - Is the `rewrite` or `try_files` clause causing the 302 to absolute URL?
3. `ssh ... 'cat /root/.cloudflared/config.yml 2>/dev/null || find / -name "config.yml" -path "*cloudflared*" 2>/dev/null | head -3 | xargs cat'` — confirm the cloudflared ingress has a `/vnc` path rule pointing at `http://localhost:8765` (the nginx edge).

### Task 2 — Fix the nginx redirect
The `Location: http://albion.orch.run:8765/...` is wrong on two counts: (a) absolute URL with port :8765 leaks an internal port to clients, (b) http: scheme will be rejected by Cloudflare's HTTPS edge. Fixes:
- Replace the `/vnc` location block's redirect/rewrite with one that emits a **relative** Location (no host, no port, no scheme), OR
- Add `proxy_redirect default;` (or explicit `proxy_redirect http://127.0.0.1:8444/ /vnc/;`) so nginx rewrites KasmVNC's internal redirects to public paths, OR
- Use `proxy_pass https://127.0.0.1:8444/;` with trailing slash so nginx strips the `/vnc` prefix before forwarding, then rewrite responses.

Whichever is the actual smallest delta — read the existing config first, don't overhaul.

After change: `nginx -t && nginx -s reload` (or supervisor restart of the nginx supervisor).

### Task 3 — Fix cloudflared ingress (if needed)
If task 1.1 shows nginx serves /vnc/index.html on localhost:8765 correctly but cloudflared returns 404 publicly, the cloudflared ingress doesn't have a `/vnc` rule. Add it:
```yaml
ingress:
  - hostname: albion.orch.run
    path: ^/vnc(/.*)?$
    service: http://localhost:8765
  - hostname: albion.orch.run
    service: http://localhost:8765   # existing default
  - service: http_status:404
```
Then SIGHUP cloudflared (don't kill it — under supervisor it will respawn, defeating the purpose) or use `cloudflared tunnel restart` if supported.

### Task 4 — Verify externally
From the orchestrator host (not inside the container): `curl -sIk https://albion.orch.run/vnc/index.html` → must return 200, `content-type: text/html`. Then `curl -sk https://albion.orch.run/vnc/index.html | head -20` → must return KasmVNC's noVNC HTML, NOT cloudflare's 404 page.

### Task 5 — Post milestone with proof
Append to talk channel:
```json
{"ts":"<iso>","from":"orchestrator","text":"VNC URL fix verified. curl -sIk https://albion.orch.run/vnc/index.html → HTTP 200 text/html (paste 3-4 header lines). Root cause was <one-line>. Fix was <one-line>. pw=albion26 still valid."}
```
Update fact `albion_kasmvnc2_browser_url_2026_05_22` to remain `https://albion.orch.run/vnc/index.html` once verified, and add a new fact `albion_kasmvnc2_url_verified_2026_05_22 = HTTP 200`.

## Constraints
- **Do NOT rebuild the 5-daemon stack.** It's healthy. The fix is configuration-only.
- **Do NOT kill Albion or KasmVNC.** Both are alive on :2 with state worth keeping.
- **Do NOT change cloudflared while it's serving /state for the dashboard** without SIGHUP-style reload — if cloudflared dies mid-edit the gamestate surface goes dark.
- LD_PRELOAD photon_tap.so MUST remain -DDISABLE_SEND_HOOKS per [[albion-send-hooks-break-client]].
- Keep your debug output focused — don't burn a 14k-event turn on inventory again; you've already inventoried.

## Relevant files / references
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Memory pointers: `[[albion-vastai-daemon-stack]]`, `[[kasmvnc-rfbauth-legacy-des]]`, `[[albion-send-hooks-break-client]]`
- Existing milestone (your prior turn's): see talk channel timestamp 2026-05-22T07:48:30Z — that one is now contradicted by external probes.

## Reporting
Either (a) a proof-positive HTTP 200 milestone with curl evidence, or (b) a precise blocker description (which config layer is wrong, what you tried, what the error was) — not vague "I think it works."
