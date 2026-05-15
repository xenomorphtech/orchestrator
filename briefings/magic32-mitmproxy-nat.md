# magic32-mitmproxy-nat — Transparent MITM via iptables NAT redirect on Waydroid

## ONE TASK
thered's HTTP client doesn't honor Android global http_proxy. Use iptables NAT inside Waydroid to redirect thered's outbound TCP/443 → host's mitmproxy in transparent mode. Capture plaintext, write artifact, set fact, exit.

## CRITICAL — memory budget
**Hard limit 500 MB.** No large-binary disasm. Pure network setup work.

## Known state
- CA already installed at `/data/misc/keychain/cacerts-added/c8750f0d.0` on Waydroid (from prior turn).
- Mitmproxy is installed (prior turn dispatched it).
- thered PID rotates; current expected ~28677 but refresh via `pidof`.
- thered uses libUnreal-bundled BoringSSL with hidden symbols; ignores `settings put global http_proxy`.
- Host IP from device's perspective: `192.168.2.1` (prior turn confirmed reachable).

## SCRIPT to execute

```bash
set -e
WORKDIR=/home/sdancer/nmss-emu-magic32-mitmproxy-capture  # reuse existing worktree
cd "$WORKDIR"
MITM_PORT=8082
HOST_IP=192.168.2.1

# Step 1: kill any lingering mitm + start mitmdump in TRANSPARENT mode
pkill -f "mitmdump.*--mode transparent" 2>/dev/null || true
pkill -f "mitmdump.*$MITM_PORT" 2>/dev/null || true
sleep 1

MITMDUMP=$(which mitmdump || echo $HOME/.local/bin/mitmdump)
# Transparent mode: mitmdump intercepts TLS based on SNI without needing client proxy config
$MITMDUMP --mode transparent --listen-host 0.0.0.0 --listen-port $MITM_PORT --set ssl_insecure=true -w captures/thered_nat_flows.mitm --set confdir=$WORKDIR/.mitmproxy &
MITM_PID=$!
sleep 4
ss -tlnp 2>/dev/null | grep ":$MITM_PORT" | head -2
echo "mitmdump transparent PID=$MITM_PID"

# Step 2: clear any prior proxy setting (it was useless and may interfere)
adb shell "settings put global http_proxy :0" 2>&1 || true

# Step 3: add iptables NAT rule inside Waydroid to redirect outbound TCP/443 → host:$MITM_PORT
# Note: device must have iptables; Waydroid runs Android, which has it built-in
adb shell "su 0 iptables -t nat -L OUTPUT --line-numbers" 2>&1 | head -5

# Remove any stale rule from prior runs
adb shell "su 0 iptables -t nat -D OUTPUT -p tcp --dport 443 -j DNAT --to-destination $HOST_IP:$MITM_PORT 2>/dev/null" || true

# Install the redirect rule
adb shell "su 0 iptables -t nat -A OUTPUT -p tcp --dport 443 -j DNAT --to-destination $HOST_IP:$MITM_PORT" 2>&1

# Verify
adb shell "su 0 iptables -t nat -L OUTPUT -n -v" 2>&1 | head -20

# Step 4: relaunch thered + wait for traffic
adb shell "su 0 am force-stop com.netmarble.thered" 2>&1 || true
sleep 2
adb shell 'monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1' >/dev/null 2>&1
echo "thered relaunched at $(date); waiting 45s for traffic"
sleep 45

# Step 5: stop mitmdump + clean up iptables
adb shell "su 0 iptables -t nat -D OUTPUT -p tcp --dport 443 -j DNAT --to-destination $HOST_IP:$MITM_PORT" 2>&1 || true
kill $MITM_PID 2>/dev/null || true
sleep 2

# Step 6: inspect captures
FLOW_SIZE=$(stat -c %s captures/thered_nat_flows.mitm 2>/dev/null || echo 0)
echo "thered_nat_flows.mitm size: $FLOW_SIZE bytes"
$MITMDUMP -nr captures/thered_nat_flows.mitm 2>&1 | head -50 > /tmp/mitm_nat_summary.txt
cat /tmp/mitm_nat_summary.txt

grep -E "netmarble|cpp-auth|identity|push/v1|userKey=|googleAuthCode" /tmp/mitm_nat_summary.txt > /tmp/mitm_nat_netmarble.txt
NMC=$(wc -l < /tmp/mitm_nat_netmarble.txt)
echo "netmarble flows: $NMC"
head -20 /tmp/mitm_nat_netmarble.txt

# Step 7: artifact + fact
cat > $WORKDIR/analysis/mitmproxy_nat_capture_2026-05-14.md << ARTEOF
# mitmproxy transparent NAT — thered HTTPS capture (Turn 2)

## Setup
- mitmdump in --mode transparent on $HOST_IP:$MITM_PORT
- iptables NAT rule on Waydroid: OUTPUT tcp/443 → $HOST_IP:$MITM_PORT
- thered force-stopped + relaunched
- CA from prior turn: /data/misc/keychain/cacerts-added/c8750f0d.0
- 45 second capture window

## Results
- thered_nat_flows.mitm size: $FLOW_SIZE bytes
- Netmarble-related flow count: $NMC

## Flow summary
\`\`\`
$(cat /tmp/mitm_nat_summary.txt)
\`\`\`

## Netmarble flows
\`\`\`
$(cat /tmp/mitm_nat_netmarble.txt)
\`\`\`
ARTEOF

if [ "$NMC" -gt 0 ]; then
  /home/sdancer/orchestrator/harness fact-set magic32_mitmproxy_nat_captured_$NMC "$NMC Netmarble flows captured via iptables NAT + mitmdump transparent mode. Capture file $WORKDIR/captures/thered_nat_flows.mitm ($FLOW_SIZE bytes). Sample URLs: $(head -3 /tmp/mitm_nat_netmarble.txt | tr '\n' '|')"
elif [ "$FLOW_SIZE" -gt 0 ]; then
  /home/sdancer/orchestrator/harness fact-set magic32_mitmproxy_nat_non_netmarble_only "Some flows captured ($FLOW_SIZE bytes) but no Netmarble matches in 45s window. Either thered traffic too short, or apps targeted other hosts. Full summary in artifact."
else
  /home/sdancer/orchestrator/harness fact-set magic32_mitmproxy_nat_zero_traffic "mitmdump transparent + iptables NAT installed but 0 bytes captured. Either iptables rule didn't apply (verify with adb shell iptables -t nat -L OUTPUT -n -v during capture), or transparent-mode requires additional handshake support, or app uses cert pinning that breaks the TLS handshake silently. Next: try mitmweb interactive + look at handshake errors."
fi
echo DONE
```

## Constraints
- ONE script run. ≤8 min wall time.
- Memory ≤500 MB.
- Do NOT redo the CA install — already done.
- Do NOT use `--mode regular` or `settings put http_proxy` — those were prior-turn approaches that failed.

## Falsification (acceptable outcomes)
- ≥1 Netmarble flow plaintext captured → success, fact `magic32_mitmproxy_nat_captured_<N>` + URLs/bodies in artifact.
- Some traffic captured but no Netmarble → fact `magic32_mitmproxy_nat_non_netmarble_only`; document hosts seen.
- 0 traffic → fact `magic32_mitmproxy_nat_zero_traffic`; check iptables rule packet counter (`iptables -t nat -L OUTPUT -n -v` shows pkt count) to distinguish "rule didn't match" from "transparent mode rejected".

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-mitmproxy-capture/` (reused)
- prior artifact: `analysis/mitmproxy_capture_2026-05-14.md` (Turn 1 explicit-proxy attempt)
- known endpoints to look for: `apis.netmarble.com/*`, `userKey=`, `googleAuthCode`
