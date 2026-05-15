# dark-december-pcap-analysis — Offline analysis of the captured cold-launch pcap

## Role & workdir
Offline packet analyst. Workdir: `/home/sdancer/dark-december-pcap-analysis` (worktree of `/home/sdancer/dark-december`, branch `pcap-analysis`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump`.
- Sub-goal: extract maximum protocol-shape information from the **existing captured cold-launch pcap** — pure offline, no device, no content patch needed. This is the only tangible protocol artifact we have until the content-patch resource ask is resolved.

## Success criteria
Concrete deliverable: `<workdir>/analysis/pcap_protocol_shape_2026-05-14.md` containing:
- Timeline of TLS endpoints contacted during cold launch (which CDN/auth/AppGuard servers, in what order, with what timing).
- Per-endpoint **flow signature** (packet count, byte volume, request-response cadence, TLS version, cipher suite, cert subject).
- Any **non-TLS frames** present (some Korean anticheats use raw-TCP custom protocols on top of TCP, or UDP — those would be readable in the clear).
- AppGuard's network behavior (is it HTTP, custom-binary, or just TLS-tunneled?).
- Cross-pollinated correlation: which captured TLS endpoints map to which Hive class from `dark_december_hive_class_map_2026_05_15` fact (e.g., auth.qpyou.cn:443 = LoginCenter, aas.withhive.com:443 = membership).

## Progress so far (from sibling paths, all retired)

- **recon (done-success)**: identified Hive auth + INCA AppGuard + UE4 4.26.2 + xapk SHA256 `12a15315601eafb8314a1594e187213c0574b44602923661d6d10053b59577e0`.
- **libcompatible-disasm (done-partial-falsification)**: characterized INCA AppGuard's native side (asmFunction indirect dispatch at `0x19e2b8`). NOT the auth crypto host.
- **hive-auth-trace (done-success)**: Hive HTTP wrappers use **timestamp-derived AES-128-CBC/PKCS7 zero-IV** (LoginCenter=SHA1, AuthV4/Membership=SHA256 of timestamp, first 16 hex chars UTF-8). WebSocket layer is plain JSON. Token chain mapped end-to-end.
- **protocol-live-dump (retired)**: captured cold-launch pcap reaching title + account selection (`/home/sdancer/dark-december-protocol-live-dump/captures/darkdec_launch_task1_2026-05-14.pcap`, 976 packets / 80.87s, TLS to status.darkdecember.net + assets.darkdecember.net + auth.qpyou.cn + aas.withhive.com + analytics-log.withhive.com + AppGuard endpoints). Blocked at content-patch gate. Region game-server URLs (`game-server-*.live.darkdecember.net:10001`) appear only post-login.

## Next 2–3 concrete tasks

1. **Open pcap + endpoint timeline.** Tools: `tshark`, `wireshark`, `python3 (pyshark or scapy)`. The pcap is at `/home/sdancer/dark-december-protocol-live-dump/captures/darkdec_launch_task1_2026-05-14.pcap`. Produce:
   - Per-flow summary: (src_ip, dst_ip, dst_port, first-seen-ts, last-seen-ts, total bytes, packet count).
   - Sort by first-seen-ts to get the temporal sequence of "what the app contacts when".
   - Annotate each with the SNI (Server Name Indication from the TLS ClientHello) — that gives the actual hostname the app intended even though we can't see the encrypted payload.

2. **AppGuard / Hercules-specific examination.** INCA's anticheat often uses an out-of-band channel. Look for:
   - Non-TLS connections (any traffic where the ClientHello doesn't appear).
   - UDP packets to unusual ports.
   - Raw-binary patterns that look like length-prefixed frames (e.g., 4-byte BE length + payload).
   - Custom HTTP on a non-443 port.

3. **TLS metadata fingerprinting.** Per flow:
   - JA3/JA3S fingerprints (TLS-level client/server fingerprints — sometimes Korean games have distinctive patterns).
   - Cert chain / subject / issuer for each endpoint.
   - HTTP/2 vs HTTP/1.1 (visible in ALPN).

4. **Cross-correlate with Hive class map**. From `dark_december_hive_class_map_2026_05_15`: `auth.qpyou.cn` should correspond to LoginCenter (login + oauth + preLogin), `aas.withhive.com` to AuthV4/Membership / GetSession, etc. Confirm with packet timing.

5. **Write the artifact** as above.

## Constraints & gotchas

- TLS is intact in this capture — content decryption requires per-session keys we don't have. Don't try to decrypt the bodies; focus on metadata, flow timing, and any non-TLS channel.
- The pcap is from a partial run (no post-auth game-server traffic) — be explicit about scope coverage in the artifact.
- This worker runs under systemd `harness-worker@dark-december-pcap-analysis.service` (system.slice, MemoryMax=24G).

## Relevant files / references

- pcap: `/home/sdancer/dark-december-protocol-live-dump/captures/darkdec_launch_task1_2026-05-14.pcap` (976 packets / 80.87s).
- Sibling artifacts:
  - `/home/sdancer/dark-december/analysis/recon_2026-05-15.md`
  - `/home/sdancer/dark-december-libcompatible-disasm/analysis/{libcompatible_disasm_2026-05-15.md, task2_xrefs_2026-05-15.md}`
  - `/home/sdancer/dark-december-hive-auth-trace/analysis/{task1_hive_class_location_2026-05-14.md, task2_session_key_derivation_2026-05-14.md}`
  - `/home/sdancer/dark-december-protocol-live-dump/analysis/{live_protocol_dump_2026-05-14.md, task2_ingame_protocol_*.md if produced}`
- Cross-pollination facts: `dark_december_recon_complete_2026_05_15`, `dark_december_hive_class_map_2026_05_15`, `dark_december_hive_http_keys_known_2026_05_15`, `dark_december_socket_layer_plain_json_2026_05_15`, `dark_december_pcap_endpoints_complete_2026_05_15`, `dark_december_obbassets_split_missing_2026_05_15`.
- Tools: `tshark`, `wireshark`, `pyshark`, `scapy`, `openssl`, `python3`.

## Falsification

- The pcap is empty or corrupted (rejected by tshark).
- All flows are bog-standard TLS-1.3 with no distinguishing features (no JA3/JA3S signal, no out-of-band channel) — in which case the artifact still has the endpoint timeline + flow summary as a documentary close.

This path is bounded to ~1-2 cycles. After artifact written, the goal stays in the **content-patch resource-ask escalation** state pending user input.
