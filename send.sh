#!/usr/bin/env bash
# Send text to a biome_term pane with automatic Enter.
# Usage: send.sh <pane-id-or-name> <message>
# Example: send.sh native_harness "Continue."
# Example: send.sh ecf38525 "Continue."

set -euo pipefail

BIOME_URL="${BIOME_URL:-http://localhost:3021}"
BIOME_API_KEY_VALUE="${HARNESS_BIOME_API_KEY:-${BIOME_API_KEY:-}}"

target="$1"
shift
msg="$*"

biome_curl() {
  if [[ -n "$BIOME_API_KEY_VALUE" ]]; then
    curl -s -H "X-API-Key: $BIOME_API_KEY_VALUE" "$@"
  else
    curl -s "$@"
  fi
}

# Resolve name to pane id if needed (not a UUID pattern)
if [[ ! "$target" =~ ^[0-9a-f]{8} ]]; then
  pane_id=$(biome_curl "$BIOME_URL/panes" | python3 -c "
import json,sys
for p in json.load(sys.stdin):
    if p.get('name') == '$target':
        print(p['id']); break
" 2>/dev/null)
  if [[ -z "$pane_id" ]]; then
    echo "error: no pane named '$target'" >&2; exit 1
  fi
else
  # Allow prefix match
  pane_id=$(biome_curl "$BIOME_URL/panes" | python3 -c "
import json,sys
for p in json.load(sys.stdin):
    if p['id'].startswith('$target'):
        print(p['id']); break
" 2>/dev/null)
  if [[ -z "$pane_id" ]]; then
    echo "error: no pane matching '$target'" >&2; exit 1
  fi
fi

# Send text first, then Enter separately after a short delay.
# Codex TUI needs the Enter as a distinct input event.
b64_text=$(printf '%s' "$msg" | base64 -w0)
b64_enter=$(printf '\r' | base64 -w0)

biome_curl -X POST "$BIOME_URL/panes/$pane_id/input" \
  -H 'Content-Type: application/json' \
  -d "{\"data\":\"$b64_text\"}"

sleep 0.15

biome_curl -X POST "$BIOME_URL/panes/$pane_id/input" \
  -H 'Content-Type: application/json' \
  -d "{\"data\":\"$b64_enter\"}"

echo "sent to $pane_id"
