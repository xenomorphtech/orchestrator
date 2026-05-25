# albion-binary-recon — extract login endpoint URLs from Albion binary

## Role & workdir
Fresh codex_app_server worker. Workdir: `/home/sdancer/albion-binary-recon`. NOT running on the live container — pure local analysis after dumping bytes.

## Goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** identify the HTTPS endpoint URLs Albion's launcher/client uses for: (a) credentials submission, (b) 2FA code submission, (c) session token issuance. Pure read-only intelligence gathering.

## Already established (do not re-investigate)
- All 5 synthesized-input substrates falsified against the 2FA modal (see `/home/sdancer/albion-prod-login/analysis/SUBSTRATE_BLOCKER_README.md`)
- Email-magic-link trust grant does not exist (see `/home/sdancer/albion-magic-link/analysis/candidate_links.json`)
- Unity prefs persist login.accountname + login.hash but NO Albion session/trust token
- Container egress IP: `45.13.105.18 (FR)`
- Albion binary install: `/home/albion/albion-online/Albion-Online` on remote container (ssh -p 14838 root@ssh8.vast.ai)

## Hypothesis
Albion's launcher binary (or supporting libraries in `/home/albion/albion-online/`) contains hardcoded URLs for its login API. Static analysis (`strings`, `grep`, `objdump`) can identify:
- Login submit URL (account+hash → session OR 2FA-required)
- 2FA verify URL (account+code → session OR error)
- Session token field name / cookie name
- API base URL pattern

This builds intelligence for a future autonomous session-injection path that bypasses the Unity dialog entirely.

## Falsification
Binary contains NO unique HTTPS URLs (only CDN/static asset URLs) OR all auth URLs are dynamically constructed at runtime (no hardcoded host strings).

## Tasks
1. **Enumerate binaries.** SSH to container, `ls -la /home/albion/albion-online/`. Identify the main executable + `Albion-Online_Data/` if Unity layout. Sizes + sha256s logged.

2. **Strings dump.** For each binary ≤200MB: `strings -n 8 <binary> | sort -u > /tmp/strings_<name>.txt`. SCP each to local `analysis/strings_<name>.txt`.

3. **URL grep.** Filter: `grep -aE 'https?://[a-zA-Z0-9./_-]+' analysis/strings_*.txt | sort -u > analysis/urls_all.txt`. Drop obvious assets (CDN, fonts, social media). Save filtered candidates to `analysis/urls_filtered.txt`.

4. **Auth-keyword scan.** Grep ALL strings (not just URLs) for: `login`, `auth`, `session`, `token`, `account`, `2fa`, `verify`, `device`, `signin`, `bearer`, `OAuth`, `code=`. Save signal hits to `analysis/auth_keywords.txt`.

5. **Cross-reference.** For each filtered URL, look for adjacent strings that suggest its purpose (request method, field names). Document in `analysis/login_api_inventory.md`.

6. **Bonus (only if time):** if `Albion-Online_Data/Managed/Assembly-CSharp.dll` exists (Unity .NET), try `monodis` or `ilspycmd` to decompile the LoginManager class names. NON-INVASIVE — local file analysis only.

7. **Append milestone** to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`: URL inventory summary + best-guess auth endpoint.

## Acceptance / closure
- HARD WIN: identified the exact `POST <URL>` that submits credentials. Documented field names + response format inferred from binary strings.
- SOFT WIN: at least one credible auth-domain URL captured (e.g., `https://api.albiononline.com/`, `https://auth.albiononline.com/`).
- FALSIFICATION: no unique auth-related URLs; suggests Albion uses dynamic URL construction or our binary lacks the launcher logic.

## Constraints (CRITICAL)
- **READ-ONLY ON CONTAINER.** Do NOT modify Albion binary, prefs, or any running process. SSH-execute `strings`, `ls`, `sha256sum`, `scp` only.
- **Do NOT restart Albion or any prod daemon.** Investigation is offline-style.
- **Do NOT commit binaries to git.** Strings outputs OK; raw binaries NEVER.
- **30 min time budget.** If binary too large or noise too high, falsification-cap is acceptable.

## Relevant files / refs
- Remote: `/home/albion/albion-online/Albion-Online` + sibling files
- SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Prior context: `/home/sdancer/albion-prod-login/analysis/SUBSTRATE_BLOCKER_README.md`
- Memory: `[[no-frida]]` — Albion is Linux Unity; Frida-on-Linux not memory-categorized; static RE is the safe alternative here.
