# dark-december-patch-driver — drive the in-app content patch + capture game-server traffic

## Role & workdir
Long-running UI + capture driver. Workdir: `/home/sdancer/dark-december-patch-driver` (worktree of `/home/sdancer/dark-december`, branch `patch-driver`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump`.
- Sub-goal: **drive the in-app content patch download to completion**, then capture the first wave of game-server traffic on `:10001` (the ingame protocol the user asked for). Replaces "8.94 GB resource ask" with an automated download.

## Success criteria
Concrete deliverable:
- `:10001` tcpdump capture with ≥1 round-trip of game-server traffic from `game-server-*.live.darkdecember.net`.
- `<workdir>/analysis/ingame_protocol_capture_2026-05-14.md` documenting: download progress timeline, captured frame inventory, JSON message taxonomy (per sibling fact `dark_december_socket_layer_plain_json_2026_05_15` — the WS layer is plain JSON, so frames should be readable post-TLS).
- Sets fact `dark_december_ingame_socket_captured`.

## Progress so far (5 retired paths — read these closing artifacts first)
- `/home/sdancer/dark-december/analysis/recon_2026-05-15.md` — UE4 + INCA AppGuard + Hive + xapk SHA256.
- `/home/sdancer/dark-december-libcompatible-disasm/analysis/{libcompatible_disasm_2026-05-15.md, task2_xrefs_2026-05-15.md}` — protection layer, not auth crypto.
- `/home/sdancer/dark-december-hive-auth-trace/analysis/{task1_hive_class_location_2026-05-14.md, task2_session_key_derivation_2026-05-14.md}` — Hive token chain + HTTP wrapper crypto (SHA1/SHA256-timestamp → AES-128-CBC zero-IV); WS = plain JSON.
- `/home/sdancer/dark-december-protocol-live-dump/analysis/live_protocol_dump_2026-05-14.md` + pcap — blocked at patch gate after reaching title + account selection.
- `/home/sdancer/dark-december-pcap-analysis/analysis/pcap_protocol_shape_2026-05-14.md` — endpoint timeline + outlier 150.109.190.139:443 unanswered SYNs.

Key cross-pollination facts (in harness):
- `dark_december_obb_seed_in_progress_2026_05_15`: main.12039.com.needsgames.darkdecember.obb already seeded.
- `dark_december_vampir_appguard_crash_2026_05_15`: VAMPIR side process crash dialog must be dismissed.
- `dark_december_stale_xerda_contamination_2026_05_15`: `pm disable com.netmarble.thered` clears NMSS leak.
- `dark_december_obbassets_split_missing_2026_05_15`: 8.94 GB ASTC pak content patch needed.

## Next 2–3 concrete tasks

1. **Pre-flight environment.** `adb connect localhost:5558`. Verify `com.netmarble.thered` is disabled (`adb shell pm list packages -d | grep netmarble`). Verify the OBB seed file exists (`adb shell ls /sdcard/Android/obb/com.needsgames.darkdecember/main.12039.com.needsgames.darkdecember.obb`). Free disk check: `adb shell df /data` should have ≥12 GB free. If any precondition fails, report and exit — DO NOT try to "fix" without authorization.

2. **Start tcpdump persistently.** Two parallel captures (broad + narrow):
   - Broad: `adb shell "tcpdump -i any -w /sdcard/Download/dd_full_$(date +%s).pcap -G 600 -W 30 'not port 53'"` — rolling 10-min chunks, 30 chunks max (5 hours).
   - Narrow on `:10001`: `adb shell "tcpdump -i any -w /sdcard/Download/dd_10001_$(date +%s).pcap port 10001"` — anything on this port means game-server.
   - Run both in background (`screen` / `&`).

3. **Launch app + drive UI to start patch download.**
   - `adb shell am start -n com.needsgames.darkdecember/...` (use whatever activity name `aapt dump badging` reports).
   - Wait for the patch prompt screen — screenshot every 10s to detect UI changes (`adb shell screencap -p > snap_$(date +%s).png`).
   - Tap "OK"/"Download"/"확인" on the patch prompt — coordinate hunting via `adb shell dumpsys window | rg 'darkdecember' -A 2` or via UI-automator. If the prompt is in Korean, the OK button is typically bottom-right or bottom-center.
   - Optional: `adb shell input tap <X> <Y>` based on the screenshot inspection.

4. **Monitor patch progress.** Every 60s during the long wait:
   - `adb shell du -sh /data/data/com.needsgames.darkdecember /sdcard/Android/data/com.needsgames.darkdecember /sdcard/Android/obb/com.needsgames.darkdecember` — see where bytes are landing.
   - `adb shell ls -la /storage/.../obb/com.needsgames.darkdecember/*.obb` — patch files often land as OBB extensions (`patch.NNNN.com.X.obb`) or as `.pak` files inside `app_sandbox/`.
   - Logcat tail: `adb shell logcat -d -t 100 | rg -i 'patch|update|download|ASTC|pak'` every 60s.
   - If download speed is < 100 KB/s for 5 consecutive checks: abort and report (Waydroid network or CDN rate-limit).

5. **When download completes** (no new bytes for 60s AND game advances past the patch screen):
   - Snapshot the device's app dir for evidence of `*.pak` arrival.
   - Drive UI to login (guest if available, otherwise stop and ask for credentials).
   - Monitor the `:10001` tcpdump file size — when it starts growing, the ingame protocol is live.
   - Let it accumulate 5+ minutes of typical-play traffic (idle on lobby, do one server-bound action like inventory inspection).

6. **Write artifact.** Even if step 5 doesn't reach full success: document progress timeline, screenshots, logcat excerpts, and the final state. The artifact lands `<workdir>/analysis/ingame_protocol_capture_2026-05-14.md`.

## Constraints & gotchas

- **systemd timeout is 10800s (3h)** for this unit. The download must complete within that window. If it doesn't, the worker reports the speed/cause and a future cycle re-attempts.
- **NO Frida on libcompatible.so/libUE4.so/AppGuard** — instrumentation gets detected. Pure UI-driving + tcpdump + memory reading via ptrace (if needed) only.
- **adb root + SELinux permissive** confirmed on this Waydroid.
- Host-side tcpdump CANNOT see Waydroid namespace traffic — MUST use on-device tcpdump (rooted ADB).
- VAMPIR crash dialog appears periodically — auto-dismiss via `adb shell input keyevent KEYCODE_BACK` if it pops mid-driving.
- Stale `com.netmarble.thered` is already pm-disabled per prior cycle — if it's re-enabled itself, just disable again.
- This worker runs under systemd `harness-worker@dark-december-patch-driver.service` in system.slice with MemoryMax=24G.

## Relevant files / references

- xapk: `/home/sdancer/dark-december/apk/dark-december-1.2.039.xapk` (588 MB).
- All prior closing artifacts listed above.
- Tools: `adb`, `tshark`, `wireshark`, `pyshark`, `python3`, `aapt`, `unzip`.

## Falsification

- 3 cycles with the worker timing out at the 3h mark having made no detectable progress on patch download (size unchanged) — escalate that the CDN is unavailable or the patch trigger requires user-account creation.
- Patch download succeeds but `:10001` never sees traffic — escalate (auth/session block, content fine but server rejecting client).
- Anti-cheat trips during the long-run capture (game crash with VAMPIR repeatedly) — escalate; tooling-side workaround beyond pm-disable is needed.
