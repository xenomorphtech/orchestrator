#!/usr/bin/env bash
set -euo pipefail
portal_host="accountportal.albiononline.com"
portal_url="https://${portal_host}/"
fallback_url="https://albiononline.com/"
tmpdir="$(mktemp -d /tmp/accountportal-probe.XXXXXX)"
trap 'rm -rf "$tmpdir"' EXIT
doh_json="$(curl -sS "https://dns.google/resolve?name=${portal_host}&type=A")"
printf '%s\n' "$doh_json" > "$tmpdir/portal_dns.json"
dns_status="$(python3 - "$tmpdir/portal_dns.json" <<'PY'
import json
import sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    print(json.load(fh)["Status"])
PY
)"
echo "portal_host=${portal_host}"
echo "portal_url=${portal_url}"
echo "portal_dns_status=${dns_status}"
if [[ "$dns_status" != "0" ]]; then
  curl -sS -D "$tmpdir/fallback.headers" -o "$tmpdir/fallback.body" \
    "$fallback_url" || true
  fallback_status="$(awk '$1 ~ /^HTTP\// { code=$2 } END { print code }' \
    "$tmpdir/fallback.headers")"
  cf_mitigated="$(awk 'BEGIN{IGNORECASE=1} /^cf-mitigated:/ {print $2}' \
    "$tmpdir/fallback.headers" | tr -d '\r')"
  if rg -qi 'Just a moment|Enable JavaScript and cookies to continue' \
    "$tmpdir/fallback.body"; then
    fallback_body_class="cloudflare-challenge"
  else
    fallback_body_class="non-login-html"
  fi
  echo "fallback_url=${fallback_url}"
  echo "fallback_http_status=${fallback_status:-unknown}"
  echo "fallback_cf_mitigated=${cf_mitigated:-absent}"
  echo "fallback_body_class=${fallback_body_class}"
  echo "classification=endpoint-not-found"
  exit 3
fi
curl -sS -D "$tmpdir/portal.headers" -o "$tmpdir/portal.body" "$portal_url"
portal_status="$(awk '$1 ~ /^HTTP\// { code=$2 } END { print code }' \
  "$tmpdir/portal.headers")"
form_action="$(python3 - "$tmpdir/portal.body" <<'PY'
import re
import sys
body = open(sys.argv[1], "r", encoding="utf-8", errors="ignore").read()
match = re.search(r"<form[^>]+action=[\"']([^\"']+)", body, re.I)
print(match.group(1) if match else "")
PY
)"
echo "portal_http_status=${portal_status:-unknown}"
echo "portal_form_action=${form_action:-missing}"
echo "classification=$( [[ -n "$form_action" ]] && echo endpoint-discovered || echo endpoint-not-found )"
