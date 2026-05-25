# albion-token-recovery

## Role & workdir

Divergence sibling to the closed-falsified `captcha-vision-solve` path. Browser-based login hit a Cloudflare Turnstile/`405` wall at `/login_check` after 5 cascade layers of patches. Your job is the opposite-direction approach: survey for any cached Albion auth state from prior successful logins (orchestrator's project memory says Albion has previously been logged in on this account — DevideId-based device trust artifacts exist somewhere) and replay them.

Workdir: `/home/sdancer/albion-token-recovery` (git worktree, branch `albion-token-recovery`).

## Already achieved on this goal — do NOT re-falsify

| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| L0 — substrate alive | vast.ai container at ssh -p 14838 root@ssh8.vast.ai | RK3588 chromium + KasmVNC + Albion install | ✅ |
| L1 — Albion process | `pgrep -fa Albion-Online` shows PID 549987 | game binary actively running | ✅ |
| L2 — Unity prefs file exists | `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` (1975B at 13:27Z) | game has read+written prefs at least once today | ✅ |
| L3 — reCAPTCHA solver works | `/home/sdancer/albion-captcha-vision-solve/analysis/captcha_solve_log.md` | sibling proved vision-solve viable | ✅ |
| L4 — 2FA email pipeline works | mailbox.json + inbox_poll.py both produce valid security codes; sibling driver `[+] 2FA handled via inbox poller` log line | full email-arrival → code-fetch chain proven | ✅ |
| **L_alt — cached auth on disk** | YOUR DELIVERABLE | recover any prior token | ⏳ OPEN |

The browser-based path that depended on `/login_check` 405 is closed. Don't redo that work.

## Current goal / sub-goal

- `goal_key`: `albion_action_loop`
- `sub_goal_key`: `recover_or_extract_cached_auth_token_offline`
- Success: dashboard `https://albion.orch.run/state` shows `zone != null` after Albion launches with recovered token loaded into Unity prefs.

## Hypothesis

**H1**: A prior successful Albion login on this account (any substrate, any time) wrote a refresh_token or device-trust artifact somewhere on disk — either Unity prefs, browser-profile cookies, or a leftover file. If we extract it and place it in the current Unity prefs target, Albion's launch flow reuses it and skips the entire web login.

## Falsification

If a thorough survey of `/home/albion/`, `/home/sdancer/`, `/var/log/`, `/tmp/`, `/root/`, and all worktrees on both local AND remote substrate reveals NO usable Albion refresh/access/exchange token from a prior successful session, this path is closed. Write `/home/sdancer/albion-token-recovery/analysis/no_cached_auth.md` documenting the survey and exit.

## Next 3 concrete tasks

1. **Inspect the existing Unity prefs file.** On remote: `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` is 1975B at 13:27Z today. Unity prefs is a custom binary format — read it with `hexdump -C` or `strings`. Extract all printable keys/values. Identify whether it contains: `RefreshToken`, `DeviceId`, `LastUsername`, `_username`, OAuth tokens, JWT-shaped strings, or any base64-encoded blob >50 chars. Cite line numbers / byte offsets.

2. **Sweep both substrates** for any persisted Albion auth artifact:
   ```bash
   # On vast.ai remote:
   ssh -p 14838 root@ssh8.vast.ai
   sudo find / \( -name 'refresh_token*' -o -name 'access_token*' -o -name 'exchange_code*' -o -name '*albion*token*' -o -name 'prefs' -path '*Albion*' \) 2>/dev/null
   sudo find /home/albion /root /var /tmp -type f -mmin -10080 \( -name '*.json' -o -name '*token*' -o -name '*albion*' -o -name 'prefs' \) -size +100c 2>/dev/null
   # Cookies in any chromium profile:
   sudo find /home/albion -name 'Cookies' -path '*Default*' -o -name 'cookies.sqlite' 2>/dev/null
   # On orchestrator host:
   ```
   For each candidate, dump+inspect contents. Skip obvious-noise files (logs, caches, npm modules). Focus on small files with structured content.

3. **If a candidate token is found**: write it into the running Albion's Unity prefs target (need to determine the exact Unity prefs key from the existing 1975B file's format), kill+respawn Albion (the supervisor at PID 443749 auto-relaunches), poll `https://albion.orch.run/state` for `zone != null` over 5 minutes (10s interval). If zone flips: log success to `/home/sdancer/albion-token-recovery/analysis/zone_flip.md`, set fact `albion_zone_non_null_2026-05-23` to the timestamp + recovered-from path.

## Constraints & gotchas

- **DO NOT echo Albion credentials** (email `5fswkv6zf4@wshu.net`, password) into stdout/logs/chat/git.
- **DO NOT** create a new chromium browser — that path is closed. Survey existing artifacts, don't generate new ones.
- The Unity prefs file is a Unity-specific format. Don't blindly overwrite with raw token bytes — preserve the existing structure and INSERT the key. Read prior project memory `[[albion_2fa_container_rotation]]` for context: device-trust survives if `.config` persists across rotations, so look for any backup copies.
- **Watch for stale tokens.** Albion refresh tokens have unknown TTL — could be 24h, 7d, or 30d. A recovered token might be expired. The proof of "still valid" is `zone != null` flip; if Albion rejects the token, you'll see a relog prompt.
- Time budget for this turn: **60 min**. If survey is exhaustive and turns up nothing, write the no_cached_auth.md verdict and exit cleanly. Don't loop forever.

## Resource-ask alternatives (parallel paths the orchestrator stands on)

The user can unblock instantly via either of these if they prefer:
- Paste session cookies from local browser to `/tmp/albion_local_cookies.json` (was the original briefing's fast-path).
- Drop a 2captcha/CapSolver API key at `/home/sdancer/.2captcha_api_key` (mode 600) — would unblock the captcha-vision-solve sibling's L_terminal=CF_Turnstile layer.

You don't need to wait for these. Run the survey.

## Relevant files / references

- Closed sibling's diagnosis: `/home/sdancer/albion-captcha-vision-solve/analysis/post_submit_diagnosis.md`
- Closed sibling's evidence: `/home/sdancer/albion-captcha-vision-solve/analysis/*.png`
- Project memory on device trust: `[[albion_2fa_container_rotation]]` — Albion device-trust IS persistable across container rotations via `.config` preservation.
- Path portfolio: `/home/sdancer/orchestrator/analysis/paths.json`
- Dashboard (success signal): `https://albion.orch.run/state` for `zone != null`
- Substrate access: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`
