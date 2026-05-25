# thered-fresh-account — capture op901 wire (challenge, Token) on fresh-account install

## Role & workdir
Codex worker, workdir `/home/sdancer/nmss-emu-magic32-live-snapshot-replay`. Drive the cert-handshake capture pipeline on the RK3588 device at `adb localhost:5558`.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `lane-y3-wire-pair-capture-on-fresh-install`

## Current state (cycle 1898)
- Fresh Netmarble account REGISTERED on thered via WebView CDP (cycle 1865): `laney31779136243@wshu.net`, I_PID=`19EA6511F5DE482E98DAD71A48CF2AA7` (fact `lane_y_fresh_account_registered_2026_05_18`).
- Patch DOWNLOAD + APPLY + VERIFY complete (assets on disk, no redownload needed).
- Region SELECTED (Taiwan/HK/Macau, 繁體 zh-TW). Device on **VAMPIR title screen** (v1.8.12 REAL) showing "請輕觸畫面" (touch to continue) + "請選擇伺服器" (select server) with refresh icon.
- **PID 22738 still alive** (RSS 1.17GB). Last probe attempt cycle 1894 attached cleanly to 22738 with 6 BPs but ran 180s with ZERO samples because device wasn't past Select Region yet — now it IS past, cert call should fire on next tap.
- Cert function constants known install-stable from cycle 1842 probe: x2=0x0f53a8d2 x4=0x2db865a3 x5=0x902226d2 x6=0xb27e8e1e x7=0x3b7541a0 x8=x9=0xd95ca94a x10=0x1 (fact `lane_y_cert_constants_install_stable_2026_05_18`). Wire challenge bytes are the missing piece.
- Cert probe NOT CURRENTLY ARMED (prior attempt ended). Goal completion fact `lane_y3_wire_pair_captured_2026_05_18` UNSET (set only on real bytes per falsification discipline).
- 3 host-side tcpdumps (PIDs 18950/18962/18997 on device) running on `host 183.110.205.25 tcp 12000-12002` — output to `/data/local/tmp/lane_y3_live.pcap` (currently 20KB, all TCP keep-alives no payload).
- **CODEX_APP_SERVER WORKER SUBSTRATE BROKEN** (cycles 1893-1897, 5 turn failures with 'rollout items not found'). This briefing is for a CLAUDE PANE WORKER substitute. Pane substrate proven via vastai-albion-sonnet successful drive.
- Unity UI input WORKS via `adb shell input tap` — confirmed cycle 1896 (monkey injected, dialog appeared, Confirm tap at (1056,680) → title screen).

## Success criteria
- Single cert handshake fires at kit+0x198290 — observable in `/tmp/lane_y_capture.log` or `outputs/lane_x_trace_<ts>/lane_x_*.jsonl`.
- `/data/local/tmp/lane_y3_live.pcap` OR the probe-spawned pcap accumulates non-zero TCP **payload bytes** (NOT just keep-alives).
- Wire (challenge, Token) bytes extracted from op901 frame (PktLobbyNetmarbleSSecurityVerify).
- Fact `lane_y3_wire_pair_captured_2026_05_18` set with: capture timestamp, MAGIC32, challenge hex, Token hex, kit+0x198290 hit count.
- Single commit on branch `magic32-live-snapshot-replay`. Verdict at `analysis/lane_y3_wire_capture_verdict.md` (≤80 lines). Final line `LANE_Y3_WIRE_DONE`.

## Concrete tasks (do in order, BUDGET 5 cycles)

1. **Arm Lane X cert probe AGAINST PID 22738 (no force-stop)**:
   - PID 22738 is alive and at title screen. Need to attach probe WITHOUT killing it. Write a wrapper or directly invoke the probe binary:
     - First: re-read `/proc/22738/maps` via `adb shell su 0 cat /proc/22738/maps` to find largest `(deleted)` file mapping ≥4MB — that gives `KIT_BASE`.
     - Then BPs = `KIT_BASE + 0x198290, KIT_BASE + 0x1983cc, KIT_BASE + 0x19847c, KIT_BASE + 0x1984a0, KIT_BASE + 0x198540, KIT_BASE + 0x198564`.
     - Launch: `adb shell "su 0 nohup /data/local/tmp/lane_x_bl_trace --target-pid 22738 --bp-list <BPS_CSV> --jsonl /data/local/tmp/lane_y3_probe.jsonl --duration 300 > /data/local/tmp/lane_y3_probe.log 2>&1 &"`.
   - If PID 22738 dies during attach OR re-arm is hopeless: fall back to full `DURATION_S=300 ./tooling/capture_lane_x_trace.sh > /tmp/lane_y_capture.log 2>&1 &` (force-stop + relaunch + new PID, patch on disk so ~30s back to title).
   - **DO NOT pm clear.** Patch state on disk must be preserved.

2. **Drive UI past title screen**:
   - Tap center of "請輕觸畫面" (touch to continue): `adb -s localhost:5558 shell "input tap 960 540"`. On 1920×1080 the title-tap-to-continue is anywhere in the main area.
   - Or tap the "請選擇伺服器" / select-server button at bottom-center: approximately `(960, 720)` on 1920×1080.
   - Screencap after each tap to confirm transition.
   - Continue tapping through server select / character select / login if encountered — cert handshake (kit+0x198290) may fire on any of these transitions.

3. **Watch for cert fire + capture**:
   - Poll `/tmp/lane_y_capture.log` every 30s for `0x198290 fired`.
   - Poll `wc -c /data/local/tmp/lane_y3_live.pcap` for byte growth beyond 4KB.
   - Once cert fires, immediately `adb pull` the device pcap to host: `/home/sdancer/nmss-emu-magic32-live-snapshot-replay/outputs/lane_y3_<ts>/lane_y3.pcap`.

4. **Decode op901**:
   - PktLobbyNetmarbleSSecurityVerify is opcode 901. Walk the TCP stream and find the frame structure (length prefix + opcode + payload).
   - Extract challenge bytes (input to cert fn) and Token bytes (return value or response).

5. **Set fact + verdict**:
   - `harness fact-set lane_y3_wire_pair_captured_2026_05_18 "<ts>; magic32=<hex>; challenge_hex=<hex>; token_hex=<hex>; cert_fn_hits=N"`.
   - Verdict `analysis/lane_y3_wire_capture_verdict.md`. Single commit `lane-y3: capture op901 wire pair on fresh install`. Final line `LANE_Y3_WIRE_DONE`.

## Falsification criteria
- **kit+0x198290 does NOT fire** within 5 minutes of Global tap + post-tap navigation → probe attachment issue OR cert call path changed in patched build. Retire path with `lane_y3_cert_fn_unreached_<reason>.md` and propose alternative (e.g., probe at a sibling cert BLR site, or hook downstream OpenSSL TLS handshake).
- **Cert fires but pcap stays payload-empty** → cert is local-only (no wire emission), goal A is structurally unsupported by this opcode. Different op (e.g. op102 cpp-auth) needed.
- **App crashes on Global tap** → re-launch and retry; if 3x crashes, escalate.

## Constraints & gotchas
- **adb localhost:5558 ONLY.** No other device addresses.
- **NO Frida on libUnreal.so** (anti-cheat). Probe uses HW BP/uprobe substrate which is invisible to /proc/self/maps.
- **NO `pm clear`** — would invalidate MAGIC32 and force redownload of 8GB patch.
- **NO `am force-stop`** ideally (we want to preserve Select Region) — but if probe arming requires it, that's acceptable (patch on disk, relaunch fast).
- **Goal NEVER blocked** per [[goals-never-blocked]] — if one path fails, retire path and propose alternative.
- **Falsification discipline**: only set `lane_y3_wire_pair_captured` fact if REAL (challenge, Token) bytes are recovered from the wire. Cert fn fires alone are NOT sufficient — need the wire payload.

## Relevant files / references
- `tooling/capture_lane_x_trace.sh` — Lane X probe entry script.
- `analysis/lane_y_verdict.md` (commit 748bed3) — prior cert fn fire on similar install.
- `analysis/lane_y2_verdict.md` (commit d459725) — prior pcap analysis (24B→1836B all empty payload).
- Facts: `lane_y_fresh_account_registered_2026_05_18`, `lane_y_cert_constants_install_stable_2026_05_18`, `lane_y3_region_select_screen_2026_05_18`, `lane_y3_orchestrator_nudge_2026_05_18`.
