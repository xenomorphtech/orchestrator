# magic32-tcpdump-classify — tcpdump thered's outbound TCP + extract SNI hosts

## ONE TASK
Capture thered's outbound TCP traffic via on-device tcpdump for 60s during a fresh launch. Parse TLS ClientHello SNI to get target hostnames. Establish whether thered is even connecting to apis.netmarble.com (validating prior mitmproxy assumptions) and what other Netmarble-family hosts it touches. Write artifact, set fact, exit.

## CRITICAL — memory budget
**Hard limit 500 MB.** Pure pcap processing.

## Known state
- thered runs on Waydroid `localhost:5558`. Refresh PID via `pidof`.
- Prior 3 mitmproxy turns had **iptables NAT counters confirming packet redirection** (31 + 35 packets across turns), so SOME outbound TCP/443 IS happening — we just don't know to which hosts.
- All prior snapshot evidence (from cycle 121) pointed to `apis.netmarble.com` — but a fresh launch may have different patterns.

## SCRIPT to execute

```bash
set -e
WORKDIR=/home/sdancer/nmss-emu-magic32-tcpdump-classify
cd "$WORKDIR"

# Step 1: clean any prior tcpdump
adb shell 'su 0 pkill -f tcpdump' 2>&1 || true
sleep 1

# Step 2: start on-device tcpdump capturing TCP/443 + DNS to /sdcard
adb shell 'su 0 sh -c "rm -f /sdcard/thered_tcp_capture.pcap; nohup tcpdump -i any -s 0 -w /sdcard/thered_tcp_capture.pcap \"tcp port 443 or udp port 53\" > /dev/null 2>&1 &"'
sleep 3
adb shell 'su 0 pgrep tcpdump' 2>&1 | head -3

# Step 3: force-stop thered + relaunch
adb shell 'su 0 am force-stop com.netmarble.thered'
sleep 2
adb shell 'monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1' >/dev/null 2>&1
echo "relaunched at $(date); capturing 60s"
sleep 60

# Step 4: stop tcpdump + pull pcap
adb shell 'su 0 pkill -f tcpdump' 2>&1 || true
sleep 2
adb pull /sdcard/thered_tcp_capture.pcap captures/thered_tcp_capture.pcap 2>&1
ls -la captures/thered_tcp_capture.pcap

# Step 5: extract SNI hostnames + DNS queries via tshark or python
# Prefer tshark if available
if command -v tshark >/dev/null 2>&1; then
  echo "=== SNI hostnames ===" > /tmp/tcpdump_hosts.txt
  tshark -r captures/thered_tcp_capture.pcap -Y 'tls.handshake.type==1' -T fields -e tls.handshake.extensions_server_name 2>/dev/null | sort -u >> /tmp/tcpdump_hosts.txt
  echo "" >> /tmp/tcpdump_hosts.txt
  echo "=== DNS queries (A/AAAA) ===" >> /tmp/tcpdump_hosts.txt
  tshark -r captures/thered_tcp_capture.pcap -Y 'dns.qry.name' -T fields -e dns.qry.name 2>/dev/null | sort -u >> /tmp/tcpdump_hosts.txt
else
  # Fallback: scapy
  python3 -c "
from scapy.all import rdpcap, IP, TCP
import re
hosts = set()
dns_qs = set()
for p in rdpcap('captures/thered_tcp_capture.pcap'):
    raw = bytes(p)
    # SNI extraction (heuristic — looks for '\\x00\\x00\\x00\\x0bserver_name' marker)
    for m in re.finditer(rb'\\x00\\x00..\\x00..\\x00..([a-z0-9.-]{4,80})', raw):
        n = m.group(1).decode('ascii', 'ignore')
        if '.' in n and len(n) < 80:
            hosts.add(n)
print('=== SNI hostnames ===')
for h in sorted(hosts): print(h)
" > /tmp/tcpdump_hosts.txt 2>&1
fi
cat /tmp/tcpdump_hosts.txt

# Step 6: count unique TLS conversations by remote IP
echo "=== TLS conversations by remote IP+port ===" > /tmp/tcpdump_ips.txt
if command -v tshark >/dev/null 2>&1; then
  tshark -r captures/thered_tcp_capture.pcap -Y 'tcp.dstport==443' -T fields -e ip.dst -e tcp.dstport 2>/dev/null | sort -u >> /tmp/tcpdump_ips.txt
fi
cat /tmp/tcpdump_ips.txt

# Step 7: count netmarble matches
NETMARBLE_HOSTS=$(grep -i "netmarble\|nmgame\|nmsec\|netmarble" /tmp/tcpdump_hosts.txt | sort -u)
NM_COUNT=$(echo "$NETMARBLE_HOSTS" | grep -c .)
echo "Netmarble hosts: $NM_COUNT"
echo "$NETMARBLE_HOSTS"

# Step 8: artifact + fact
PCAP_SIZE=$(stat -c %s captures/thered_tcp_capture.pcap)
cat > $WORKDIR/analysis/tcpdump_classify_2026-05-14.md << ARTEOF
# tcpdump classify — thered outbound TCP destinations

## Setup
- on-device tcpdump on \`any\` interface, filter \`tcp port 443 or udp port 53\`
- 60s capture after fresh thered launch
- Pcap: $WORKDIR/captures/thered_tcp_capture.pcap ($PCAP_SIZE bytes)

## SNI + DNS hostnames seen
\`\`\`
$(cat /tmp/tcpdump_hosts.txt)
\`\`\`

## TLS remote IP:port conversations
\`\`\`
$(cat /tmp/tcpdump_ips.txt)
\`\`\`

## Netmarble-pattern hosts (n=$NM_COUNT)
\`\`\`
$NETMARBLE_HOSTS
\`\`\`
ARTEOF

if [ "$NM_COUNT" -gt 0 ]; then
  /home/sdancer/orchestrator/harness fact-set magic32_tcpdump_netmarble_hosts_$NM_COUNT "Captured $NM_COUNT Netmarble hostnames in thered's outbound TLS during 60s launch: $(echo $NETMARBLE_HOSTS | tr '\n' '|' | head -c 500). pcap at $WORKDIR/captures/thered_tcp_capture.pcap ($PCAP_SIZE bytes)."
else
  /home/sdancer/orchestrator/harness fact-set magic32_tcpdump_no_netmarble "0 Netmarble SNI hostnames in 60s launch window. Pcap captured $PCAP_SIZE bytes. Either thered didn't make TCP/443 traffic during window OR uses non-Netmarble hosts only OR launches don't fire Netmarble flows from cached state. Top hostnames: $(head -10 /tmp/tcpdump_hosts.txt | tr '\n' '|')"
fi

echo DONE
```

## Constraints
- ONE script run, ≤8 min wall time, ≤500 MB.
- tcpdump must be available on Waydroid (`su 0 which tcpdump`) — if not, install via apt or use `simpleperf` packet capture as fallback.
- tshark is preferred for SNI parsing; fallback to scapy is a regex hack and less reliable.

## Falsification (acceptable outcomes)
- `apis.netmarble.com` appears in SNI list → mitmproxy SHOULD have worked; debug deeper why DNAT'd packets didn't decrypt.
- Different Netmarble host (e.g., `mw.nmgame.net`) → that's the actual auth host; retarget mitmproxy reverse mode there.
- No Netmarble hosts → fresh launches don't fire auth; need to trigger an action (account-switch, settings tap) that re-auths.
- 0 traffic in 60s → thered not active; investigate launch flow.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-tcpdump-classify/`
- prior facts: `magic32_url_jwt_full_recovery_2026_05_14` (snapshot showed apis.netmarble.com), `magic32_mitmproxy_reverse_zero` (35 pkts to host:8082, 0 captured)
