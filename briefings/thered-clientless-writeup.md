# thered-clientless-writeup — Document the proven login chain

## Role & workdir
Documentation worker. Workdir: `/home/sdancer/nmss-emu-device-webview-signin`.

## Why this turn exists
The login chain end-to-end (cpp-auth → lobby → GS PktLogin) is proven via SOCKS5 replay (see `replay_pktlogin.json` Result=0 with Token=9253DD6691CE2E35). Conn #2 with opcode 901 is blocked on cert oracle (`162.244.80.97:9876` connection refused) and we don't have a session.json for current PID `87BCB74629734B9BAF2D948A4B8823E7`. Rather than idle-poll the oracle, produce a clean writeup the user can act on.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `thered-clientless-writeup`

## Task — write ONE file: `analysis/THERED_CLIENTLESS_CHAIN.md`

Concise (≤200 lines), structured as:

### 1. What works (proven end-to-end on 2026-05-17)
- **SOCKS5 path:** `195.95.201.34:12324` user `14ad3b4ec590f` pass `f575239276` (matches device's `I_ConnectIP`)
- **cpp-auth:** plain JSON `{nmDeviceKey, deviceKey, countryCode, playerId}`. Key correction: `nmDeviceKey` = 16-char `I_NMDeviceKey`; `deviceKey` = 32-char top-level `<UDID>` from `cpp_native_shared.xml` (these are DIFFERENT). Returns ES256 SToken.
- **Lobby (183.110.205.25:12000):** Send `opcode 1 PktLobbyVersion` then `opcode 103 PktLobbyServerListRead` (with body = `pid + nid + i32(1) + u16(1)`). Receive server list. Close.
- **GS conn #1 (183.110.40.52:12000 for Hector 2 in this case):** Send `PktVersion` then `PktLogin` (25 fields per `protocol-rs::PktLogin`). Receive `PktLoginResult Result=0` (frame is LZ4-compressed when `flags=128` — vampir wire codec must handle this).
- **Wire codec:** vampir framing `[u16 LE total_len][XOR_offset0(0x9A + crc32_LE + opcode_LE + seq_LE + payload)]` with XOR key `9A A7 84 20 D0 C9 78 B3`. S→C decode flags byte; if `flags & 0x80`, body after flag is LZ4-compressed.

### 2. The 3 missing discriminators (vs vampir's 2026-03 hardcoded values)
| Field | 1.8.12 real | vampir hardcoded |
|---|---|---|
| `PacketHash` | `75164F0C6FB1EED830DD0220852A17B2` | `561FBA1D50539D938EE501EF845B0795` (1.6.12-era) |
| `Token` | 16-char hex session from lobby handshake (e.g. `9253DD6691CE2E35`) | tried SToken / various 32-char hex |
| `Language` | `1` | `14` |
| Other fields match: ClientVersion=`1.8.12`, ClientAssetVersion=`212665`, ServerId=`206` for Hector 2, OsType=1, MarketType=0, DeviceModelName=`""` empty, OsVersion=`""` empty |

### 3. Reusable values for current device (PID 87BCB74629734B9BAF2D948A4B8823E7)
List these so the user can hand the recipe to anyone:
- Per-account: PID, NId, deviceKey (32 hex), nmDeviceKey (16 hex)
- Per-session (rotates): Token (16-char hex), SToken (JWT)
- Per-build constants: PacketHash, ClientAssetVersion, EnumHash, NetmarbleSElements JSON (mostly device-stable)

### 4. Remaining work for goal 5/5 (opcode 901 PktLobbyNetmarbleSSecurityVerify)
- **Need:** cert from cert-rust-repro
- **Blocker A:** oracle `162.244.80.97:9876` connection refused. Bring service back up.
- **Blocker B:** if doing locally, need a session.json for the current PID. Old `donor_session_2026-04-29.json` is for PID `2FCF99…`, not `87BCB7…`.
- **Open question:** is the cert challenge = PID, or is it a server-issued nonce (one of the post-login frames op8/op27/op102 we observed)?

### 5. Reproduction recipe (numbered, copy-pastable)
For someone who wants to do this end-to-end from scratch:
1. Capture lobby+GS pcap via `adb shell tcpdump` (port 12000)
2. Drive UI past "Please tap the screen"
3. Pull pcap, run `proxy-rs/target/release/test-wire` to decode
4. Replay PktLogin via SOCKS5 using `validate_gameserver.py` patched with the 3 discriminators
5. Get `PktLoginResult Result=0`

## Output
- ONE file: `analysis/THERED_CLIENTLESS_CHAIN.md`
- Set fact `thered_clientless_chain_writeup_2026_05_17`.
- Print `THERED_WRITEUP_DONE` on final line.

## Constraints
- 15 min wall cap. NO network probes. NO sends to server. NO worker memory above 256 MB.
- Read-only on existing artifacts. The file is a sweep/distillation of what's already known.
- Do NOT include speculation as if it were fact. Mark unknowns clearly.
