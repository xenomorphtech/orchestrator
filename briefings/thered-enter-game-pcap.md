# thered-enter-game-pcap — capture device's conn#2 + opcode 901 → 902

## Role & workdir
Pcap+decode worker. Workdir: `/home/sdancer/nmss-emu-device-webview-signin`. Target: thered (PID 28576 right now) on RK3588 at `adb -s localhost:5558`.

## Why this turn exists
The lobby+GS PktLogin chain is PROVEN end-to-end (fact `thered_clientless_chain_proven_2026_05_17`). Replay via SOCKS5 yields `PktLoginResult Result=0`. The remaining 0.05/1 of the goal is **opcode 901 (PktLobbyNetmarbleSSecurityVerify) on conn#2 → 902 response**. Our SOCKS5 chain is blocked on the cert oracle (`162.244.80.97:9876` connection refused) AND we don't have a session.json for the current PID `87BCB7…`.

The decoupled move: **let the device itself send op901** — capture the real frame in a tcpdump, decode it, see the cert structure, and answer the writeup's open question: is the cert challenge = PID, or is it a server-issued nonce from one of op8/op27/op102?

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `thered-conn2-pcap`

## Device state right now (verify, do NOT assume)
- thered PID 28576 alive
- ONE ESTABLISHED GS conn at `192.168.2.2:47284 ↔ 183.110.40.52:12000` (Hector 2 GS) — heartbeat-only, login already happened
- Device screen at "Class Selection" — character creation, **NO character exists on this account**
- Prior turn (cycle 1019-1020) captured `conn2.pcap` (12.6KB, 132 frames, 60+s) on this alive conn — only opcode 16 heartbeats, NO op901 observed (login transitioned before tcpdump). Verdict in `conn2_cert_analysis.md`.

The cert-capture goal blocked because we missed the conn#1→conn#2 transition. **This briefing now adds a NEW path:** force-stop + relaunch to capture a FRESH login including op901 on conn#2.

## Hard rules
- **adb target: localhost:5558 only.** NOT Waydroid.
- **NO Frida, NO LKM, NO kprobe.** Just tcpdump + vampir decoder. The chain works at the byte layer already.
- **NEVER tap "Create" / "Create Character".** It's destructive state. If a character already exists, tap into the existing one.
- **NEVER clear thered app data.**
- Verify each tap with a fresh screenshot before deciding the next action.
- 30 min wall cap. 512 MB RSS cap. Read-only on existing prior artifacts.

## Step 1 — confirm device state
```bash
adb -s localhost:5558 shell screencap -p > analysis/artifacts/conn2_screen_00_before.png
adb -s localhost:5558 shell "ss -tn 2>/dev/null | grep -E '12000|183\.110' | head -20"
adb -s localhost:5558 shell "pidof com.netmarble.thered"
```
Verify thered is running + you see at least one ESTAB to a 183.110.* host on port 12000 (lobby or GS).

## Step 2 — start broad tcpdump (≤30 s)
```bash
adb -s localhost:5558 shell "su 0 sh -c '
  pkill -INT tcpdump 2>/dev/null; sleep 1
  nohup tcpdump -i any -s 0 -w /sdcard/conn2.pcap \
    \"(net 183.110.0.0/16) and port 12000\" \
    > /sdcard/tcpdump_conn2.log 2>&1 &
  echo TCPDUMP_PID=\$!
'"
sleep 2
adb -s localhost:5558 shell "su 0 ls -la /sdcard/conn2.pcap"
```
The wider filter `net 183.110.0.0/16` catches any GS in the 40.x or 50.x ranges if the assignment shifts.

## Step 3 — FORCE-STOP and RELAUNCH to trigger a fresh login (NEW path, cycle 1020)
Account has no character, so we can't drive UI deeper. Instead, force a fresh login cycle. The CLI command is **process-level only** — it kills the running app process, **does not clear app data or credentials**. The user explicitly allowed this distinction: "Do NOT clear thered app data" — that refers to `pm clear`, NOT `am force-stop`.

```bash
# tcpdump is already running from Step 2 with broad filter — keep it running
# Confirm thered is alive
adb -s localhost:5558 shell "pidof com.netmarble.thered"  # expect a PID

# Force-stop the app process (NOT 'pm clear' — keep data)
adb -s localhost:5558 shell "am force-stop com.netmarble.thered"
sleep 2

# Verify process is gone
adb -s localhost:5558 shell "pidof com.netmarble.thered"  # expect empty

# Re-launch thered's main launcher activity
adb -s localhost:5558 shell "monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1"
sleep 5

# Verify it's back
adb -s localhost:5558 shell "pidof com.netmarble.thered"  # expect new PID

# Take screenshot
adb -s localhost:5558 shell screencap -p > analysis/artifacts/conn2_screen_relaunch.png
```

After relaunch, the app will go through its full login flow (lobby Version → ServerListRead → close → GS conn#1 PktVersion+PktLogin → close → GS conn#2 op901 SecurityVerify). Wait **60–90 s** of network activity before stopping tcpdump.

If the app shows "Please tap the screen" after relaunch:
```bash
adb -s localhost:5558 shell input tap 960 540
sleep 3
adb -s localhost:5558 shell screencap -p > analysis/artifacts/conn2_screen_post_tap.png
```
Just enough to push it past the title — do NOT tap Create / Class Selection if it shows up again. The op901 traffic will already have flowed by the time the UI reaches character select.

## Step 4 — stop tcpdump, pull pcap (≤30 s)
```bash
adb -s localhost:5558 shell "su 0 pkill -INT tcpdump; sleep 1; ls -la /sdcard/conn2.pcap"
adb -s localhost:5558 pull /sdcard/conn2.pcap analysis/artifacts/conn2.pcap
```

## Step 5 — extract + decode
```bash
tshark -r analysis/artifacts/conn2.pcap \
  -Y "tcp.flags.push==1 and (tcp.dstport==12000 or tcp.srcport==12000) and tcp.len>0" \
  -T fields -e frame.number -e ip.src -e ip.dst -e tcp.srcport -e tcp.dstport -e tcp.payload \
  > analysis/artifacts/conn2_payloads.tsv
```

Then a python decoder (reuse the pattern from cycle 1006 that produced `enter_game_pktlogin_decoded.json`):
- vampir framing: `[u16 LE total_len][XOR_offset0(0x9A + crc32_LE + opcode_LE(u16) + seq_LE(u16) + payload)]`
- XOR key: `9A A7 84 20 D0 C9 78 B3`
- For S→C frames: read flags byte; if `flags & 0x80` then LZ4-decompress body after flag
- For each frame, emit `{frame, src→dst:port, opcode, payload_len, payload_hex}`
- For opcode 901 (C→S to 183.110.40.x:12000): try to decode the schema — `(Token: fstring, Log: fstring)` — and dump `Token` as the cert candidate
- For opcode 902 (S→C from 183.110.40.x:12000): dump payload as Result + any post-fields

Save **`analysis/artifacts/conn2_decoded.jsonl`** — one record per packet (both C→S and S→C). Also save a focused **`analysis/artifacts/conn2_op901_extracted.json`** with just the opcode 901 frame and its 902 reply.

## Step 6 — answer the writeup's open question
Compare the **cert bytes** captured from op901 against:
1. The device's PID `87BCB74629734B9BAF2D948A4B8823E7` (and its lowercase / hex-decoded form)
2. The device's conn#1 Token `9253DD6691CE2E35` (or whatever fresh Token the device's conn#1 produced this turn — also decode it from conn2.pcap)
3. Each of op8/op27/op102 payloads from prior `replay_pktlogin.json` (`c0270900...`, zeros, etc.)

If the cert is a literal/derivative of one of these → record the function and we're 5/5 with no oracle.
If the cert is a 48-char hex that doesn't match any → confirms the cert is genuinely the cycle-71 SHA-256-based multi-phase function, needing oracle or session.json.

Save **`analysis/artifacts/conn2_cert_analysis.md`** — short doc, ≤60 lines, with the comparison and verdict.

## Outputs
- `analysis/artifacts/conn2.pcap`
- `analysis/artifacts/conn2_payloads.tsv`
- `analysis/artifacts/conn2_decoded.jsonl`
- `analysis/artifacts/conn2_op901_extracted.json`
- `analysis/artifacts/conn2_cert_analysis.md`
- Screenshots `conn2_screen_NN.png`
- Facts on completion:
  - `thered_device_conn2_op901_captured_2026_05_17` = true (with source_ref)
  - `thered_op901_cert_shape_2026_05_17` = one-line description of the cert structure observed
  - If 902 was received with success result: `thered_device_op901_902_witnessed_2026_05_17` = true
- Final line on the codex turn: `CONN2_PCAP_DONE`

## Reference files
- vampir test-wire: `/home/sdancer/games/vampir/proxy-rs/target/release/test-wire`
- vampir XOR key + framing: `proxy-rs/src/wire/framing.rs`
- vampir registry: `protocol-rs/src/registry/mod.rs:407-408` (`PktLobbyNetmarbleSSecurityVerify=901, …Result=902`)
- Prior proven artifacts:
  - `analysis/artifacts/replay_pktlogin.json` (SOCKS5 conn#1 success)
  - `analysis/artifacts/enter_game_pktlogin_decoded.json` (frame 49 = real device PktLogin)
  - `analysis/THERED_CLIENTLESS_CHAIN.md` (writeup)
