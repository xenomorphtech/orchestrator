# dark-december-protocol-live-dump — Live capture of in-game protocol traffic

## Role & workdir
Dynamic live-process analyst on Dark December's in-game protocol. Workdir: `/home/sdancer/dark-december-protocol-live-dump` (worktree of `/home/sdancer/dark-december`, branch `protocol-live-dump`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump` — recover Dark December's in-game protocol (the actual game RPC traffic: combat, inventory, chat, server state).
- Sub-goal (this path): **Live dynamic dump** — capture in-game protocol frames from a running process, since the lib is obfuscated and static disasm won't reach the on-the-wire shape efficiently.

## Success criteria
Closing fact: `dark_december_protocol_dumped`. Concrete deliverable:
- Captured raw frames of the in-game protocol (TCP/WebSocket) + decoded message inventory (opcodes, payload structures).
- Identification of the encryption scheme (if any) protecting the frames — and the session-key derivation point (cross-pollinates with `dark-december-hive-auth-trace`).
- Recommendation on whether protocol replay/forge is feasible offline.

## Progress so far (cross-pollination from sibling paths)

**Recon (done-success)** at `/home/sdancer/dark-december/analysis/recon_2026-05-15.md`:
- UE4 4.26.2 build (`RzGame`), INCA AppGuard/Hercules anticheat, Hive auth stack.
- APK: `dark-december-1.2.039.xapk` SHA256 `12a15315601eafb8314a1594e187213c0574b44602923661d6d10053b59577e0`.

**libcompatible-disasm (done-partial-falsification)** at `/home/sdancer/dark-december-libcompatible-disasm/analysis/`:
- libcompatible.so is the **anti-debug/protection layer**, NOT the auth crypto host.
- `asm_ptrace` dispatch table at `0x19e2b8` with 19 indirect xrefs (anti-debug glue).
- 3 named strings (rsaEncryption/oJWT/asm_ptrace) are OID/syscall metadata.

**hive-auth-trace (in flight)**: Java baksmali identifies Hive classes `com.hive.auth.AuthNetwork$LoginCenter`, `com.hive.protocol.UrlManager$Membership`, `com.hive.auth.AuthImpl` — auth endpoints traced; useful for understanding the **session-key handshake** that precedes the in-game protocol.

## Next 2–3 concrete tasks

1. **Install Dark December on Waydroid + capture network at the boundary.**
   - APK: `/home/sdancer/dark-december/apk/dark-december-1.2.039.xapk` (588 MB split-APK bundle). Install via `bundletool` or extract base.apk + split_config.apks and use `adb install-multiple`.
   - Start `tcpdump` on the Waydroid network interface BEFORE launching the game (root host shell): `sudo tcpdump -i any -w <workdir>/captures/raw_traffic.pcap host <game-server-ip>` (use a broader filter initially — `port not 80 and port not 443` to skip noise, then narrow).
   - Capture covers: app launch → auth handshake → game-world entry → some in-game actions. Save to pcap.

2. **Frida-on-Java hooks on socket I/O.** (Memory rule: Frida on Java is OK, Frida on libcompatible.so / libUE4.so is NOT — those have INCA AppGuard anti-frida.)
   - Use `xerda` (the Frida-on-Java binary per memory) to hook:
     - `java.net.Socket.connect`, `java.net.SocketOutputStream.write`, `java.net.SocketInputStream.read`
     - `okhttp3.RealCall.execute` and `okhttp3.RealWebSocket.send/onMessage` (if Hive uses OkHttp / WebSocket)
     - The Hive protocol's send/recv at the Java boundary (look for methods in `com.hive.*` returning byte[] or taking byte[]).
   - Capture: timestamp, direction (send/recv), length, hex dump (first 256 bytes), Java caller stack.

3. **Memory-inspect the running game for protocol structures.** With ptrace (NOT Frida on native libs):
   - Get live PID: `adb shell pidof com.needsgames.darkdecember` (or whatever the package id resolves to from the xapk; check `aapt dump badging` or `pm list packages`).
   - Look at `/proc/<pid>/maps` for libUE4.so + libcompatible.so + libgame.so load bases.
   - Scan heap for repeating protocol-frame headers (e.g. 4-byte length + 2-byte opcode + payload).
   - If WebSocket: search for `Sec-WebSocket-Key` and trace back.

4. **Write the artifact.** `<workdir>/analysis/live_protocol_dump_2026-05-15.md` with:
   - PCAP file location + frame count.
   - Decoded opcode/message inventory (at least 5 distinct messages).
   - Encryption scheme analysis (TLS only? custom-encrypted-over-TCP? per-message key?).
   - Session-key derivation pointer (cross-pollination with hive-auth-trace).
   - Replay feasibility verdict.

## Constraints & gotchas

- **No Frida on libcompatible.so / libUE4.so / any anticheat-protected native lib** — INCA AppGuard will detect it and crash/ban. Frida-on-Java only.
- **adb may briefly disconnect on Waydroid restart** — tolerate via `adb connect localhost:5558` retries.
- The xapk is a split-APK bundle. `aapt dump badging /home/sdancer/dark-december/apk/dark-december-1.2.039.xapk` may fail (only handles standard APKs); use `unzip -p ... base.apk` first to extract the base APK, then `aapt`.
- **Waydroid may share network with host** — your tcpdump on host should see the traffic. If it doesn't, the game may be using a VPN/local-proxy or Waydroid's `lxc-net` bridge — fall back to `tcpdump -i lxcbr0` or similar.
- This worker runs under systemd `harness-worker@dark-december-protocol-live-dump.service` in `system.slice` with MemoryMax=24G. PCAPs and Frida traces can be large — keep `<workdir>/captures/` clean.

## Relevant files / references

- xapk: `/home/sdancer/dark-december/apk/dark-december-1.2.039.xapk` (588 MB).
- Recon doc: `/home/sdancer/dark-december/analysis/recon_2026-05-15.md`.
- libcompatible recon: `/home/sdancer/dark-december-libcompatible-disasm/analysis/{libcompatible_disasm_2026-05-15.md, task2_xrefs_2026-05-15.md}`.
- Sibling Java trace (cross-pollinate): `/home/sdancer/dark-december-hive-auth-trace/` (in flight, may have Hive class map by the time you start).
- Tools: `tcpdump`, `wireshark` (or `tshark`), `aapt`, `unzip`, `bundletool`, `adb`, `xerda` (Frida-on-Java), `python3 (scapy or pyshark)`, `r2`, `strings`.
- Cross-pollination facts: `dark_december_recon_complete_2026_05_15`, `dark_december_libcompatible_retired_2026_05_15`, `dark_december_libcompatible_indirect_dispatch_2026_05_15`.

## Falsification

- xapk install fails on Waydroid (architecture / split-APK / signature mismatch) — escalate for a different APK source.
- Game launches but **never connects to a network server** (offline-only game in the recon was wrong) — reframe; this is a single-player title.
- All traffic is TLS-encrypted with proper cert-pinning AND the Java-side hooks don't reveal pre-encryption byte[] payloads → escalate to need-for-bypass cert-pinning approach.
- 3 cycles with no frames captured → retire.
