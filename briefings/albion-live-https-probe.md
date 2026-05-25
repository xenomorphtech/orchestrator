# albion-live-https-probe — enumerate API surface of live.albiononline.com:443

## Role & workdir
Fresh Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-live-https-probe`. Live target: `https://live.albiononline.com/` (92.223.84.84:443).

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **enumerate the HTTPS API surface of `live.albiononline.com:443` to find an auth/account/oauth path that Albion's runtime contacts during startup, which can be probed directly to obtain refresh-token-equivalent credentials**.

## Big-picture rationale
Cycle-3303 tcpdump capture revealed Albion contacts **`live.albiononline.com:443`** during startup (TLS ClientHello at t=137s, repeated at t=137.9s). This endpoint was NEVER surfaced by any static binary RE (cycles 3210-3268 binary recon, cycle-3263 launcher-args audit, cycle-3268 UnityPlayer/boot.config/GameAssembly audit all missed it). The cycle-3304 tcpdump worker probed the bare URL and got 404/403 — but only tested top-level paths.

The actual auth/account API behind `live.albiononline.com` may use:
- `/v1/...`, `/v2/...` (versioned REST API)
- `/api/...` (common API prefix)
- `/account`, `/account/login`, `/accountportal`
- `/oauth`, `/oauth/authorize`, `/oauth/token`
- `/launcher`, `/launcher/auth`
- `/signin`, `/login`, `/authenticate`
- `/refresh-token`, `/exchange-code`
- Path patterns based on Photon Realtime conventions (e.g., `/photonlogin`)

A successful API hit would reveal the protocol surface. Even 401 responses (instead of 404/403) tell us *which paths exist*.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `/home/sdancer/albion-tcpdump-runtime/analysis/runtime_tls_client_hellos.txt` (c3303) | live.albiononline.com:443 contacted at startup t=137s | ✅ DONE |
| 2 | `/home/sdancer/albion-tcpdump-runtime/analysis/tcpdump_verdict.md` (c3304) | Bare URL probes returned 404/403; deeper paths untested | ✅ DONE |
| 3 | DNS: live.albiononline.com → 92.223.84.84 (single A record) | Single-host endpoint, not load-balanced | ✅ DONE |
| 4 | Albion has 4 login modes: `password`, `refreshToken`, `exchangecode`, `accountportal` (c3212 strings) | API likely exposes endpoints for each | ✅ DONE |
| 5 | `albion-token-watcher.service` armed on LOCAL orchestrator host | Captures autonomous tokens if any zone-change | ✅ DONE |

## Success criteria
1. **Surface mapping**: identify ≥3 paths under `live.albiononline.com:443` that return non-404. Anything returning 401/403/400/405 (instead of 404) means the path EXISTS but requires auth/method/headers.
2. **Auth-flow identification**: identify which path corresponds to the `password`/`refreshToken`/`exchangecode`/`accountportal` login modes. If any path returns a JSON error that names a known login-mode string, that's a candidate.
3. **Token capture (stretch)**: if a `password` POST endpoint accepts the stored credentials and returns a refresh/access token in response, save to `secrets/refresh_token.json` (mode 600, gitignored).
4. **End-to-end verification**: if a token is captured, write it to the Albion-expected on-disk location, restart Albion, observe `/state` for zone-change.
5. **Verdict**: `analysis/live_https_api_surface.md` with Achievement-levels-+-gaps framing. Level 5 = self.zone non-null without manual 2FA.

## Tasks (sequential, ~2-3h hard cap)

### Task 1 — sense baseline + enumerate common paths
1. From this orchestrator host (NOT the vast.ai container — `live.albiononline.com:443` is a public-internet endpoint), use `curl -sv` to probe common paths. Capture full status code + response headers + first 1000 bytes of body for each:
   - `/`
   - `/v1`, `/v1/`, `/v2`, `/v2/`
   - `/api`, `/api/`, `/api/v1`, `/api/v2`
   - `/launcher`, `/launcher/`
   - `/account`, `/account/login`, `/accountportal`, `/account-portal`
   - `/oauth`, `/oauth/authorize`, `/oauth/token`
   - `/signin`, `/login`, `/auth`, `/authenticate`
   - `/refresh`, `/refresh-token`, `/exchange`, `/exchangecode`
   - `/status`, `/health`, `/.well-known/openid-configuration`, `/robots.txt`
   - `/photonlogin`, `/photon`, `/realtime`
2. Save matrix of (path, status, content-type, body-head, redirect-target) to `analysis/path_probe_matrix.tsv`.
3. Anything ≠ 404 is a finding. Especially watch for: 401, 403 (path exists but needs auth/perms), 405 (path exists but wrong method), 400 (path exists but wrong payload), 200 (UNREDACTED success), 3xx (redirect to actual API).

### Task 2 — methods + headers expansion
For every path that returns ≠ 404 from Task 1, retry with:
- `OPTIONS` (reveals allowed methods via `Allow:` header)
- `POST` with `Content-Type: application/json` and empty body
- `POST` with the stored credentials: `{"email":"5fswkv6zf4@wshu.net","password":"albion260518q9"}` (and variants: `{"username":...}`, `{"accountname":...}`, etc.)
- With `User-Agent: Albion-Online/...` (mimic actual client — check the TLS ClientHello in `runtime_tls_client_hellos.txt` for ALPN hints)

Save to `analysis/method_probe_matrix.tsv`.

### Task 3 — TLS ClientHello + ALPN analysis
1. Decode the Albion client's actual TLS ClientHello from the pcap at `/home/sdancer/albion-tcpdump-runtime/analysis/albion_startup.pcap.gz`:
   ```bash
   tshark -r albion_startup.pcap -Y 'tls.handshake.type==1 and ip.dst==92.223.84.84' \
     -T fields -e tls.handshake.extensions_server_name \
     -e tls.handshake.extensions.alpn_str \
     -e tls.handshake.cipher_suites
   ```
2. Use those exact handshake details when crafting probes — server may reject "wrong" clients.
3. Pull cert chain + check if any cert SAN reveals additional hostnames: `openssl s_client -connect 92.223.84.84:443 -servername live.albiononline.com | openssl x509 -text`. Save to `analysis/tls_cert_chain.txt`.

### Task 4 — verdict
Write `analysis/live_https_api_surface.md` with Achievement-levels-+-gaps framing:
- Level 1: paths enumerated
- Level 2: ≥3 non-404 paths identified
- Level 3: auth-flow path identified (path that recognizes login-mode strings)
- Level 4: credentials successfully posted → received a token
- Level 5: token wrote to disk + Albion restart → zone non-null

If Level 5 hits → fact `albion_live_https_auth_unblock_2026_05_22 = <path>` + milestone in talk channel. If Level 3-only, that's still major progress — document the path for future direct-POST experiment.

## Constraints & gotchas
- **Run probes from THIS orchestrator host**, not the vast.ai container. live.albiononline.com is public.
- **Rate-limit: max 5 requests/second**. Server may have abuse protection. Sleep 200ms between probes.
- **DO NOT** post real credentials more than 5 times — could trigger account lockout. Use a fake-credential dry run first to see if the endpoint even accepts the JSON structure.
- **DO NOT commit credentials or tokens** to git. `secrets/` is gitignored.
- **Token-watcher must remain armed** on LOCAL orchestrator host (already running): `systemctl is-active albion-token-watcher.service` → `active`.
- **No Frida, no ptrace, no LD_PRELOAD-touching-Albion** — but this path doesn't touch Albion's process anyway, just talks to live.albiononline.com:443.
- **One worker per path**: tcpdump-runtime is now closed-partial; you own this path alone.
- **Production daemons stay healthy**: cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest, Albion-Online — but this work doesn't touch them either.

## Relevant files / references
- pcap evidence (read-only): `/home/sdancer/albion-tcpdump-runtime/analysis/albion_startup.pcap.gz`
- TLS captures: `/home/sdancer/albion-tcpdump-runtime/analysis/runtime_tls_client_hellos.txt`
- DNS answers: `/home/sdancer/albion-tcpdump-runtime/analysis/runtime_dns_answers.txt`
- Previous accountportal-CDP work (refer back): `/home/sdancer/albion-accountportal-cdp/analysis/accountportal_flow_verdict.md`
- Previous binary-recon strings: `/home/sdancer/albion-binary-recon/analysis/strings_global_metadata.txt`
- Account credentials (read-only): `/home/albion/.albion_credentials.txt` (on vast.ai container; copy via ssh)
- Sister-path watcher (DO NOT TOUCH): LOCAL host `albion-token-watcher.service`
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Memory pointers: `[[albion-2fa-container-rotation]]`, `[[orchestrator-role]]`, `[[macromanage-workers]]`.
- Facts: `albion_runtime_https_live_endpoint_2026_05_22`, `albion_tcpdump_endpoints_discovered_2026_05_22`.

## Reporting
Concise progress at each task boundary. If a non-404 path is discovered → milestone + fact. If credentials post yields a token → milestone + secret save + Albion restart + zone-change verification. Achievement-levels-+-gaps throughout. Hard cap 2-3h.
