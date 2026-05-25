# albion-magic-link — investigate 2FA email body for trust-grant link

## Role & workdir
Fresh codex_app_server worker. Workdir: `/home/sdancer/albion-magic-link`. NOT the same worker as albion-prod-login (different path).

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** find an HTTP-based trust-grant path that doesn't require synthesized input on Albion's :3 modal.

## Already established (do not re-investigate)
- All 5 synthesized-input substrates falsified against Albion 2FA modal: xdotool focus/window, raw RFB, full RFB, vncdotool. See `/home/sdancer/albion-prod-login/analysis/SUBSTRATE_BLOCKER_README.md` for evidence trail.
- Unity prefs contain login.accountname + login.hash but NO session/trust token.
- mail.tm credentials valid; PRV6T7E code captured at 13:55:05Z but unusable (no input substrate works).
- Account email: `5fswkv6zf4@wshu.net`. Mailbox secret: `/home/sdancer/albion-prod-login/secrets/mailbox.json` (gitignored, mode 644 root). Use it via `cat` from your worker; do not commit anywhere.

## Hypothesis
Some 2FA emails include a clickable "trust this device" or "approve this login" HTTP link in addition to the 6-digit code. If Albion's email has such a link, GET-requesting it from the vast.ai container (whose egress IP is what Albion sees) would mark the IP as trusted SERVER-SIDE without requiring any input to the Unity client.

## Falsification
The PRV6T7E email body, when fully dumped, contains NO HTTP(S) URLs other than asset/footer/unsubscribe links. OR: the trust-grant link, when GETed from the container, returns an error or a UI page requiring further user action (not auto-grant).

## Tasks
1. **Recover mail.tm session.** Read `/home/sdancer/albion-prod-login/secrets/mailbox.json` for `address` + `password`. POST to `https://api.mail.tm/token` with that JSON; get JWT bearer. (Reference flow: `/home/sdancer/albion-prod-login/scripts/inbox_poll.py`.)

2. **Dump the full PRV6T7E message body.** `GET https://api.mail.tm/messages?from=albiononline.com` (or list all and grep for the right one), then fetch message detail (`GET /messages/{id}`) and inspect `html[]` + `text` fields. Save the raw email body to `analysis/2fa_email_full_body.txt` (NOT in git — add to .gitignore).

3. **Identify trust-grant link candidates.**
   - Extract all `href=` URLs from html body (use BeautifulSoup or regex).
   - Filter out: unsubscribe, view-in-browser, asset CDNs (`cdn.albion.com` etc), social media, footer.
   - Flag any URL containing tokens like `verify`, `approve`, `trust`, `device`, `grant`, `confirm`, `auth`, or a long random-looking token query parameter.
   - Save filtered candidates to `analysis/candidate_links.json`.

4. **Test candidate from the container.** If 1+ candidate found, SSH into the vast.ai container (`ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`) and run `curl -sSL -w '\n%{http_code}\n' '<URL>'` for each candidate. Capture response body to `analysis/link_grant_response_<n>.html`. Egress IP is `92.223.84.84` per prior facts.

5. **Re-check /state.** After each link GET, immediately curl `https://albion.orch.run/state` to see if zone populates (server-side trust grant would let the running Albion's 2FA modal resolve OR the next auto-login attempt would succeed).

6. **Report.** Append a milestone entry to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` with: (a) full link inventory, (b) which links you GET'd, (c) HTTP responses summary, (d) `/state` zone value before vs after.

## Acceptance / closure
- HARD WIN: `/state self.zone != null` within ≤30min — goal MET.
- SOFT WIN: trust-grant URL found in email; GET returns expected page; `/state` still null but next Albion relaunch skips 2FA (would require restarting Albion to test — DON'T do this without flagging; touches running daemon stack).
- FALSIFICATION: no trust-grant links in email body OR all candidate GETs return non-grant pages.

## Constraints (CRITICAL)
- **Do NOT restart Albion, photon-pcap-send, gamestate_service, albion-frida-ingest, or cloudflared.** These daemons are stable; restart = lose running 2FA window if user is about to act manually.
- **Do NOT commit secrets/mailbox.json to git.** Treat the file as ephemeral.
- **Do NOT modify the running albion-prod-login worker's worktree.** Your worktree is `/home/sdancer/albion-magic-link`, fully isolated.
- **Time budget**: 30 min max. If no progress, append a falsification note and exit.

## Relevant files / refs
- `/home/sdancer/albion-prod-login/secrets/mailbox.json` — mail.tm address+password (read-only for you)
- `/home/sdancer/albion-prod-login/scripts/inbox_poll.py` — reference for mail.tm flow
- `/home/sdancer/albion-prod-login/analysis/SUBSTRATE_BLOCKER_README.md` — full context dump
- `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` — append milestone here
- Memory `[[albion-2fa-container-rotation]]` for 2FA device-trust mechanism.
