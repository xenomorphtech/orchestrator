#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIOME_URL="${BIOME_URL:-http://localhost:3021}"
BIOME_API_KEY_VALUE="${HARNESS_BIOME_API_KEY:-${BIOME_API_KEY:-}}"
TARGET_PANE="${TARGET_PANE:-orchestrator}"
COMMAND_TEXT="${COMMAND_TEXT:-\$orchestrate}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-300}"
ENTER_DELAY_MS="${ENTER_DELAY_MS:-150}"
SKILL_CONFIRM_DELAY_MS="${SKILL_CONFIRM_DELAY_MS:-300}"

log() {
  printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"
}

log_error() {
  printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >&2
}

biome_curl() {
  if [[ -n "$BIOME_API_KEY_VALUE" ]]; then
    curl -fsS -H "X-API-Key: $BIOME_API_KEY_VALUE" "$@"
  else
    curl -fsS "$@"
  fi
}

resolve_pane_id() {
  local panes_json pane_id

  if ! panes_json="$(biome_curl "$BIOME_URL/panes")"; then
    log_error "failed to query panes from '$BIOME_URL'"
    return 1
  fi

  if ! pane_id="$(printf '%s' "$panes_json" | python3 -c '
import json
import sys

target = sys.argv[1]
try:
    panes = json.load(sys.stdin)
except json.JSONDecodeError:
    sys.exit(2)

for pane in panes:
    if pane.get("name") == target or pane.get("id", "").startswith(target):
        print(pane["id"])
        break
' "$TARGET_PANE")"; then
    log_error "failed to parse panes response from '$BIOME_URL'"
    return 1
  fi

  printf '%s' "$pane_id"
}

send_input_b64() {
  local pane_id="$1"
  local payload_b64="$2"

  biome_curl -X POST "$BIOME_URL/panes/$pane_id/input" \
    -H 'Content-Type: application/json' \
    -d "{\"data\":\"$payload_b64\"}" >/dev/null
}

send_command() {
  local pane_id="${1:-}" text_b64 carriage_b64 newline_b64

  if [[ -z "$pane_id" ]]; then
    if ! pane_id="$(resolve_pane_id)"; then
      return 1
    fi
  fi

  if [[ -z "$pane_id" ]]; then
    log "skipped dispatch; pane '$TARGET_PANE' not found during send"
    return 1
  fi

  text_b64="$(printf '%s' "$COMMAND_TEXT" | base64 -w0)"
  carriage_b64="$(printf '\r' | base64 -w0)"
  newline_b64="$(printf '\n' | base64 -w0)"

  send_input_b64 "$pane_id" "$text_b64"
  sleep "$(awk "BEGIN { printf \"%.3f\", $ENTER_DELAY_MS / 1000 }")"
  send_input_b64 "$pane_id" "$carriage_b64"

  if [[ "$COMMAND_TEXT" == \$* ]]; then
    sleep "$(awk "BEGIN { printf \"%.3f\", $SKILL_CONFIRM_DELAY_MS / 1000 }")"
    send_input_b64 "$pane_id" "$carriage_b64"
    sleep "$(awk "BEGIN { printf \"%.3f\", $SKILL_CONFIRM_DELAY_MS / 1000 }")"
    send_input_b64 "$pane_id" "$carriage_b64"
    log "sent '$COMMAND_TEXT' to pane '$TARGET_PANE' with skill confirm and submit enters"
  else
    sleep "$(awk "BEGIN { printf \"%.3f\", $ENTER_DELAY_MS / 1000 }")"
    send_input_b64 "$pane_id" "$newline_b64"
    log "sent '$COMMAND_TEXT' to pane '$TARGET_PANE' with delayed carriage return and newline"
  fi
}

main() {
  local pane_id

  log "scheduler started for pane '$TARGET_PANE' every ${INTERVAL_SECONDS}s"

  while true; do
    if ! pane_id="$(resolve_pane_id)"; then
      log "skipped dispatch; failed to resolve pane '$TARGET_PANE'"
    elif [[ -z "$pane_id" ]]; then
      log "skipped dispatch; pane '$TARGET_PANE' not found"
    else
      send_command "$pane_id"
    fi

    sleep "$INTERVAL_SECONDS"
  done
}

main "$@"
