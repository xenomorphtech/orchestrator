# proxy-resurrect — Find or rebuild the working Korean/HK proxy from vampir March-29 era

## Role & workdir
Recon worker. Workdir: `/home/sdancer/nmss-emu-proxy-resurrect`. Outbound HTTPS to known proxy hosts allowed for connectivity probing only.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `proxy-resurrect`

## Why this turn exists
Both attack paths converged on geo-gating from our DE IP (cycle 949). The 2026-03-29 vampir end-to-end success worked through a Korean proxy tunnel (fact `vampir.full_pipeline_working`). This turn locates that proxy's credentials/URL and tests whether it's still alive.

## Hypothesis
The Korean proxy URL (and credentials, if any) used by the 2026-03-29 vampir signup is recorded somewhere under `/home/sdancer/games/vampir/`, `/home/sdancer/games/autoproto/`, or in shell history / env files. It may or may not still be live, but recovering its identity is the prerequisite to either reuse or replace.

## Falsification (3 outcomes)
- (a) **Proxy URL + creds recovered AND still reachable** → fact `vampir_korean_proxy_<host>_live`. Sibling paths get unblocked next cycle.
- (b) **Proxy URL recovered but offline / 407** → fact `vampir_korean_proxy_<host>_offline`. User needs to provision a replacement.
- (c) **No proxy URL recorded anywhere local** → fact `vampir_korean_proxy_undocumented`. User has to provide one from scratch.

## Success criteria
**Primary deliverable**: `/home/sdancer/nmss-emu-proxy-resurrect/analysis/proxy_resurrect_2026-05-17.md` with:
1. **Search**: grep for `socks5`, `https_proxy`, `http_proxy`, `KOREAN_PROXY`, `KR_PROXY`, `HK_PROXY`, `proxy_host`, `proxy_url`, `\.kr/`, `\.hk/` across `/home/sdancer/games/vampir/`, `/home/sdancer/games/autoproto/`, `/home/sdancer/.config/`, `/home/sdancer/.env*` (max depth 3).
2. **History**: scan `~/.zsh_history`, `~/.bash_history` for proxy lines.
3. **For each candidate**: record the host, port, scheme, auth (mask password), source file/line.
4. **Connectivity probe** (≤5 GETs total): for each candidate, `curl --proxy <url> -o /dev/null -w "%{http_code}\n" --max-time 10 https://members.netmarble.com/auth?countryCode=HK 2>&1 | tail -1`. Record response code or timeout/cert/connect error.
5. **Verdict** matched to (a)/(b)/(c) above with closing fact via `harness fact-set`.

Final line: `PROXY_RESURRECT_DONE`.

## Constraints
- **Memory budget**: 256 MB. **Time budget**: 15 min wall.
- **No new proxy registration** — search-and-probe only.
- **Mask passwords** in any output. Truncate to first 4 / last 4 chars at most.
- **≤5 outbound probes total** (proxy reachability). 2-second pacing.

## Progress so far
- pdj8pyp3 blocked on HK/TW/MO SMS phone (cycle 947, fact `unrestriction_flow_drive_blocked_sms_only_release_type`)
- Fresh signup blocked on members.netmarble.com WebView HTTP failure from DE IP (cycle 949, fact `fresh_account_signup_failed_geo_gating_de_ip`)
- Both converge on need for HK/Korean egress IP

## Next 1 concrete task
Produce `analysis/proxy_resurrect_2026-05-17.md` per the success criteria above.

## Relevant files
- vampir worktree: `/home/sdancer/games/vampir/`
- autoproto: `/home/sdancer/games/autoproto/`
- existing nmss-emu config blob: `/home/sdancer/nmss-emu-magic32-strings/analysis/`
- prior proxy hint: cycle 938 `validate_gameserver_failure.json` recorded `proxied_url_probe: "407_proxy_authentication_required"` — that's the same stale proxy, the URL of which should be in the vampir code
- Harness: `/home/sdancer/orchestrator/harness`
