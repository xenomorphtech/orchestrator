# magic32-mitmproxy-capture — Turn 3: mitmdump REVERSE mode for apis.netmarble.com

## ONE TASK
Prior turn's iptables NAT inside Waydroid works (31 pkts matched) but mitmproxy transparent mode can't read SO_ORIGINAL_DST when NAT happens in the device namespace. Switch mitmdump to `--mode reverse:https://apis.netmarble.com:443` — it'll proxy every incoming connection AS IF the target were apis.netmarble.com, which is exactly what we want for the issue endpoint. Capture, write artifact, set fact, exit.

## CRITICAL — memory budget
**Hard limit 500 MB.** Pure network setup. No analysis-heavy operations.

## Known state from prior turns
- CA installed at `/data/misc/keychain/cacerts-added/c8750f0d.0` on Waydroid (works).
- iptables NAT rule on Waydroid `OUTPUT -p tcp --dport 443 -j DNAT --to-destination 192.168.2.1:8082` matched 31 pkts last turn (mechanism works for routing).
- mitmproxy `--mode transparent` failed to decrypt because SO_ORIGINAL_DST returns local sock (host:8082) not original (apis.netmarble.com:443) when DNAT happens in Waydroid namespace.

## SCRIPT to execute

```bash
set -e
WORKDIR=/home/sdancer/nmss-emu-magic32-mitmproxy-capture
cd "$WORKDIR"
MITM_PORT=8082
HOST_IP=192.168.2.1

# Step 1: clean any prior mitmdump
pkill -f "mitmdump.*mode" 2>/dev/null || true
sleep 1

MITMDUMP=$(which mitmdump || echo $HOME/.local/bin/mitmdump)

# Step 2: launch mitmdump in REVERSE mode → apis.netmarble.com
# Every TCP connection to host:8082 will be proxied AS IF the target were apis.netmarble.com:443.
# mitmproxy generates a cert for "apis.netmarble.com" (our CA, already trusted on device).
$MITMDUMP --mode reverse:https://apis.netmarble.com:443 --listen-host 0.0.0.0 --listen-port $MITM_PORT --set ssl_insecure=true -w captures/thered_reverse_flows.mitm --set confdir=$WORKDIR/.mitmproxy 2>/tmp/mitm_stderr.log &
MITM_PID=$!
sleep 4
ss -tlnp 2>/dev/null | grep ":$MITM_PORT" | head -2
echo "mitmdump reverse PID=$MITM_PID"

# Step 3: re-install the Waydroid iptables NAT rule (it was cleaned up at end of last turn)
adb shell "su 0 iptables -t nat -D OUTPUT -p tcp --dport 443 -j DNAT --to-destination $HOST_IP:$MITM_PORT 2>/dev/null" || true
adb shell "su 0 iptables -t nat -A OUTPUT -p tcp --dport 443 -j DNAT --to-destination $HOST_IP:$MITM_PORT" 2>&1

# Step 4: snapshot rule counter pre-launch
adb shell "su 0 iptables -t nat -L OUTPUT -n -v" 2>&1 | head -10 > /tmp/iptables_pre.txt
cat /tmp/iptables_pre.txt

# Step 5: relaunch thered + wait
adb shell "su 0 am force-stop com.netmarble.thered" 2>&1 || true
sleep 2
adb shell 'monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1' >/dev/null 2>&1
echo "thered relaunched at $(date); waiting 45s for traffic"
sleep 45

# Step 6: snapshot rule counter post-launch + tear down
adb shell "su 0 iptables -t nat -L OUTPUT -n -v" 2>&1 | head -10 > /tmp/iptables_post.txt
cat /tmp/iptables_post.txt
adb shell "su 0 iptables -t nat -D OUTPUT -p tcp --dport 443 -j DNAT --to-destination $HOST_IP:$MITM_PORT" 2>&1 || true
kill $MITM_PID 2>/dev/null || true
sleep 2

# Step 7: inspect captures + stderr
FLOW_SIZE=$(stat -c %s captures/thered_reverse_flows.mitm 2>/dev/null || echo 0)
echo "thered_reverse_flows.mitm size: $FLOW_SIZE bytes"
$MITMDUMP -nr captures/thered_reverse_flows.mitm 2>&1 | head -60 > /tmp/mitm_summary.txt
cat /tmp/mitm_summary.txt
echo "--- mitm stderr (last 30 lines) ---"
tail -30 /tmp/mitm_stderr.log

# Step 8: count netmarble-shaped flows
grep -E "GET|POST|PUT|netmarble|cpp-auth|identity|push|userKey|googleAuthCode" /tmp/mitm_summary.txt > /tmp/mitm_netmarble.txt
NMC=$(wc -l < /tmp/mitm_netmarble.txt)
echo "netmarble flows: $NMC"
cat /tmp/mitm_netmarble.txt

# Step 9: artifact + fact
cat > $WORKDIR/analysis/mitmproxy_reverse_capture_2026-05-14.md << ARTEOF
# mitmproxy reverse-mode capture (Turn 3)

## Setup
- mitmdump --mode reverse:https://apis.netmarble.com:443 on $HOST_IP:$MITM_PORT
- iptables NAT on Waydroid: OUTPUT tcp/443 → $HOST_IP:$MITM_PORT
- 45s capture window after thered relaunch

## iptables counters
Pre:
\`\`\`
$(cat /tmp/iptables_pre.txt)
\`\`\`
Post:
\`\`\`
$(cat /tmp/iptables_post.txt)
\`\`\`

## Capture
- thered_reverse_flows.mitm: $FLOW_SIZE bytes
- netmarble flows: $NMC

## Flow summary
\`\`\`
$(cat /tmp/mitm_summary.txt)
\`\`\`

## mitm stderr tail
\`\`\`
$(tail -20 /tmp/mitm_stderr.log)
\`\`\`
ARTEOF

if [ "$NMC" -gt 0 ]; then
  /home/sdancer/orchestrator/harness fact-set magic32_mitmproxy_reverse_captured_$NMC "$NMC apis.netmarble.com flows decrypted via mitmdump reverse mode + Waydroid iptables DNAT. Capture: $WORKDIR/captures/thered_reverse_flows.mitm ($FLOW_SIZE bytes). Sample: $(head -3 /tmp/mitm_netmarble.txt | tr '\n' '|')"
elif [ "$FLOW_SIZE" -gt 100 ]; then
  /home/sdancer/orchestrator/harness fact-set magic32_mitmproxy_reverse_flows_no_netmarble "Reverse mode captured $FLOW_SIZE bytes but no Netmarble-pattern matches. Either traffic not for apis.netmarble.com or capture window missed it. Check artifact for actual hostnames hit."
else
  /home/sdancer/orchestrator/harness fact-set magic32_mitmproxy_reverse_zero "Reverse mode + iptables DNAT yielded $FLOW_SIZE byte capture. Check stderr tail in artifact — likely TLS handshake errors (cert pinning) OR client closes connection seeing wrong cert SAN. Next fallback: LKM library injection per standing rule."
fi

echo DONE
```

## After the script
ONE artifact `analysis/mitmproxy_reverse_capture_2026-05-14.md` + ONE fact set + exit.

## Constraints
- ONE script run, ≤8 min wall time, ≤500 MB.
- Do NOT redo CA install.
- Do NOT touch host-level iptables (only Waydroid-side).

## Falsification (acceptable outcomes)
- ≥1 Netmarble flow captured → success, URL + body in artifact, fact set.
- Bytes captured but no Netmarble matches → wrong host targeting; document what was seen.
- 0 bytes captured AND stderr shows TLS handshake errors → real cert pinning by thered's BoringSSL; next: LKM library injection.
- 0 bytes captured AND stderr clean → app didn't make TCP/443 traffic in window (Status=10 PGS gate + no other traffic); extend capture window.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-mitmproxy-capture/`
- prior artifacts: `analysis/mitmproxy_capture_2026-05-14.md` (Turn 1), `analysis/mitmproxy_nat_capture_2026-05-14.md` (Turn 2)
- mitmproxy stderr will reveal TLS errors if they happen
