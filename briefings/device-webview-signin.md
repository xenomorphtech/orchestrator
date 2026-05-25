# device-webview-signin — Drive on-device WebView signin + complete clientless chain

## Role & workdir
On-device integration worker. Workdir: `/home/sdancer/nmss-emu-device-webview-signin`. ADB on localhost:5558 (rk3588_s, Android 12, root via `adb shell su 0 <cmd>`). All Netmarble HTTPS goes out of the device's own network (already HK, no proxy needed).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `device-webview-signin`

## Terminal success criterion
GS accepts a security packet (opcode 901 → 902) on the GS TCP connection for a freshly-created clientless account (`rplovhfdkm@wshu.net`, NetmarbleId `N82eef0101a9043599eecea053951e90`). Success fact: `nmss_fresh_account_clientless_login_complete`.

## Why this turn exists
Off-device fetch-player attempts have all returned `errorCode 1100` (10 probes with JSON Members-API fields, 2 with snapshot-derived shapes). The fetch-player call is what links a fresh Netmarble account to thered's playerId space. **Instead of reverse-engineering that call's shape, drive thered's own SDK on-device to make the call for us**, then read the resulting playerId from disk and feed it into vampir's already-working sign-in→lobby→GS pipeline.

## Hypothesis
thered's in-app WebView accepts CDP control (webview_devtools_remote_11954 is up). Driving a fresh signin via the WebView causes the on-device Netmarble SDK to call fetch-player itself, which writes the fresh playerId to `/data/data/com.netmarble.thered/shared_prefs/cpp_native_shared.xml` as `I_PID`. Reading that file gives us the fresh playerId; vampir's `/cpp-auth/v1/sign-in` JSON path then completes the chain.

## Falsification (4 outcomes)
- (a) **Fresh signin via WebView CDP succeeds, I_PID updates, full chain completes (sign-in→lobby→GS→901→902)** → fact `nmss_fresh_account_clientless_login_complete`. **Goal 5/5.**
- (b) **Fresh signin succeeds, I_PID updates, but downstream fails** → fact `clientless_chain_failure_<stage>_<short>`. Goal at ~4/5. Diagnose the new gap.
- (c) **WebView CDP cannot drive signin** (page navigates wrong, react setter trick fails, captcha blocks) → fact `webview_signin_blocked_<reason>`. Document. Fall back to live-pcap path (uprobe SSL_write on libssl in thered process).
- (d) **Phase 0 baseline (chain with device's existing I_PID) fails before fresh signin is attempted** → the pipeline itself is broken; cannot blame fetch-player. Document and stop.

## Phases — execute in order, gate each on the prior outcome

### Phase 0: Baseline — verify chain works with the device's CURRENT I_PID (≤10 min)
- **CYCLE-965 UPDATE**: First attempt (phase0_baseline.json) used `deviceKey=7B0D26CDC87D42EA` (16 hex chars = I_UDID), got `errorCode 1210 invalid device key`. The 16-char I_UDID is the **nmDeviceKey** (matches vampir NM_DEVICE_KEY constant). The 32-char **deviceKey** is the top-level `<string name="UDID">` in cpp_native_shared.xml: `0361D9069F8941EC947CA31E6DAD4029`.
- Retry command (write artifact to `analysis/artifacts/phase0_baseline_v2.json`):
  ```
  python3 /home/sdancer/games/vampir/create_account/get_stoken.py --pid 87BCB74629734B9BAF2D948A4B8823E7 \
      --nm-device-key 7B0D26CDC87D42EA --device-key 0361D9069F8941EC947CA31E6DAD4029 --country-code HK
  ```
- If sign-in returns 200 with `restriction == []` and a valid SToken → write artifact `analysis/artifacts/phase0_baseline_v2.json`, set fact `phase0_stoken_ok_2026_05_17`.
- If 1210 again, try the values from `<string name="UDID">` AND vampir's hardcoded fallback `DEVICE_KEY = "0FE7218207D84888BDA3442EDE4B3E5A"` (get_stoken.py:59). Document each attempt in artifacts/.
- Then run `validate_gameserver.py` against that SToken. If lobby `Result: 0` and GS `Result: 0` → chain is intact.
- **If Phase 0 v2 fails too**: STOP. Don't attempt fresh signin until the device-key gap is understood.

### Phase 0.7: Fix vampir's hardcoded LoginPlatform (≤10 min, gated on Phase 0 v2 success)
**Cycle 967 hypothesis**: PktLobbyLogin times out because vampir's `validate_gameserver.py:305` hardcodes `pack_u32(1)` for LoginPlatform, but `login_config_example.json` + `CommonLogJson.I_ChannelType` say **33** = Google Play. Vampir's 2026-03-29 run worked when test accounts were registered as channel 1; the device's current account-87BCB7 is channel 33 (per I_ChannelType in cpp_native_shared.xml). Server likely silent-drops on channel mismatch.
- **Do NOT modify the central vampir codebase.** Copy `validate_gameserver.py` and `get_stoken.py` into the worker's workdir, patch the local copy.
- Patch line 305 (in lobby_login): change `pack_u32(1) + pack_u32(0)` to `pack_u32(33) + pack_u32(0)`.
- Re-run the chain end-to-end with the existing device PID 87BCB74629734B9BAF2D948A4B8823E7 and deviceKey 0361D9069F8941EC947CA31E6DAD4029.
- Save the new lobby trace to `analysis/artifacts/phase0_7_lobby_login_channel33.json` with the PktLobbyLogin Result and any session Token.
- **Outcomes**:
  - PktLobbyLogin Result=0 with session Token → set fact `device_lobby_login_ok_channel33`. PROCEED to Phase 0.8.
  - Still timeout → try LoginPlatform=20 (the I_NMNIDChannelType from xml) and LoginPlatform=100 (I_NMChannelCode) as a 2-shot fallback. If all 3 fail → falsify (b) lobby-login-timeout-persists; do NOT go to Phase 2.

### Phase 0.8: Full chain with channel-fixed lobby (≤10 min, gated on 0.7)
- Run validate_gameserver.py.lobby_login → game_server_login with cert from cert-rust-repro using PID 87BCB7... as MAGIC32.
- Save final response trace; set fact `device_chain_complete_existing_pid` if 901→902 received. **This proves chain integrity end-to-end with an existing account** (not satisfying the fresh-account criterion yet, but unblocks Phase 1+2+3).
- Then proceed to Phase 1 (CDP discovery) to swap in the fresh account.

### Phase 1.12: Capture thered "entering the game" with corrected constants — CYCLE 996
**Title-screen recon (cycle 996)**: device screenshot shows the IDLE state — `v1.8.12 (REAL) **212665**` with "Please tap the screen", server selected = **Hector 2 (ServerId 206)**. ESTABLISHED lobby connection only, no GS. Phase 1.06+1.08+1.09 used WRONG ClientAssetVersion (199001 etc.) AND wrong ServerId (324 vs 206). The Result=22 may simply be because of those.

**Plan**:
1. Start on-device tcpdump on a broad filter: `host 183.110.40.52 or host 183.110.40.34 or host 183.110.205.25` (covers Hector 2 + Kurzel 2 + lobby), 5min capture, output to /sdcard/thered_enter_game.pcap.
2. Drive the UI to actually enter the game:
   ```
   adb shell input tap 960 540   # tap center to dismiss "Please tap the screen"
   sleep 2
   adb shell screencap -p > screen_after_tap1.png
   # Watch what appears (lobby/character select)
   # If a "Play" button appears, tap that. If character list, tap selected char.
   ```
   Take screenshot every 5s through 3 minutes, save to artifacts/.
3. Stop tcpdump. Pull pcap. tshark for PSH packets to 183.110.40.52 (Hector 2 cluster). Find opcode 3 PktLogin.
4. Decode with vampir test-wire. Save `analysis/artifacts/thered_real_login_decoded.json`.
5. **AT THE SAME TIME** retry off-device PktLogin via SOCKS5 with **ClientAssetVersion=212665, ServerId=206** (and game_host=183.110.40.52 NOT .34). Save phase1_12_pktlogin_corrected.json.

If either step yields Result≠22, we have the discriminator.

### Phase 1.11: ON-DEVICE memory extraction (correction of 1.10) — CYCLE 995
**Cycle 988 OOM**: prior 1.10 tried to stream /proc/PID/mem THROUGH codex worker → 17.5GB RSS. Wrong approach.

**Correct approach**: do all memory scanning **ON-device via adb shell** with filtered output, only pull tiny extracted results back. The device has cheap RAM, the orchestrator host doesn't need to see raw mem bytes.

**STRICT CAPS**: 512 MB RSS on worker, 30 min wall.

**Task**:
1. Find thered's current pid: `adb -s localhost:5558 shell "pidof com.netmarble.thered"`. Use that pid below (NOT 8715; thered relaunched, last seen 15771).
2. Find writable anon heap regions:
   ```
   adb -s localhost:5558 shell "su 0 grep 'rw-p .* \[anon' /proc/<pid>/maps | head -50"
   ```
3. For each candidate region in [12c00000-2ac00000] etc., scan for JWT patterns **on-device** with one adb-shell command:
   ```
   adb -s localhost:5558 shell "su 0 sh -c 'dd if=/proc/<pid>/mem bs=4096 skip=$START_PAGE count=$N_PAGES 2>/dev/null | tr -c \"a-zA-Z0-9._-\" \"\\n\" | grep -E \"^eyJhbGciOiJFUzI1Ni[A-Za-z0-9._-]{60,}$\"'"
   ```
   That pipeline returns only JWTs (very small) — pulls only matched strings back to orchestrator. Limit each scan to ≤256 pages (1MB region at a time).
4. For any JWT found, decode payload (base64url middle segment) to check `iss=87BCB7…` AND `sub=thered`. Save candidates to `analysis/artifacts/device_jwt_candidates.json`.
5. Also scan for 16-char hex tokens adjacent to the JWT (likely lobby/session token).
6. **Test replay**: pick the freshest JWT (largest `iat` in payload), use as `NetmarbleSToken` in PktLogin via SOCKS5 to GS, ServerId=324. If Result != 22 → discriminator found.

Save `analysis/artifacts/phase1_11_device_token_replay.json` with each attempt's full pktloginresult.

### Phase 1.10: Device-memory token extraction (DIVERGENCE) — CYCLE 988 PATH STALLED (SUPERSEDED by 1.11)
**Stall**: Phase 1.06 + 1.08 + 1.09 = 12+ PktLogin attempts, all returned Result=22, no movement on any constant or Token-field-mutation. Field-mutation exhausted. **MANDATORY DIVERGENCE** per orchestrator rule.

**New hypothesis**: the device's running thered (pid 8715) holds the CORRECT session token in process memory. Per memory `vampir.stoken_extracted` ("NetmarbleSToken JWT extracted from /proc/PID/mem"), the technique is proven. Use the device's ACTUAL token instead of our cpp-auth-fresh one. If Result changes from 22, the discriminator is that we need the real device-session token, not a freshly-minted one.

**STRICT CAPS**: 512 MB RSS, 25 min wall.

**Task**:
1. `adb -s localhost:5558 shell "su 0 cat /proc/8715/maps | grep -E '^[0-9a-f]+\\-[0-9a-f]+ rw' | wc -l"` — count anonymous RW heap pages.
2. `adb -s localhost:5558 shell "su 0 sh -c 'cat /proc/8715/mem 2>/dev/null'"` is unreliable — instead use `dd` per region from /proc/PID/maps. For each RW anonymous region: `adb shell su 0 dd if=/proc/8715/mem bs=4096 skip=N count=N_PAGES 2>/dev/null` (computed offset from start) — streaming-pipe through `grep -aE 'eyJhbGciOiJFUzI1Ni[^"]+'` to find JWT-shaped strings.
3. Filter found JWTs by decoding base64 payload, look for `iss=87BCB7…` AND `sub=thered`. Save candidate tokens to `analysis/artifacts/device_mem_tokens.json`.
4. Also search for 16-char hex strings near these JWTs (likely lobby-session Token like vampir's "C1F48D17BF49BC58").
5. **Most important**: re-run PktLogin to GS via SOCKS5 with NetmarbleSToken=<device-memory-JWT>. ServerId=324. If Result changes from 22 → DISCRIMINATOR IDENTIFIED.

Save trace to `analysis/artifacts/phase1_10_device_token_replay.json`.

**Outcomes**:
- Result != 22 with device-token → fact `gs_pktlogin_needs_device_session_token_2026_05_17`. Iterate further on cert + 901.
- Result still 22 → likely the GS account "87BCB7..." simply has no character record server-side; need to create one. Fact `gs_pktlogin_22_no_character_record_2026_05_17`. Escalate.

### Phase 1.09: Token-field source hunt — CYCLE 986 (after 1.08 fully exhausts constants)
**Phase 1.08 result**: 6 variants varying ClientAssetVersion / PacketHash / EnumHash all returned `Result=22`. The discriminator is NOT in those fields. The 2026-03-19 working capture had `Token = "C1F48D17BF49BC58"` (16-char hex SESSION token) which came from PktLobbyLoginResult. Android 1.8.12 flow skips that — but the SDK MUST hold an equivalent token somewhere.

**STRICT CAPS**: 512 MB RSS, 20 min wall, ≤6 attempts via SOCKS5.

**Token candidates from device state** (cpp_native_shared.xml):
- `I_TID = "F4CEDCAA304D441B889CB5758CCCF7A4"` (32-char hex — try first 16 chars `F4CEDCAA304D441B`)
- `I_NMSessionID = "BD9119DFF02041ECBDAE690161CA042A"` (32-char hex — try first 16 chars `BD9119DFF02041EC`)
- `I_NMNID = "0ba01db12e4d3f2dac833e6724bf915783dcf99c96087643b209f1ef03150c7a"` (64-char hex — try first 16 `0ba01db12e4d3f2d`)

**ServerId candidates** (some servers are in maintenance per ServerListReadResult tail; try non-maintenance):
- 324 (Kurzel 2, used so far)
- 320 (K'Valkan 1, 183.110.40.43 — 2026-03 working ServerId for Windows account)
- 201 (Carisa 1 at 183.110.40.11)
- 0xC9 (201) is the first in 183.110.40.x cluster

**Variants** (each = Token + ServerId combo):
- G: Token=`F4CEDCAA304D441B`, ServerId=324
- H: Token=`BD9119DFF02041EC`, ServerId=324
- I: Token=`F4CEDCAA304D441B`, ServerId=201 (Carisa 1)
- J: Token=`BD9119DFF02041EC`, ServerId=320 (K'Valkan 1)
- K: Token=full I_TID 32-char, ServerId=324
- L: Token="" empty, ServerId=201

Save to `analysis/artifacts/phase1_09_token_sweep.json`. If any returns Result != 22, that's the discriminator combo.

### Phase 1.08: Iterate around Result=22 — CYCLE 984 SERVER ENGAGING
**Phase 1.06 outcome**: server RESPONDED with `PktLoginResult.Result = 22` (artifact `phase1_06_gs_login_synth.json`). Account echoed correctly (87BCB7...), Token empty, BlockReason empty. NOT silent-drop anymore — server engaging us. Hypothesis: vampir's 1.6.12-era constants (ClientVersion="1.6.12", ClientAssetVersion=199001, PacketHash="561FBA1D50539D938EE501EF845B0795", EnumHash=201) are stale for current 1.8.12 server.

**STRICT CAPS**: 512 MB RSS, 20 min wall, ≤6 GS PktLogin attempts via SOCKS5 (rate-limit 5s).

**Task**:
1. First — decode the 2026-03-19 capture (`/home/sdancer/games/vampir/proxy_ex/captures/20260319_094343_183.110.40.34_12000.jsonl`) using `test-wire` binary already-built. Find C→S PktLogin frame; record its exact ClientVersion, ClientAssetVersion, PacketHash, EnumHash, MarketType, OsType. Save `analysis/artifacts/gs_capture_2026-03-19_pktlogin.json`.
2. Run 4-6 PktLogin variants iterating on the **probably-stale** fields:
   - A: ClientVersion="1.8.12", ClientAssetVersion=199001, PacketHash=vampir-default (561FBA1D...), EnumHash=201
   - B: ClientVersion="1.8.12", ClientAssetVersion=181200 (best guess for 1.8.12.0), PacketHash=vampir-default, EnumHash=201
   - C: ClientVersion="1.8.12", ClientAssetVersion=181200, PacketHash="" empty, EnumHash=201
   - D: ClientVersion="1.8.12", ClientAssetVersion=181200, PacketHash=00..0 (32 zeros), EnumHash=0
   - (if A-D all return 22) E: try lobby_token = lobby SecurityToken value (`PktLobbyVersionResult.Time` field if available, or some other PktLobbyVersionResult field)
3. Record each (Result, fields) pair in `analysis/artifacts/phase1_08_result22_sweep.json`.
4. **Successful Result (any non-22, not 0 yet) names the next discriminator. Result=0 with Token != "" means session — proceed to opcode 901 conn #2.**

Cross-reference Phase 1.07 captured constants as priority guidance.

### Phase 1.07: Decode vampir's existing GS capture + use proxy-rs encoder — CYCLE 982 USER POINTER
**User pointed at `~/games/vampir`'s packet dumper.** Two key assets that supersede synthesis-from-spec:
1. **Existing GS capture**: `/home/sdancer/games/vampir/proxy_ex/captures/20260319_094343_183.110.40.34_12000.jsonl` — 1612 frames (539 C→S, 1073 S→C) on **183.110.40.34:12000** = the exact GS we need. Contains real PktLogin (opcode 3) bytes from a working session.
2. **Authoritative encoder**: `/home/sdancer/games/vampir/proxy-rs/` (`vampir_proxy::wire::framing::C2sParser`, `protocol-rs/src/generated_packets/packets_00.rs` PktLogin struct, 25 typed fields). Encoder is field-correct by construction.

**Task** (≤30 min, ≤512 MB RSS):
1. `cd /home/sdancer/games/vampir/proxy-rs && cargo build --release --bin test-wire` — verify binary builds.
2. `./target/release/test-wire /home/sdancer/games/vampir/proxy_ex/captures/20260319_094343_183.110.40.34_12000.jsonl 2>&1 | head -200` — decode all packets. Save to `analysis/artifacts/gs_capture_2026-03-19_decoded.txt`.
3. Find C→S PktLogin (opcode 3) frame. Extract every field value (especially: Account, Token, NId, NetmarbleSToken, NetmarbleSElements, PacketHash, EnumHash, ClientAssetVersion).
4. Identify which field values are **device-dynamic** (change per-session) vs **constant** (PacketHash, ClientAssetVersion, EnumHash should be constant for the same game build).
5. If `test-wire` doesn't print field details, write a tiny Rust bin in worker workdir that uses `vampir_proxy::wire` + `protocol::generated_packets::PktLogin::deserialize` to decode the matching frame fully.
6. With known-constants extracted, construct PktLogin for current device's PID 87BCB7... NID N7d2... SToken from cpp-auth phase0_baseline_v2.json. Encode via vampir's wire codec. Send via SOCKS5 to 183.110.40.34:12000.
7. Read PktLoginResult. Save `analysis/artifacts/phase1_07_gs_login_replay.json`.
8. If Result=0 → close, open conn #2 to same GS, send opcode 901 with cert from oracle service. Recv 902.

**Outcomes**:
- Login Result=0 + Token → fact `device_gs_login_ok_replay_2026_05_17`. Continue to opcode 901.
- Login returns specific error → name fact with errorMessage, identify which field needs to be different in 1.8.12 vs 2026-03 era.

### Phase 1.06 (UNCONDITIONAL — supersedes the "destructive clear" gate): Synthesize GS PktLogin — CYCLE 982
**Worker should NOT wait for captured bytes.** The device-cached-session means we won't see fresh GS traffic without destructive app-data wipe, which is OUT OF SCOPE. Instead, **just try synthesis** — server will tell us what's wrong via Result codes (or silent-drop, which is informative too).

**STRICT CAPS**: 512 MB RSS, 20 min wall, max 4 GS connection attempts (rate-limit 5s between).

**Task**:
1. Open SOCKS5 to `195.95.201.34:12324` (creds `14ad3b4ec590f` / `f575239276` per fact `socks5_proxy_kr_2026_05_17`).
2. Connect via SOCKS5 to `183.110.40.34:12000` (game server, Kurzel 2).
3. Send vampir's `PktVersion` (opcode 1, 13 fields) — use the 1.8.12-corrected field values:
   - ClientVersion="1.8.12", ClientAssetVersion=199001 (vampir default), OsVersion="12", DeviceModelName="rk3588_s", DeviceId=nm_key=7B0D26CDC87D42EA, Ip="", PushToken="", NetmarbleSElements=`_build_nme_json(pid=87BCB7..., nid=N7d2..., nm_key=7B0D..., device_key=0361...)`, Dummy="", EnumHash=201, TimeZone="+08:00", PacketHash="561FBA1D50539D938EE501EF845B0795" (vampir hardcode — may be stale; tolerate), PacketVersion=1.
4. Read response. Save Result code.
5. Send `PktLogin` (opcode 3, 25 fields per vampir game_server_login `login_payload`). For the unknown `lobby_token` field, try in order:
   - Attempt #1: `lobby_token = stoken` (NetmarbleSToken duplicated)
   - Attempt #2: `lobby_token = ""` (empty)
6. Read response. Whatever frame comes back (PktLoginResult / any error), decode and save. Save to `analysis/artifacts/phase1_06_gs_login_synth.json`.
7. If Login Result=0: close conn #1, open conn #2 to GS, send opcode 901 PktLobbyNetmarbleSSecurityVerify (Token=cert from oracle service `root@162.244.80.97:9876` POST /cert with challenge=PID 87BCB7... as 32-hex, Log=""). Recv. Save trace.

**Falsification**:
- Both Token attempts silent-drop → PktLogin requires a field vampir's spec doesn't model. Set fact `gs_pktlogin_field_unknown_2026_05_17`.
- Login returns specific Result/errorMessage → name fact `gs_pktlogin_result_<code>_2026_05_17` with the error.
- Login Result=0 → fact `device_gs_login_ok_2026_05_17`; proceed conn #2.
- Conn #2 opcode 902 received → **`nmss_fresh_account_clientless_login_complete`** (goal success-fact-key). Metric 5/5.

### Phase 1.05: Two-connection GS flow (login conn → security conn) — CYCLE 979 USER CLARIFICATION
**Authoritative flow per user 2026-05-17** (fact `nmss_gs_two_connection_flow_2026_05_17`):
1. **Lobby** 183.110.205.25:12000 — Version + ServerListRead + close. NO login here.
2. **GS conn #1** to 183.110.40.34:12000 — PktVersion (opcode 1) + PktLogin (opcode 3, 25 fields). Login completes. Close.
3. **GS conn #2** to 183.110.40.34:12000 (FRESH TCP CONNECTION) — opcode 901 `PktLobbyNetmarbleSSecurityVerify` (Token=cert from cert-rust-repro, Log="").
4. Server replies with 902 (or any post-security packet). **THAT is the goal-success condition.**

**Vampir's `game_server_login` fuses #2 and #3 into ONE socket** (see `send_security_verify_packet` flag). For thered 1.8.12 we must SPLIT them — close after PktLoginResult, open a new TCP socket for 901.

**Plan**:
1. (Phase 1.03 in-flight) capture full traffic to confirm 2-connection structure on-wire. Look for TWO distinct TCP sessions to 183.110.40.34:12000.
2. Decode GS conn #1 PktLogin field-by-field, save `analysis/artifacts/thered_gs_login_decoded.json`.
3. Patch worker-local `game_server_login`:
   - Run PktVersion + PktLogin in conn #1 (route via SOCKS5 195.95.201.34:12324).
   - On PktLoginResult, extract session token/state needed for conn #2.
   - **Close conn #1.**
   - Open conn #2 (same SOCKS5 → 183.110.40.34:12000).
   - Send opcode 901 with cert (cert pipeline: oracle service root@162.244.80.97:9876, MAGIC32=87BCB74629734B9BAF2D948A4B8823E7).
   - Receive frame, save trace.
4. Save artifact `analysis/artifacts/phase1_05_two_conn_security.json`. If response contains opcode 902 or any frame → **fact `nmss_fresh_account_clientless_login_complete` for the existing PID** (proves chain end-to-end; fresh-account swap is a later phase).

### Phase 1.03: Capture GAME-SERVER (not lobby) traffic — CYCLE 979 PARADIGM SHIFT
**Cycle 978+979 outcome**: `thered_lobby_login_decoded.json` says: *"No client opcode 3 found in this 60s fresh-launch capture. Device only sent opcode 1 then opcode 103 on the lobby stream."*

**This means the entire PktLobbyLogin chase has been on a packet thered doesn't send.** The lobby (183.110.205.25:12000) is used only for: (1) Version handshake, (2) ServerListRead, (3) close. Login happens elsewhere — likely **directly on the game server (183.110.40.34:12000 = Kurzel 2 = ServerId 324)** via the existing PktVersion+PktLogin sequence vampir's `game_server_login` already implements.

**STRICT CAPS**: 512 MB RSS. 30 min wall.

**Task**: capture ALL traffic from thered to BOTH lobby AND game server during a fresh launch.
1. `adb -s localhost:5558 shell su 0 am force-stop com.netmarble.thered`
2. Start broad tcpdump: `adb -s localhost:5558 shell su 0 sh -c "tcpdump -i any -s 0 -w /sdcard/full_flow.pcap '(host 183.110.205.25 or host 183.110.40.34) and tcp' &"`
3. `sleep 3; adb -s localhost:5558 shell monkey -p com.netmarble.thered 1`
4. `sleep 90`  (more time so login actually fires)
5. Kill tcpdump. Pull pcap to `analysis/artifacts/full_flow.pcap`.
6. tshark dump all client→server TCP PSH payloads:
   - `tshark -r full_flow.pcap -Y "tcp.flags.push==1 and tcp.dstport==12000" -T fields -e ip.dst -e tcp.payload`
7. Decrypt with XOR_KEY (offset=0). Identify all opcodes per destination.
8. Specifically look for opcode 3 (PktLogin) sent to **183.110.40.34**. Decode field-by-field per `protocol_base_report.yaml` PktLogin spec (25 fields).
9. Save `analysis/artifacts/thered_game_server_flow_decoded.json` with per-frame ip.dst + opcode + payload + decoded fields.

**Then** (Phase 1.04): patch vampir's `game_server_login` to match observed PktLogin payload exactly. Skip lobby entirely. Connect direct to GS 183.110.40.34:12000 via SOCKS5. Send PktVersion + PktLogin. Then opcode 901 → 902.

**Outcomes**:
- opcode 3 captured at GS → decode + replay → expect Result=0 + opcode 901 → 902. Goal hits 5/5.
- opcode 3 NOT captured (device cached session) → need to clear thered app data on device first (destructive, requires user approval).

### Phase 1.02: Capture device's FULL lobby flow + replay verbatim (≤30 min) — CYCLE 978 PIVOT
**Cycle 977+978 outcome**: even with all 3 ClientVersion/OsVersion/DeviceModelName fixes + opcode 103 ServerListRead inserted + SOCKS5 (matching device's I_ConnectIP) + ServerId=324, PktLobbyLogin STILL silent-drops. The discriminator is NOT just Version-field-values — there's still some Login-payload divergence vampir's pack.

**Hypothesis**: thered may send additional opcodes between ServerListRead and Login (e.g. opcode 105 PktLobbyServerRead), AND/OR Login payload itself differs from vampir's 10-field pack. The earlier pcap only captured Version+103 (frame 16) — Login was never observed.

**STRICT CAPS**: 512 MB RSS. 30 min wall. No bruteforcing.

**Task**: get the device's actual PktLobbyLogin bytes on-wire.
1. Restart thered to force a fresh lobby flow:
   ```
   adb -s localhost:5558 shell su 0 am force-stop com.netmarble.thered
   ```
2. Start tcpdump BEFORE relaunching:
   ```
   adb -s localhost:5558 shell su 0 sh -c "tcpdump -i any -s 0 -w /sdcard/lobby_full.pcap host 183.110.205.25 and port 12000 &"
   sleep 2
   adb -s localhost:5558 shell monkey -p com.netmarble.thered 1
   ```
3. Wait 60s. Kill tcpdump. Pull `/sdcard/lobby_full.pcap`.
4. Stream through tshark for ALL device→lobby PSH packets (not just frame 16). Save hex to `analysis/artifacts/lobby_full_payloads.hex`.
5. Decrypt each frame using vampir XOR key + offset=0. Identify all opcodes sent. **Look specifically for opcode 3 (PktLobbyLogin) bytes.**
6. If opcode 3 found: decode field-by-field. Compare vs vampir's 10-field pack. Save `analysis/artifacts/thered_lobby_login_decoded.json`.
7. Patch worker-local validate_gameserver.py to match thered's exact Login pack. Retry through SOCKS5. Save `analysis/artifacts/phase1_02_lobby_login_replay.json`.

**Outcomes**:
- Login Result=0 + Token → fact `device_lobby_login_ok_replay_2026_05_17`. Proceed to GS login.
- Opcode 3 was never sent by device in capture window → thered uses different lobby flow than expected. Investigate device traffic more.

### Phase 1.01: PktLobbyVersion with EXACT 1.8.12 field values + opcode 103 (≤15 min) — CYCLE 977 DISCRIMINATOR FOUND
Worker decoded the actual on-wire PktLobbyVersion (artifact `thered_lobby_version_decoded.json`):
- **ClientVersion = "1.8.12"** (vampir hardcodes "1.6.12" — WRONG)
- **OsVersion = "12"** (vampir hardcodes "Android_12" — WRONG)
- **DeviceModelName = "rk3588_s"** (vampir hardcodes "rockchip|rk3588_s|rk30board" — WRONG)
- MarketType=0, OsType=1, DeviceId="7B0D26CDC87D42EA", Dummy="", PacketVersion=1 (these match)

And opcode 103 (PktLobbyServerListRead, seq=1) is sent IMMEDIATELY AFTER opcode 1 in the same TCP segment. Its payload contains `PID (fstring 32) + NetmarbleId (fstring 32) + i32 1 + u16 1`:
```
20 00 87BCB7...8823E7  (PID, 32 chars)
20 00 N7d225b...c4c    (NID, 32 chars)
01 00 00 00             (i32=1)
01 00                   (u16=1)
```

**Patch worker-local validate_gameserver.py**:
1. Line 297-299 PktLobbyVersion: replace `"1.6.12"` → `"1.8.12"`, `"Android_12"` → `"12"`, `"rockchip|rk3588_s|rk30board"` → `"rk3588_s"`.
2. After receiving `r1` (VersionResult) but BEFORE PktLobbyLogin, send PktLobbyServerListRead (opcode 103, seq 1) with payload = `pack_fs(pid) + pack_fs(nid) + pack_i32(1) + pack_u16(1)`. Recv the result (opcode 104) — that gives the live server list (matches phase0_9 finding).
3. THEN send PktLobbyLogin (opcode 3, seq 2) with ServerId=324 (or pick from list).
4. Use the SOCKS5 (cycle 976 fact) for connection so source IP matches device's I_ConnectIP.
5. Save to `analysis/artifacts/phase1_01_lobby_login_full_fix.json`.

**Outcomes**: Login Result=0 + Token → set fact `device_lobby_login_ok_full_fix_2026_05_17`. Continue Phase 0.8 (game_server_login + opcode 901 → 902 through same SOCKS5).

### Phase 1.00: SOCKS5 with EXACT device-matching upstream (≤15 min) — CYCLE 976 BREAKTHROUGH
User provided SSH host `sdancer@127.0.0.1:2223` whose `~/simple_proxy/src/main.rs` hardcodes the upstream SOCKS5:
- **Host=195.95.201.34, Port=12324, User=14ad3b4ec590f, Pass=f575239276**
- **CRITICAL**: This is the SAME IP as `I_ConnectIP=195.95.201.34` in the device's `cpp_native_shared.xml` CommonLogJson. thered SDK on device routes through this same proxy. The lobby silent-drop from our DE Hetzner IP IS the geo-gate — fixed by routing through this proxy.

Connect directly to the SOCKS5 (do NOT tunnel through SSH per user). Python:
```python
import socks, socket
socks.set_default_proxy(socks.SOCKS5, "195.95.201.34", 12324,
                        username="14ad3b4ec590f", password="f575239276")
socket.socket = socks.socksocket
```
Or pass via `proxy_url` to vampir's `_open_tunneled_socket` (if it supports socks5 format like `socks5://user:pass@host:port`).

Re-run lobby_login (ServerId=324, original vampir packing) with the SOCKS5 active. Expect `Result=0` + Token. If success → fact `device_lobby_login_ok_socks5_kr_2026_05_17`. PROCEED to Phase 0.8 (game_server_login through same socks5, then opcode 901 → 902).

### Phase 0.99: Route lobby TCP via Korean SOCKS5 proxy (≤15 min) — CYCLE 975 PIVOT
**Cycle 974+975 outcome**: ServerId=324 with all combinations (vanilla, ServerListRead inserted, compact_version) still times out at PktLobbyLogin. The maintenance-list flag puts 313/205/206/146/147/148/316/317/318 in maintenance, but 324 is NOT in that list — so the timeout for 324 is NOT a maintenance issue.

**STRONGEST REMAINING HYPOTHESIS**: vampir's 2026-03-29 lobby+GS pipeline succeeded only via Korean proxy tunnel (fact `vampir.full_pipeline_working`, `vampir.auth_complete`: "AUTH COMPLETE via Korean proxy tunnel"). Lobby server likely IP-gates login attempts (silent drop from non-KR/HK IPs). Our Hetzner DE IP fails geo check.

**STRICT MEMORY CAP**: 512 MB RSS. Time cap: 15 min wall.

**Task**:
1. Pick a SOCKS5-capable Korean proxy from `/home/sdancer/games/vampir/proxy_ex/test_lobby.exs` (lines ~30-32). The candidates per memory `vampir_korean_proxy_88_223_47_170_live`:
   - Primary HTTP: `http://14a5fdfb7aaa7:0cf8669540@88.223.47.170:12323` (HTTP — needs CONNECT method for TCP CONNECT, may or may not work)
   - Alternates SOCKS5: `140.235.108.104:12324`, `185.186.62.85:{10324,12323,12324}` (memory mentions these)
2. Verify a proxy supports raw TCP via SOCKS5 by attempting `socat - SOCKS5:<proxy>:183.110.205.25:12000` or python `socks` library.
3. Verify exit IP is KR/HK via httpbin first (if HTTP proxy) or via direct ipinfo lookup tunneled through SOCKS5.
4. Re-run lobby_login (worker-local validate_gameserver.py, ServerId=324, original vampir packing) through the proxy.
5. Expect: PktLobbyLogin Result=0 + session Token != "".
6. Save trace to `analysis/artifacts/phase0_99_lobby_login_kr_proxy.json`.

**Outcomes**:
- Login Result=0 → fact `device_lobby_login_ok_kr_proxy_2026_05_17`. Continue Phase 0.8 (GS login + opcode 901 through the same proxy).
- Login still timeout → fall back to (a) `Phase 0.97` (decode actual on-wire PktLobbyVersion via streaming pcap) and (b) live-capture device's actual PktLobbyLogin payload via longer tcpdump.

### Phase 0.97: Decode thered's actual PktLobbyVersion bytes (≤20 min) — CYCLE 974 PIVOT
**Cycle 974 outcome**: ServerId=324 AND ServerListRead-before-Login both still timed out at PktLobbyLogin. Discriminator is NOT just ServerId or sequence ordering. The pcap shows thered's PktLobbyVersion frame = 144 bytes vs vampir's ~84 byte pack. **The server probably stores the extra Version fields and uses them for Login state validation.**

**STRICT CONSTRAINTS** (cycle 974 worker hit 20 GB OOM):
- Memory cap: 512 MB RSS hard. Do NOT load whole pcap into python memory; stream it.
- Time cap: 20 min wall.
- No bruteforce enumeration. No sliding-window searches.
- Single targeted analysis only.

**Task**: extract and decode thered's actual PktLobbyVersion bytes from `analysis/artifacts/lobby2.pcap` frame 16 (144-byte device→lobby payload).

1. `tshark -r lobby2.pcap -Y "tcp.flags.push==1 and tcp.dstport==12000 and tcp.len>40" -T fields -e tcp.payload` to get hex bytes of all device→lobby PSH packets. Save to `analysis/artifacts/thered_lobby_payloads.hex`. Should be small (a few hundred bytes total).
2. For the first 144-byte payload (PktLobbyVersion candidate):
   - Parse vampir framing: first 2 bytes LE = total_len. Then xor_crypt(rest of body) using `XOR_KEY = bytes([0x9A, 0xA7, 0x84, 0x20, 0xD0, 0xC9, 0x78, 0xB3])` and offset=0.
   - Resulting plaintext should be: `0x9A + (crc32:u32 LE) + (opcode:u16 LE) + (seq:u16 LE) + payload`.
   - Decoded opcode should be 1 (PktLobbyVersion).
   - Payload after opcode+seq = thered's actual Version fields, ~135 bytes.
3. Compare against vampir's PktLobbyVersion fields (validate_gameserver.py:297–299):
   - `pack_u8(0) + pack_fs("1.6.12") + pack_fs("Android_12") + pack_u8(1) + pack_fs("rockchip|rk3588_s|rk30board") + pack_fs(nm_key) + pack_fs("") + pack_i32(1)`
4. Identify the **extra ~55 bytes** thered sends that vampir doesn't. Could be: an additional NetmarbleSElements JSON, a PacketHash hex, extra dummy/version fields, etc.
5. Save decoded structure to `analysis/artifacts/thered_lobby_version_decoded.json` with field-by-field hex+ascii.

**Then** (if Phase 0.97 yields extra fields): write a Phase 0.98 patched lobby_login that includes those fields in PktLobbyVersion. Run lobby_login again with ServerId=324. Expect Result=0 + Token. **Then** Phase 0.8 (GS login + opcode 901).

### Phase 0.95: Lobby-login retry with valid ServerId + ServerListRead (≤10 min) — CYCLE 972 BREAKTHROUGH
**DISCRIMINATOR IDENTIFIED**: vampir's ServerId=313 is stale. PktLobbyServerListReadResult (opcode 104) returned 99 live servers (artifact `phase0_9_live_bootstrap_313.json`). vampir's GAME_HOST=183.110.40.34 = "Kurzel 2" = ServerId **324** (0x144). Also: server requires opcode-103 (ServerListRead) between Version (1) and Login (3); vampir skips it.

**Two attack vectors — try in order**:

**Attempt A** (cheapest, vampir's existing code with one-line fix):
- Patch worker-local validate_gameserver.py:307 — change `pack_i32(server_id)` to `pack_i32(324)` (or pass `--server-id 324` to the script). Keep LoginPlatform=1 (original vampir value — it's correct; channel speculation falsified).
- Re-run lobby_login. If `Result=0` and `Token != ""` → set fact `device_lobby_login_ok_serverid_324_2026_05_17`. **Skip to Phase 0.8**.

**Attempt B** (if A still times out):
- Sequence becomes: Version (1) → recv VersionResult (2) → ServerListRead (103) → recv ServerListReadResult (104) → Login (3) with ServerId from list.
- This requires adding a new packet send between current Version and Login in the worker-local validate_gameserver.py.

**Outcomes**:
- A or B → `Result=0` + session Token → proceed to Phase 0.8 (game-server login + opcode 901).
- Both fail → live capture says Login is sent but no response — escalate as `lobby_login_substrate_exhausted` (would need full RE of the 144-byte version payload).

### Phase 0.9: Lobby-login live-diff (≤30 min) — cycle 968 outcome: all 3 channel values falsified
The hypothesis space is exhausted on field-mutation. **Live capture is now the cheapest diagnostic.** Run all three in parallel:

**(a) Device state mining (≤5 min)**:
- `adb -s localhost:5558 shell su 0 cat /data/data/com.netmarble.thered/shared_prefs/VAMPIR.xml` — likely has ServerId, character state.
- Also: `VAMPIR___Options.xml`, `cpp_engine_shared.xml`, `com.netmarble.thered_preferences.xml`, `/data/data/com.netmarble.thered/files/login-identifier.txt`.
- Find any field labeled `ServerId`, `LastServer`, `Server`, or numeric value in [1, 1000].
- If a ServerId ≠ 313 is found, save to fact `device_stored_serverid` and re-run lobby_login with that ServerId.

**(b) TLS check (≤2 min)**:
- `grep -n 'TLS\|SSL\|ssl\.\|ctx=\|wrap_socket\|_open_tunneled_socket' /home/sdancer/games/vampir/create_account/validate_gameserver.py | head -20` — determine whether lobby is plaintext or wrapped.
- If plaintext → Phase (c1). If TLS → Phase (c2).

**(c1) Plaintext on-wire capture (≤15 min)**:
- Start `adb -s localhost:5558 shell su 0 tcpdump -i any -s 0 -w /sdcard/lobby.pcap host 183.110.205.25 and port 12000` in background.
- Force-stop thered: `adb shell am force-stop com.netmarble.thered`.
- Re-launch: `adb shell monkey -p com.netmarble.thered 1`.
- Wait 30s for the SDK to do its bootstrap connection.
- Stop tcpdump.
- Pull pcap: `adb pull /sdcard/lobby.pcap`.
- Open in tshark / scapy. Find PktLobbyLogin packet (opcode 3 = first 2 bytes after 4-byte length header).
- Dump the bytes. Compare against vampir's packed payload field-by-field.
- The discriminator is the first differing byte/field. Save analysis to `analysis/artifacts/lobby_capture_diff.md`.

**(c2) TLS uprobe capture (≤15 min, only if lobby is wrapped)**:
- Per memory `feedback_kernel_instrumentation`: write a small uprobe-on-SSL_write script for thered's libssl. Capture buf+len at SSL_write entry. Dump first 100 bytes per call to `analysis/artifacts/ssl_write_lobby.bin`.
- Same comparison as (c1).

**(c3) If both fail, fall back to**: read VAMPIR.xml carefully — maybe lobby_login needs a value from PktLobbyVersionResult.Time or TimeZoneMinutes echoed back, vampir's r1 buffer parse only extracts Result.

**Outcomes**:
- Diff identifies one field → set fact `lobby_login_discriminator_<field>`, patch validate_gameserver.py-local-copy, re-run, hit Phase 0.8.
- No diff identified → mark `lobby_login_substrate_exhausted`, escalate to user as resource ask (need a binary-decoder for the lobby protocol).

### Phase 1: WebView CDP discovery (≤10 min, gated on Phase 0.8)
- `adb forward tcp:9222 localabstract:webview_devtools_remote_11954`
- `curl http://localhost:9222/json/list` → list pages. Save to `analysis/phase1_cdp_pages.json`.
- Look for any Netmarble auth URL or signin form. If the WebView is showing a non-signin page (e.g. game UI), the signin URL pattern from vampir's webview_signup.py / Members API can be navigated to via CDP `Page.navigate`. See `/home/sdancer/games/vampir/create_account/webview_signup.py` for URL patterns.
- Document what's currently visible. If no signin context is reachable, falsify (c).

### Phase 2: Drive fresh signin (≤20 min, gated on Phase 1)
- Account: `rplovhfdkm@wshu.net` / password `VampirT3st2026!x` / NetmarbleId `N82eef0101a9043599eecea053951e90`. Source: `/home/sdancer/games/autoproto/accounts/netmarble_thered_rplovhfdkm.json`.
- Per memory `feedback_inapp_webview_cdp`: use `Input.dispatchMouseEvent` (NOT JS clicks), React setter trick for inputs, label-click for checkboxes.
- Drive sign-out (if a session is showing) → sign-in with fresh creds → captcha (if any) → confirmation.
- Watch `/data/data/com.netmarble.thered/shared_prefs/cpp_native_shared.xml` (poll every 5s) until `I_PID` changes to a value different from `87BCB74629734B9BAF2D948A4B8823E7`. That new value is the fresh playerId. Save it.

### Phase 3: Complete chain with fresh PID (≤10 min)
- Run `get_stoken.py --pid <fresh_pid> --nm-device-key <fresh_NMDeviceKey> --device-key <fresh_NMDeviceKey> --country-code HK`.
- Run `validate_gameserver.py` with the resulting SToken.
- Confirm GS Result 0.
- Run opcode 901 (Token=cert from cert-rust-repro using fresh_pid as MAGIC32; Log="").
- Receive 902 or any post-security packet.
- Set fact `nmss_fresh_account_clientless_login_complete` with the artifacts list.

## Constraints (HARD)
- **Read-only on disk first**. Do NOT run commands that uninstall thered or wipe `/data/data/com.netmarble.thered/`. The device's existing session is precious (user may resume it).
- **Memory cap: 1 GB RSS** for this worker. No big binary dumps. No objdump on libUnreal.so.
- **No Frida on libUnreal.so** (memory `feedback_no_frida`). Java/system-side Frida is OK, but not needed here.
- **Time cap: 60 min wall.**
- **Rate limit**: 1 apis.netmarble.com req per 2s on Phase 0 and Phase 3.
- **Do NOT modify cpp_native_shared.xml directly**. We want the SDK to do the write, so we observe the legitimate state change.

## Output
- `analysis/device_webview_signin_2026-05-17.md` — narrative
- `analysis/artifacts/` — phase0_baseline.json, phase1_cdp_pages.json, phase2_signin_*.log, phase3_chain_*.json
- closing fact per outcome
- final line: `DEVICE_WEBVIEW_SIGNIN_DONE`

## Relevant files
- vampir signin: `/home/sdancer/games/vampir/create_account/get_stoken.py` (sign-in JSON shape at line 431)
- vampir webview signup (CDP reference): `/home/sdancer/games/vampir/create_account/webview_signup.py`
- vampir GS client: `/home/sdancer/games/vampir/create_account/validate_gameserver.py`
- Fresh creds: `/home/sdancer/games/autoproto/accounts/netmarble_thered_rplovhfdkm.json`
- Cert pipeline: `/home/sdancer/nmss-emu/cert-rust-repro/` + remote oracle `root@162.244.80.97:9876`
- Opcode 901 shape (Token fstring + Log fstring): `/home/sdancer/games/vampir/protocol_base_report.yaml`
- Device shared_prefs: `/data/data/com.netmarble.thered/shared_prefs/cpp_native_shared.xml`
- WebView CDP socket: `@webview_devtools_remote_11954` (thered's WebView pid 11954)
- Harness: `/home/sdancer/orchestrator/harness`
