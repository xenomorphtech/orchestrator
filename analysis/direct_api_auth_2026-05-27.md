# Direct API Auth Probe - 2026-05-27

## Scope
This run tested the web-auth question directly from the orchestrator host and
avoided every launcher, X11, CDP, Frida, and vast.ai path that blocked the
prior seven sibling attempts.

Target:
- Can `account_2` from `/home/sdancer/albion-mobile-signup/secrets/account_2.json`
  authenticate against Albion's web-facing account infrastructure?

Repo / branch:
- `/home/sdancer/albion-direct-api-auth-accountportal`
- `albion-direct-api-auth-accountportal`

## Why this probe mattered
The c424->c458 siblings all failed before any real server-side auth decision.
They were substrate or launcher failures. This probe was the first one aimed at
the web surface with `curl` and `python`, so any result here is materially
closer to backend truth than the prior blocked runs.

## T-checkpoint note
This file was created during the mandated early-write checkpoint and finalized
after the scripted rerun completed.

## Outcome
Final classification:
- `endpoint-not-found`

Scripted evidence from `scripts/probe_accountportal_auth.sh`:
- `portal_dns_status=3`
- `fallback_http_status=403`
- `fallback_cf_mitigated=challenge`
- `fallback_body_class=cloudflare-challenge`
- script exit code: `3`

Meaning:
- The exact hostname named in the briefing does not currently exist in public
  DNS.
- The fallback main site is challenge-gated and does not expose a simple login
  form to a non-browser `curl` probe.
- No credential POST was attempted because there was no reachable accountportal
  host to submit to.
- No session token or cookie was obtained.

## Endpoint discovery
Primary discovery target:
- `GET https://accountportal.albiononline.com/`

Observed result from the orchestrator host:
- `curl -sSI https://accountportal.albiononline.com/`
- failure before TLS or HTTP:
  `curl: (6) Could not resolve host: accountportal.albiononline.com`

Resolver cross-checks:
- Python `socket.getaddrinfo` returned
  `gaierror(-2, 'Name or service not known')` for
  `accountportal.albiononline.com`
- the same Python check resolved `albiononline.com`
- public DoH confirmed the host absence:
  - `https://dns.google/resolve?name=accountportal.albiononline.com&type=A`
  - `https://cloudflare-dns.com/dns-query?name=accountportal.albiononline.com&type=A`
  - both returned `Status: 3` (`NXDOMAIN`)

Discovery conclusion:
- The briefed accountportal hostname does not currently resolve, so no login
  HTML, POST action URL, field names, CSRF token, or auth cookie flow can be
  extracted from that host on 2026-05-27.

## Fallback comparison
Fallback pivot:
- `GET https://albiononline.com/`

Observed response:
- HTTP `403`
- `server: cloudflare`
- `cf-mitigated: challenge`
- `content-type: text/html; charset=UTF-8`

Body classification:
- Cloudflare interstitial headed `Just a moment...`
- requires JavaScript and cookies to continue
- no plain username/password form visible to `curl`

Interpretation:
- `endpoint-requires-jsdom` applies to the main landing page
- that fallback does not repair the missing `accountportal` hostname

## Final interpretation
This run does not falsify `account_2` credentials themselves. It falsifies the
availability of the named `accountportal.albiononline.com` path as a usable
direct web-auth probe on 2026-05-27. The most defensible mechanism label is
`endpoint-not-found`, with a secondary note that the main site is Cloudflare-
challenged for non-browser traffic.

## Artifact
Implemented:
- `scripts/probe_accountportal_auth.sh`

Behavior:
- queries Google DoH for `accountportal.albiononline.com`
- classifies non-zero DNS status as host absence
- fetches `https://albiononline.com/` for comparison
- records Cloudflare challenge characteristics
- exits before any credential use when the target host is unavailable

Safety:
- no password is printed
- no password is submitted in the `endpoint-not-found` path
- no token exists to redact
