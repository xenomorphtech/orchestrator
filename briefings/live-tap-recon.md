## live-tap-recon — Passive live-tap reconnaissance on Waydroid loopback

## Role & workdir
Device-aware recon worker. Workdir: `/home/sdancer/dark-december-live-tap-recon` (worktree of `/home/sdancer/darkdecember/`, branch `live-tap-recon`).

## Current goal / sub-goal
- **goal_key**: `dark_december_live_tap_recon` (new)
- **sub_goal_key**: `passive-pcap-capture-and-compare-to-frozen-corpus`

## Why this turn exists
The full DD protocol decode work (23 closures + protocol wiki) has been done on a **frozen pre-reassembled corpus** (`first_quest_{c2s,s2c}.tcpstream.bin`). All synthesized timestamps are `t=0.0`. Cycle 417's Rank 3 (live-loopback-typed-consumer) was deferred. A passive, anticheat-clean tcpdump on Waydroid loopback `lo` (with the game in-game) would:
- Validate the cycle-435 framing model on fresh data
- Provide real microsecond timestamps for c2s↔s2c correlation
- Surface message types absent from the first_quest capture (chat, NPC dialog, combat events)
- Test whether the typed Rust decoder's classifier holds on live frames

This is passive observation only — NO injection, NO Frida, NO ptrace. Anticheat-clean per [[feedback_no_frida]].

## Hypothesis
A 60-second passive tcpdump on the Waydroid `lo` interface, while the game is in-game and a character is moving, captures ≥1000 application frames; ≥30% match the cycle-435 framing model + the existing typed Rust decoder's 7 classifiers; AND the capture contains ≥3 length classes not present in the first_quest corpus (chat / quest progress / inventory).

## Falsification (3 outcomes)
- (a) **≥1000 frames captured + ≥30% typed coverage + ≥3 new length classes observed** → SUCCESS. Fact: `dark_december_live_tap_<n>_frames_<m>_typed_<k>_new_classes`.
- (b) **Device not in-game OR no traffic on loopback** → precondition failure; abort cleanly per [[feedback_verify_precondition_before_probe]]. Fact: `dark_december_live_tap_precondition_failed`.
- (c) **Capture works but typed coverage <10% OR no new length classes** → corpus drift or framing model breaks on live data. Fact: `dark_december_live_tap_drift_observed`.

## Success criteria — SINGLE TURN

**Primary**: write `/home/sdancer/dark-december-live-tap-recon/analysis/live_tap_recon_2026-05-15.md` with:

1. **Precondition check** ([[feedback_verify_precondition_before_probe]]):
   - `adb -s localhost:5558 shell screencap -p > /tmp/dd_screen.png && ls -la /tmp/dd_screen.png` — confirm screenshot saved.
   - Capture is NOT a pixel-perfect inspection; just verify the file exists + size > 100 KB (= game is rendering, not at a black title screen).
   - `ss -tn` or `netstat -tn` to confirm at least 1 ESTABLISHED TCP connection on Waydroid loopback to the DD server.
   - If either check fails: abort with verdict (b) + write the artifact + set fact + exit.

2. **Find target interface**:
   - `adb shell ip addr show` or `ip link show` to identify the loopback or bridge interface carrying the DD traffic.
   - The DD server is `158.101.105.58:10001` per the cycle-383 pcap headers; find the host-side bridge that routes to Waydroid.
   - **Fallback**: tcpdump on `any` interface filtered to `dst port 10001 or src port 10001`.

3. **Run tcpdump for 60 seconds**:
   - `sudo tcpdump -i any -w /tmp/dd_live_capture.pcap 'tcp and (port 10001 or src host 158.101.105.58 or dst host 158.101.105.58)' -G 60 -W 1` — single 60s rotation.
   - Save to `/home/sdancer/dark-december-live-tap-recon/captures/dd_live_2026-05-15.pcap`.
   - Report packet count from `tcpdump -r capture.pcap | wc -l`.

4. **TCP-reassemble + decode** with `darkdec_decoder.py`:
   - Strip TCP, separate C2S/S2C by direction, build per-direction byte streams.
   - Decode frames via the standard 4B-length + 2B-channel + adjacent-XOR pipeline.
   - Report: total frames, C2S/S2C split, decoded-length histogram.

5. **Diff against first_quest corpus**:
   - List length classes present in live capture but NOT in cycle-408 s2c-inventory or cycle-390 c2s-decoder.
   - Top-5 new decoded prefixes (4-byte) per direction.

6. **Apply typed Rust decoder** (cycle-422 typed_packets.csv format):
   - Pipe the live capture through `darkdec_decoder_cli --c2s ... --s2c ... --emit-typed` if feasible (or apply the Python equivalent of the typed classifier inline).
   - Report typed coverage % on live vs frozen corpus (64.63% baseline).

7. **Verdict (a)/(b)/(c) + fact-set + DONE.** Print `LIVE_TAP_RECON_DONE`.

## Constraints & gotchas
- **PASSIVE ONLY.** NO Frida injection on libUnreal.so [[feedback_no_frida]]. NO ptrace. NO kernel module install. tcpdump is read-only on the network stack.
- **HARD memory budget: 500 MB.** pcap files are bounded by 60s × ~20kbps ≈ 200 KB; decode is small.
- **HARD output cap**: artifact ≤400 lines. pcap file separately at `captures/`.
- **ONE Codex turn budget: ≤25 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **Precondition gate is non-negotiable**: if the screenshot is missing or the device shows the title screen or 0 ESTABLISHED connections, abort cleanly with (b). Do NOT try to autoenter the game — that's a separate path.
- **Capture location**: put the pcap under `captures/` in the worktree (gitignore it) — pcap files may contain sensitive traffic.
- The `streams/` directory in darkdecember/main is gitignored under `*.pcap` so no commit concern.
- Cross-pollination memory: [[project_dark_december_wire_framing_plus8]] (REVISED to +7), [[project_dark_december_wire_decoder]], [[feedback_verify_precondition_before_probe]], [[feedback_no_frida]].

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-live-tap-recon/` (branch `live-tap-recon`).
- Frozen corpus (DIFF TARGET): `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- Typed Rust decoder (REFERENCE): `/home/sdancer/dark-december-rust-decoder-typed-v2/target/release/darkdec_decoder_cli`
- Protocol wiki (REFERENCE): `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md`
- Cycle-408 s2c-inventory: `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
- Cycle-390 c2s-decoder: `/home/sdancer/dark-december-c2s-decoder/analysis/c2s_decode_2026-05-15.md`
- 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- success-fact key: `dark_december_live_tap_<n>_frames_<m>_typed_<k>_new_classes` (a)
- block-fact keys: `dark_december_live_tap_precondition_failed` (b), `dark_december_live_tap_drift_observed` (c)
