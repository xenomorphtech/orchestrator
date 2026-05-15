# dark-december-live-capture — live game-server traffic capture + encryption analysis

## Role & workdir
Live pcap analyst. Workdir: `/home/sdancer/dark-december-live-capture` (worktree of `/home/sdancer/dark-december`, branch `live-capture`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump`.
- Sub-goal: **the game is logged in NOW**. Capture live `:10001` traffic, determine encryption scheme, decode protocol framing.

## Live state (confirmed pre-dispatch by orchestrator)
- Game PIDs: `19306 19328 19334` (com.needsgames.darkdecember).
- Active connection: `192.168.2.2:40466 → 130.162.236.3:10001` ESTABLISHED (game-server).
- Numerous TLS connections on :443 (api/Hive/CDN traffic).
- Time-sensitive: capture before the session disconnects.

## Success criteria
Concrete deliverable: `<workdir>/analysis/live_protocol_capture_2026-05-14.md` documenting:
- Active pcap path + frame count.
- Per-flow summary (5-tuple, packet count, byte volume).
- **Encryption verdict**: is `:10001` traffic TLS-wrapped, raw-binary-encrypted, length-prefixed-JSON, or something else?
- If readable: opcode/JSON-frame inventory (5+ distinct message types).
- If encrypted: framing analysis (header size, length-prefix shape, MAC bytes), and best guess at the algorithm.
- Set fact `dark_december_ingame_socket_captured` on success.

## Cross-pollination facts
- `dark_december_socket_layer_plain_json_2026_05_15`: sibling Hive-auth-trace work HYPOTHESIZED WS frames are plain JSON over TLS. Confirm or refute here.
- `dark_december_hive_http_keys_known_2026_05_15`: Hive HTTP wrapper crypto = AES-128-CBC zero-IV, key = SHA1/SHA256(timestamp) first 16 hex chars UTF-8.
- `dark_december_anti_frida_detected_2026_05_14`: no Frida on Dark December. Use ONLY: on-device tcpdump (passive, kernel-level, AC-invisible) + offline pcap analysis.

## Next concrete tasks (single long turn, 3h budget)

1. **Verify app + connection still live.**
   - `adb connect localhost:5558`
   - `adb shell pidof com.needsgames.darkdecember` — must be non-empty.
   - `adb shell netstat -tn | grep 10001` — must show ESTABLISHED. If not, capture is futile — escalate.

2. **Start tcpdump captures NOW (priority: don't miss session).**
   ```bash
   adb shell "nohup tcpdump -i any -w /sdcard/Download/dd_live_full_$(date +%s).pcap -G 600 -W 30 'not port 53' > /dev/null 2>&1 &"
   adb shell "nohup tcpdump -i any -w /sdcard/Download/dd_live_10001_$(date +%s).pcap port 10001 > /dev/null 2>&1 &"
   adb shell "nohup tcpdump -i any -w /sdcard/Download/dd_live_443_$(date +%s).pcap port 443 -G 600 -W 30 > /dev/null 2>&1 &"
   ```
   - Verify all 3 tcpdump PIDs alive: `adb shell pgrep tcpdump`.

3. **Drive in-game activity to generate traffic.**
   - Take baseline screenshot.
   - Tap center (`adb shell input tap $((W/2)) $((H/2))`) — generate generic input events.
   - If a menu is visible: try `adb shell input keyevent KEYCODE_BACK` and `KEYCODE_MENU` to open inventory/menu.
   - Let it idle 60s (heartbeat traffic).
   - Try moving (swipe gestures via `adb shell input swipe X1 Y1 X2 Y2`) — game-action traffic.
   - Accumulate at least 3-5 minutes of varied activity.

4. **Pull pcaps to host.**
   ```bash
   adb shell ls -l /sdcard/Download/dd_live_*.pcap
   adb pull /sdcard/Download/dd_live_10001_*.pcap captures/
   adb pull /sdcard/Download/dd_live_full_*.pcap captures/
   ```

5. **Encryption verdict — TLS vs raw.**
   - Run `tshark -r captures/dd_live_10001_*.pcap -Y 'tcp.port == 10001' -V | head -200` and look at the first few bytes of payload after handshake:
     - If `16 03 0X` at the start → TLS ClientHello → traffic is TLS-wrapped.
     - If `<4-byte length> + <opcode-byte> + <body>` shape → custom binary protocol.
     - If readable ASCII → plain JSON or HTTP.
   - Count flow direction: `tshark -r captures/dd_live_10001_*.pcap -Y 'tcp.port == 10001' -T fields -e ip.src -e ip.dst -e tcp.len | sort | uniq -c | sort -rn | head -20`.
   - Get raw bytes of the first 5 distinct client→server packets and first 5 server→client packets:
     ```bash
     tshark -r captures/dd_live_10001_*.pcap -Y 'tcp.port == 10001 and tcp.len > 0' -T fields -e frame.number -e ip.src -e tcp.payload | head -20
     ```

6. **If plain text or length-prefixed (NOT TLS)**: extract message body bytes, attempt JSON parse on each frame after stripping the length prefix. Build opcode/message-type inventory.

7. **If TLS-wrapped**: confirm cert subject (`tshark ... -Y 'tls.handshake.type == 11' -V`), note SNI, note ALPN. Then this is structurally similar to Hive's TLS+JSON layer; document that decoding the bodies requires session-key extraction (out of scope for this turn).

8. **Cross-correlate with :443 tcpdump** — does the game make :443 calls right before connecting to :10001 that look like session-key delivery (e.g., HTTPS POST to a `/session/start` endpoint with auth tokens)? Mark those endpoints.

9. **Write artifact** `<workdir>/analysis/live_protocol_capture_2026-05-14.md` with everything above + set fact `dark_december_ingame_socket_captured` on success.

## Constraints & gotchas
- **No Frida on Dark December** (confirmed: spawn-owned xerda kills the process). On-device tcpdump is kernel-level and invisible to userspace AC.
- adb root + SELinux permissive confirmed.
- Game-server IP `130.162.236.3` (Oracle Cloud netblock — Korean publisher hosting).
- Time-sensitive: pcap should be growing every minute; if it's not growing after 60s, the connection may have closed (re-check netstat).
- Don't touch the game UI in disruptive ways — risk of disconnect.

## Falsification
- Connection closes before useful payload captured → re-attempt next cycle.
- Traffic is fully TLS-encrypted with no readable bodies → document the metadata + framing; protocol body decryption is a separate sub-goal needing session-key extraction.
- App detects host-side capture and kills the socket (rare for tcpdump — it's passive — but flag if observed).

## Relevant files / references
- patch-driver closing artifact: `/home/sdancer/dark-december-patch-driver/analysis/ingame_protocol_capture_2026-05-14.md`.
- hive-auth-trace artifacts for HTTP wrapper crypto: `/home/sdancer/dark-december-hive-auth-trace/analysis/`.
- Tools: `tcpdump`, `tshark`, `wireshark`, `python3 (scapy, pyshark)`.
