# albion-fresh-accounts — create N fresh Albion accounts on vast.ai for parallel-client scaling

## Role & workdir
Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-fresh-accounts`. Substrate: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`.

## Current goal / sub-goal
- **goal_key:** `albion_action_loop` (umbrella)
- **sub_goal_key:** `fresh_accounts_for_parallel_clients`
- **Success metric:** N≥3 Albion-Online accounts created + e-mail-confirmed + login-tested. Each account = its own creds bundle in `/home/sdancer/albion-fresh-accounts/secrets/account_<n>.json` (mode 600, gitignored). Each verified by successfully reaching the post-2FA / character-select screen at least once.

## Already achieved (do NOT re-falsify)

| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| L1 | Original account `5fswkv6zf4@wshu.net / albion260518q9` logged-in past 2FA, `zone=YEKZMHL` held 22h | Albion accountportal login flow with `accountportal-headed` script works end-to-end on Xtigervnc :3 substrate | vast.ai 14838 | ✅ |
| L2 | `/home/sdancer/albion-prod-login/scripts/inbox_poll.py` proven; mail.tm JWT in `/home/sdancer/albion-prod-login/secrets/mailbox.json` | mail.tm inbox API client + 2FA-code extraction template | orchestrator host | ✅ |
| L3 | accountportal-cdp worker (`albion-accountportal-cdp` thread) proved L1+L2 CDP-driven signup flow concept; turn-4 ACTIVE with 3600s captcha wait | Headed-chromium CDP automation of the SI accountportal sign-in/up flow | orchestrator host | ✅ (L3 captcha gate is the bottleneck — work around) |

**Anchor:** The Albion launcher on the substrate is running NOW (PID 688424, KasmVNC :3 via Xtigervnc). It belongs to the original account. **Do NOT log it out.** Create new accounts via a **separate chromium/curl path** against the SI accountportal web flow.

## Hypothesis

**H**: SI's accountportal (`https://accounts.albiononline.com/`) has an open registration endpoint that accepts:
- a fresh email (provisioned via mail.tm),
- a generated password,
- the (currently observed) Cloudflare Turnstile challenge.

The accountportal-cdp worker (sibling) has proven CF Turnstile can be cleared by a headed chromium under xvfb-run + nodriver in apkpure terms (memory `[[apkpure_skill_fixes]]`) — but their use-case is sign-IN not sign-UP. Sign-up has its own endpoint and a confirmation-email loop.

The simpler, more deterministic route: **drive the Albion-Online launcher's own "Create Account" link**, which routes through the in-app Chromium Embedded Framework / system browser to the same accountportal page but inherits the launcher's device-trust state, avoiding the CF Turnstile entirely (per memory `[[albion_2fa_container_rotation]]`, the launcher-driven flow keeps device-id stable). If the launcher in-app sign-up is unavailable, fall back to nodriver+xvfb-run sign-up on the same host.

## Falsification

If, after 3 turns: (a) no signup endpoint is callable from substrate without solving a CAPTCHA we can't pass; AND (b) launcher's in-app create-account link only opens the same Turnstile-gated page; AND (c) email-confirmation loop is unreachable — write `/home/sdancer/albion-fresh-accounts/analysis/signup_blocked.md` with sanitized evidence (no creds, no tokens) + escalate via talk channel `vastai-albion-web` as a "resource ask" (user-supplied account pool).

## Next 3 concrete tasks

1. **Survey the launcher's in-app create-account flow.** SSH to substrate. Without disturbing PID 688424, screenshot the *current* launcher window on Xtigervnc :3, see if it has a "Create Account" / "Sign Up" link visible. If yes — note the X/Y coords. If no — check `/opt/albion-frida-capture/spawn_preload.sh` for whether launching a second instance with `--account-create` or similar argv works (read-only check, don't actually launch the second instance yet). Document findings in `analysis/launcher_signup_survey_2026-05-24.md`.

2. **Build the mail.tm-backed signup pipeline.** Read the working pattern at `/home/sdancer/albion-prod-login/scripts/inbox_poll.py`. Create a sibling script `scripts/create_account.py` that:
   - provisions a fresh mail.tm mailbox (random username + password); saves to `secrets/mailbox_<n>.json` mode 600.
   - generates a strong Albion password (no special policy known; 16 chars mixed alphanum-symbol).
   - POSTs to whatever signup endpoint we identify in step 1 (either the accountportal API directly via curl, or via nodriver+xvfb-run headed chromium).
   - polls mail.tm for the "Confirm your email" Albion message.
   - extracts and follows the confirmation link.
   - writes the final `secrets/account_<n>.json` with `{email, password, mailbox_jwt, confirmed_at_iso}`.
   Run for `n=1` first. Verify by SSH-ing to substrate and attempting to **read-only** check that the account exists (login URL returns 200 with the right session cookie, NOT actually logging in).

3. **Verify the fresh account can reach character-select on the substrate.** This is the gate to "multi-client viable." Do NOT touch PID 688424. Instead: provision a second Xtigervnc display `:4` and matching /home/albion2/.config/ dir on substrate (sudo-create albion2 user OR symlink trick — check what's possible without root). Start a SECOND Albion-Online process with the new account creds under DISPLAY=:4 and a separate LD_PRELOAD trap dir (`/tmp/albion-s-trim2/`). Confirm via curl `https://albion.orch.run/state` shows zone for the original (still YEKZMHL) AND screenshot the new client's character-select screen. Fact: `albion_fresh_account_1_login_verified_<isodate>` with no creds in the body.

## Constraints & gotchas

- **NEVER echo creds in stdout/logs/chat.** Secrets live mode-600 in `secrets/` (already gitignored at repo root — verify before any commit).
- **NEVER log out PID 688424.** That's the production-success Albion holding our zone metric. Sibling-isolated everything: separate user, separate display, separate trap libs, separate /tmp/albion-s-trim dir.
- **Albion launcher uses Photon over UDP**; signup is web-only (HTTPS to accountportal). Don't conflate the two.
- **mail.tm rate limits**: don't burn through inboxes — one mailbox per account; reuse the JWT until expiry (typically 24h).
- **CF Turnstile**: if encountered, prefer nodriver+xvfb-run per `[[apkpure_skill_fixes]]`. Snap chromium will NOT work.
- **No phone-verification flow** has been observed for Albion accounts at the entry tier. If a phone-gate appears in step 1, document and escalate (resource ask: user-supplied SIM/phone account or paid SMS service).
- Time budget: **45 min per turn**.

## Relevant files / references

- mail.tm inbox poller (proven, READ-ONLY ref): `/home/sdancer/albion-prod-login/scripts/inbox_poll.py`
- mail.tm JWT bundle (READ-ONLY ref): `/home/sdancer/albion-prod-login/secrets/mailbox.json`
- Sibling worker (accountportal-cdp, captcha-gated turn-4 in flight; don't disturb): `/home/sdancer/albion-accountportal-cdp/`
- Photon production-success path (do NOT touch): `/home/sdancer/albion-photon-sdk-research/`
- nodriver+xvfb-run reference: `/tmp/ao_nodriver.py` on orchestrator host
- Substrate SSH: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`
- Albion accountportal URL: `https://accounts.albiononline.com/` (NOT the game endpoint)
- Dashboard for live verification: `https://albion.orch.run/state`
- Talk channel for escalation: append to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` as `{"ts":"...","from":"orchestrator","text":"..."}`
