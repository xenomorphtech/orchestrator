# dark-december-handshake-capture — fresh :10001 capture from SYN + full decode pass

## Role & workdir
Live capture + protocol decoder. Workdir: `/home/sdancer/dark-december-handshake-capture` (worktree of `/home/sdancer/dark-december`, branch `handshake-capture`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump`.
- Sub-goal: capture the **full** :10001 game-server session **from TCP SYN forward** including login/enter-world handshake (NOT mid-session like sibling `dark-december-live-capture`). Then decode the body obfuscation using the handshake's known-structure prefixes.

## Cross-pollination facts
- `dark_december_ingame_socket_captured` (live-capture closure): framing solved. `u32_le total_length + u16=0x0001 + body`. Body has XOR-style repeated-byte runs.
- `dark_december_protocol_10001_observed_2026_05_14`: 8-byte heartbeats + activity frames (41 s→c / 45 c→s).
- `dark_december_anti_frida_detected_2026_05_14`: NO Frida on DD. Use on-device tcpdump only.
- `dark_december_guest_auth_api_live_2026_05_14`: Hive login already succeeded once on this device (logged-in session is live as of cycle 71).

## Live state pre-dispatch
- com.needsgames.darkdecember alive PIDs 19306/19328/19334.
- Existing :10001 to 130.162.236.3 ESTABLISHED but app backgrounded.
- Hive credentials cached (Hive login succeeded once via external user action).

## Success criteria
Concrete deliverable:
- `:10001` pcap that includes the **TCP SYN → SYN-ACK → ACK → first application bytes** sequence.
- Decoded handshake bytes interpreted: session key delivery / opcodes / IDs.
- Either:
  - Body decoded for at least 5 distinct frames (sets `dark_december_protocol_body_decoded`), OR
  - Clear evidence that bodies use session keys negotiated in :443 traffic (sets `dark_december_protocol_session_keys_in_https`).

## Next concrete tasks (single long turn, 3h budget)

1. **Start ALL captures BEFORE touching the app.**
   ```bash
   # Use -U (packet-buffered, write immediately) and snap full packets.
   adb shell "nohup tcpdump -i any -s 0 -U -w /sdcard/Download/dd_handshake_full_$(date +%s).pcap 'not port 53' > /dev/null 2>&1 &"
   adb shell "nohup tcpdump -i any -s 0 -U -w /sdcard/Download/dd_handshake_10001_$(date +%s).pcap 'port 10001' > /dev/null 2>&1 &"
   adb shell "nohup tcpdump -i any -s 0 -U -w /sdcard/Download/dd_handshake_443_$(date +%s).pcap 'port 443' > /dev/null 2>&1 &"
   # Verify
   adb shell 'pgrep -f tcpdump | wc -l'  # should be >=3
   ```
   - Wait 5 seconds for tcpdump to settle.
   - Note: prior `live-capture` worker had a Waydroid bug where narrow `port 10001` filter on `-i any` returned 0 bytes when broad capture also ran. Workaround: stop the prior broad-pcap tcpdump first (`adb shell pkill -f dd_live`), THEN start fresh, OR rely on extracting :10001 from the broad pcap (as live-capture did).

2. **Force a fresh login flow.**
   - Soft-kill DD: `adb shell am force-stop com.needsgames.darkdecember`
   - Wait 3s.
   - Relaunch: `adb shell am start -n com.needsgames.darkdecember/com.epicgames.ue4.GameActivity`
   - This drops the existing :10001 socket (verify with netstat).
   - The app should auto-progress through cached Hive auth (logged-in state preserved per the prior login).
   - If Hive WebView prompts for credentials AGAIN (cached token expired), the user is supposed to be available — but if not, document the fall-back: pull `/data/data/com.needsgames.darkdecember/shared_prefs/` to see what Hive auth state survived.
   - Drive past any title screens with `adb shell input tap $((W/2)) $((H/2))` and screenshot every 5s to track progress.

3. **Watch for SYN → first :10001 packet.**
   - `adb shell netstat -tn` — watch for fresh ESTABLISHED to 130.162.236.3:10001 (different local port than 40466).
   - Once ESTABLISHED, screenshot, then idle ~30s for heartbeats.
   - Drive ONE in-game action (tap somewhere, swipe) to generate at least one 45-byte client frame.
   - Total capture window: ~3 minutes from launch.

4. **Stop captures, pull pcaps.**
   ```bash
   adb shell pkill -f "dd_handshake"
   sleep 2
   adb pull /sdcard/Download/dd_handshake_10001_*.pcap captures/
   adb pull /sdcard/Download/dd_handshake_full_*.pcap captures/
   adb pull /sdcard/Download/dd_handshake_443_*.pcap captures/
   ```

5. **Handshake analysis.**
   - `tshark -r captures/dd_handshake_10001_*.pcap -Y 'tcp.flags.syn == 1' -V | head -30` — confirm SYN captured.
   - `tshark -r captures/dd_handshake_10001_*.pcap -Y 'tcp.port == 10001 and tcp.len > 0' -T fields -e frame.time_relative -e ip.src -e ip.dst -e tcp.len -e tcp.payload | head -20` — list all application-payload packets in order.
   - First server→client packet AFTER 3-way handshake — that's the server hello / session-start. Likely contains:
     - A larger frame than the 8-byte heartbeat (session key material, server-side state).
     - Possibly a server-generated nonce/timestamp.
   - First client→server packet — likely contains the client's session token / auth blob / version stamp.

6. **Body decoding with handshake context.**
   - Combine handshake-pcap frames with the prior live-capture's 47 frames.
   - **Key insight**: the FIRST frame after handshake usually has known plaintext structure (e.g. "GAME_INIT" magic + version + auth token). XOR(first_frame_body, known_plaintext_prefix) recovers the cipher's first N bytes of keystream.
   - With cipher's first-N-byte keystream, decrypt all subsequent frame headers (which are likely opcode + length etc.).
   - Workflow:
     - For each candidate cipher mode (XOR-with-rotating-key, XOR-with-positional-key derived from session key, RC4 with session key, AES-CTR with session-key):
       - Apply to first frame body. Check if result has structure.
     - If a session key is needed, look at :443 traffic for a `session/start` API call that returns hex/base64 keys.
   - Apply recovered cipher to ALL 47+ frames; document the decoded opcodes/payloads.

7. **Write artifact** `analysis/handshake_capture_decode_2026-05-14.md`:
   - The captured handshake bytes (raw hex + decoded).
   - The cipher recovered (algorithm + key derivation).
   - Per-direction frame opcode inventory.
   - Python script that decodes any frame given the session context.
   - Set fact `dark_december_protocol_body_decoded` on full success, OR `dark_december_handshake_captured_body_partial` on partial.

## Constraints & gotchas
- DO NOT use Frida (confirmed detected on DD).
- On-device tcpdump is kernel-level passive → AC-invisible.
- The Waydroid `-i any` multi-tcpdump bug: if narrow :10001 capture is 0 bytes, extract :10001 frames from the broad pcap instead.
- Hive WebView might re-prompt for credentials if cached token expired during the elapsed time. If so: take screenshot, document, escalate to user only as last resort.
- This worker runs under systemd `harness-worker@dark-december-handshake-capture.service`.

## Relevant files / references
- Prior :10001 pcaps: `/home/sdancer/dark-december-live-capture/captures/dd_live_10001_*.pcap`, `dd_live_full_*.pcap`.
- Prior closure artifact: `/home/sdancer/dark-december-live-capture/analysis/live_protocol_capture_2026-05-14.md`.
- Hive class map (for :443 cross-correlate): `/home/sdancer/dark-december-hive-auth-trace/analysis/task1_hive_class_location_2026-05-14.md`.
- Tools: `tcpdump`, `tshark`, `python3 (scapy, pyshark)`, `adb`.

## Falsification
- Hive WebView re-prompts AND no cached token → escalate to user for credential.
- Capture succeeds but first :10001 application bytes are noise even with the SYN context → bodies are pre-encrypted before transport (rare).
- After 2-3h analysis: no decoded plaintext → bodies use session keys delivered via :443 TLS that we can't decrypt without bypassing TLS-pinning → falls back to needing session-key extraction from running app (would require defeating Hercules).
