# dashboard-screenshots — multi-display screenshots on albion.orch.run with JPG default

## Role & workdir
Codex worker (`codex_app_server`). Workdir: **fresh worktree** at `/home/sdancer/albion-gamestate-service-screenshots` from the `albion-gamestate-service` repo (the orchestrator will `git worktree add` for you; if it doesn't exist yet, create it from `/home/sdancer/albion-gamestate-service` main HEAD).

## Goal
- **goal_key:** `dashboard_screenshots_multi_jpg`
- **success_fact_key:** `dashboard_screenshots_multi_jpg_shipped`
- **Success metric (binary):** `curl https://albion.orch.run/screenshots.json` returns a JSON array of ≥2 available screens with metadata; `curl https://albion.orch.run/screenshot.jpg?display=:3` returns a non-empty JPEG (>10KB) sourced from vast.ai DISPLAY=:3 (the live Albion Xtigervnc); the dashboard's "Live View" tab visibly displays the actual game window when loaded; JPG is the default (`/screenshot` and `/screenshot.jpg` both return JPEG; `/screenshot.png` opt-in only).

## Why this is the next move
User reported in talk channel: *"https://albion.orch.run/#screenshot < this doesn't show any on the screens, make it able to show them, allow for default of cheap jpg screenshot rather than full quality png"*. Confirmed root cause via orchestrator probe: current code hard-codes `SCREENSHOT_DISPLAY=":2"` (line 38 of `gamestate_service.py`), but vast.ai's `:2` (KasmVNC) currently renders a 406-byte solid-grayscale frame — empty. Meanwhile `:3` (Xtigervnc, where production Albion `Veldrynx` runs) and `:4` (another Xtigervnc) ARE alive with content. The HTML "Live View" tab and `/screenshot.png` endpoint exist but point at the dead `:2` display.

## Already implemented (do NOT re-build)

| Component | Location | Status |
|---|---|---|
| `GameStateService.get_screenshot_png()` with cache + lock | `scripts/gamestate_service.py:532-552` | ✅ works, PNG only, single hard-coded display |
| `_capture_screenshot_png()` via ImageMagick `import` resized to 1280x1280 | `scripts/gamestate_service.py:554-573` | ✅ works for any DISPLAY |
| `/screenshot.png` HTTP route in `StateRequestHandler.do_GET` | `scripts/gamestate_service.py:1668` | ✅ wired |
| HTML "Live View" tab UI scaffold (`tab-screenshot`, `screen-img`, `screen-meta`, `screen-head` CSS classes + JS auto-refresh loop) | `scripts/gamestate_service.py:1237, 1328-1338, 1352-1429, 1617-1621` | ✅ wired, just points at dead display |
| `deploy.sh` ships code to vast.ai `/opt/albion-gamestate/` and restarts supervised service | `scripts/deploy.sh` | ✅ canonical deploy path |

## Concrete plan (~90min budget)

### Task 1 — refactor capture to multi-display + JPG (~30min)
1. Add `SCREENSHOTS_DEFAULT_DISPLAYS=":3,:4,:2"` constant (env-overridable). Order matters: `:3` first because that's where the live Albion runs on vast.ai per `[[albion_login_substrate]]`.
2. Generalize `_capture_screenshot_png(display)` and `_capture_screenshot_jpg(display, quality=75)`. JPG capture: `import -window root -resize 1280x1280> -quality 75 jpg:-`. JPG is default; PNG is only used when explicitly requested.
3. Per-display cache: `{display -> (jpeg_bytes, ts), (png_bytes, ts)}` dicts under `screenshot_lock`. Keep TTL at 0.9s.
4. New `list_available_displays()` helper that reads `/tmp/.X<N>-lock` files and returns `[':3', ':4', ':2']` order-respecting. Skip lock files for displays that don't have a backing X server process.
5. New `get_screenshot_jpg(display)` mirroring `get_screenshot_png(display)`.

### Task 2 — HTTP routes (~15min)
1. `/screenshot.jpg` — JPG default, optional `?display=:3` query (defaults to first available from `SCREENSHOTS_DEFAULT_DISPLAYS`).
2. `/screenshot` — alias for `/screenshot.jpg`.
3. `/screenshot.png` — PNG opt-in, same `?display=` query.
4. `/screenshots.json` — JSON metadata: `[{"display":":3","label":"Albion live (Xtigervnc :3)","width":1920,"height":1080,"jpg_size":nnn,"last_capture_ts":nnn,"error":null},...]`. Skip displays where capture errored.
5. Keep the old `/screenshot.png` working (backwards-compatible — defaults to first available display rather than :2).

### Task 3 — HTML "Live View" tab (~20min)
1. Replace the single `<img>` with: a `<select id="screenshot-display">` dropdown populated from `/screenshots.json`, plus the existing `<img>`.
2. On dropdown change, update `<img>` src to `/screenshot.jpg?display=<selected>&t=<ts>` and persist selection in `localStorage`.
3. JS refresh loop: every 1s while the tab is active, update src with cache-buster. Same logic as today, just respect the selector.
4. Title and blurb update to "Live displays — choose substrate" (replace "DISPLAY=:2 screenshot of the live KasmVNC X session").
5. Update the `<a href="/screenshot.png">` link to `<a href="/screenshot.jpg">`.

### Task 4 — deploy + verify (~15min)
1. `./scripts/deploy.sh` to push to vast.ai.
2. Validate:
   - `curl -s https://albion.orch.run/screenshots.json | python3 -m json.tool` returns ≥2 displays.
   - `curl -s -o /tmp/scrjpg.jpg https://albion.orch.run/screenshot.jpg?display=:3 && file /tmp/scrjpg.jpg` → "JPEG image data"; size >10KB.
   - `curl -s -o /tmp/scrpng.png https://albion.orch.run/screenshot.png?display=:3 && file /tmp/scrpng.png` → "PNG image data".
   - Open browser to https://albion.orch.run/#screenshot — confirm the actual Albion window is visible, dropdown lists available displays.
3. Set fact `dashboard_screenshots_multi_jpg_shipped` with deployed-commit-sha + the curl outputs.
4. Post one-line milestone to `analysis/talk_channels/vastai-albion-web.jsonl` from `dashboard-screenshots` with the public URL the user can refresh.
5. Write `analysis/dashboard_screenshots_verdict_2026-05-24.md` summarizing what shipped.

### Task 5 — git + cleanup (~10min)
- Commit on a feature branch in the worktree; do NOT push unless asked.
- Leave `deploy.sh` artifact log under `/var/log/` paths visible in the verdict.
- Worktree stays — orchestrator decides when to `git worktree remove`.

## Falsification (mechanism-scoped — read this before any closure)

This path is falsified ONLY if EVERY mechanism in the relevant class fails. Specifically:
- If `import -window root` fails on vast.ai across DISPLAYs :3 AND :4 AND :2 (UI-scraping class), enumerate ≥3 untried alternatives BEFORE writing any `*_blocked.md`:
  1. `xwd -root | xwdtopnm | pnmtojpeg` (XWD-based capture)
  2. `ffmpeg -f x11grab -i :3 -frames:v 1 -q:v 5 jpg:-` (x11grab)
  3. `python3 -c "from PIL import ImageGrab; ImageGrab.grab().save(...)" ` (PIL via Xlib backend)
  4. `gnome-screenshot`/`scrot` if installed
  5. RFB-direct screenshot via `vncdo --server :3 capture screen.png` (talks to the VNC socket, bypasses X entirely)

Per orchestrator skill *Falsification scoping* rule. NEVER commit `*_falsified.md` after just one mechanism fails.

## Constraints & gotchas
- **Do NOT push to git remote.** Local commits only on the worktree branch.
- **Do NOT echo creds/JWTs/tokens.** No secrets in JSONL/git/chat.
- **The vast.ai gamestate-service runs supervised** (`/usr/local/bin/albion-supervise gamestate_service`). The deploy.sh handles restart correctly — do NOT manually `pkill` the service.
- **Keep PNG endpoint working** for backwards compatibility with anything that already curls it.
- **Cache TTL 0.9s is per-display now** — don't share one cache across displays.
- **Resize cap 1280x1280** stays; JPG quality 75 default is right (verify image size <100KB).
- **`SCREENSHOT_DISPLAY=":2"` constant is now stale** — remove it OR repurpose it as `SCREENSHOTS_DEFAULT_DISPLAYS=":3,:4,:2"`.
- ImageMagick `import` with `-quality 75 jpg:-` produces JPEG directly; no need for PIL.
- If `/tmp/.X<N>-lock` exists but the X server is dead, capture will fail — let it skip with the per-display `error` field rather than failing the whole list.

## Memory references
- `[[albion_login_substrate]]` — Xtigervnc :3 is the live display on vast.ai (where Albion is rendered post-cycle-3151 substrate fix)
- `[[falsify-mechanism-not-path]]` — mechanism-scoped falsification rule

## Relevant files / references
- `scripts/gamestate_service.py` — the entire service (single file, ~1700 lines)
- `scripts/deploy.sh` — pushes to `/opt/albion-gamestate/` on vast.ai
- HTML template inlined inside `gamestate_service.py` at lines ~1167-1640
- Cloudflared tunnel: vast.ai → albion.orch.run (already provisioned, don't touch)
- vast.ai SSH: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai` (key at `/home/sdancer/.ssh/id_ed25519`)

## Side-channel abort
Each iteration: `test -f /tmp/abort_dashboard-screenshots`.
