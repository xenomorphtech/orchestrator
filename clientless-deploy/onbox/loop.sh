#!/usr/bin/env bash
export IS_SANDBOX=1
# Durable 5-min self-driving loop. Launch detached: setsid --fork bash ~/clientless/onbox/loop.sh
set -u
cd "$HOME/clientless" || exit 1
mkdir -p analysis
echo "loop start $(date -u +%FT%TZ) pid=$$" >> analysis/loop.log
while true; do
  timeout 280 bash "$HOME/clientless/onbox/tick.sh" || echo "tick timeout/err $(date -u +%FT%TZ)" >> analysis/loop.log
  python3 "$HOME/clientless/onbox/gt2paths.py" 2>/dev/null
  sleep 300
done
