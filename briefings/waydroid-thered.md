# waydroid-thered — Bring up Waydroid on SSH host 2223, install thered fresh, capture PktLogin

## Role & workdir
Waydroid bring-up worker. Workdir: `/home/sdancer/nmss-emu-waydroid-thered` (this is just where you save artifacts on the orchestrator host — the actual Waydroid work is REMOTE on `127.0.0.1:2223`).

## **HARD RULES — read these first**

1. **DO NOT TOUCH the RK3588 device at `localhost:5558` for ANY reason.** No `adb -s localhost:5558`. No screenshots of it. No `am force-stop com.netmarble.thered`. The previous turn wasted 8 minutes on that device — abandon that entirely.
2. **ALL work happens on the SSH host `127.0.0.1:2223`** via `ssh -p 2223 sdancer@127.0.0.1 "<cmd>"`. That host has Waydroid installed; the RK3588 does not.
3. **Don't take screenshots until thered is actually running inside Waydroid.** Screenshots of the wrong target are noise.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `waydroid-thered`

## Hypothesis
Bringing up a Waydroid Android session on the SSH host, installing thered fresh, and forcing a real login produces actual on-wire PktLogin bytes (capturable via tcpdump on the SSH host) that reveal what 12+ off-device synthesis attempts kept getting wrong (server consistently returned `Result=22`).

## Falsification
- (a) Waydroid session boots + thered runs + tcpdump captures PktLogin to 183.110.40.34 → success, decode, replay.
- (b) Waydroid session won't boot (binder/ashmem/headless-compositor blockers) → diagnose specific failure mode, set fact `waydroid_session_blocked_<reason>`, stop.
- (c) thered launches but crashes/freezes on weston-headless → fact `waydroid_thered_unreal_freeze`. Stop (Unity froze on this; thered=Unreal may also fail).

## Plan

### Step 1: Survey what's on the SSH host (≤2 min)
```bash
ssh -p 2223 sdancer@127.0.0.1 "
  echo === installed compositors ===
  command -v weston cage sway gnome-shell labwc river
  echo === dbus-run-session ===
  command -v dbus-run-session
  echo === current XDG_RUNTIME_DIR contents ===
  ls /run/user/\$(id -u)/ 2>/dev/null
  echo === waydroid status ===
  sudo -n waydroid status 2>&1 | head -6
  echo === lxc ===
  sudo -n lxc-ls --running 2>&1 | head -5
"
```
Save to `analysis/artifacts/host_survey.txt`.

### Step 2: Start a wayland session (≤5 min)
Pick the first compositor available from Step 1. Try in order: `cage`, `sway --unsupported-gpu`, `weston --backend=headless-backend.so`.

Example with cage (which is a one-shot kiosk):
```bash
ssh -p 2223 sdancer@127.0.0.1 "
  export XDG_RUNTIME_DIR=/run/user/\$(id -u)
  mkdir -p \$XDG_RUNTIME_DIR
  chmod 700 \$XDG_RUNTIME_DIR
  dbus-run-session -- nohup cage -s -- sleep infinity >/tmp/cage.log 2>&1 &
  sleep 3
  ls \$XDG_RUNTIME_DIR/wayland-* 2>&1
"
```
Verify `wayland-0` (or similar) exists. If no compositor works, fact `waydroid_no_compositor_<list_tried>` and stop.

### Step 3: Start waydroid session (≤5 min)
```bash
ssh -p 2223 sdancer@127.0.0.1 "
  export XDG_RUNTIME_DIR=/run/user/\$(id -u)
  export WAYLAND_DISPLAY=wayland-0
  nohup waydroid session start >/tmp/waydroid_session.log 2>&1 &
  sleep 30
  sudo -n waydroid status
"
```
If status shows `Session: RUNNING` → continue. Else save log, set fact `waydroid_session_failed`, stop.

### Step 4: scp thered APK to SSH host + install (≤5 min)
```bash
scp -P 2223 /home/sdancer/games/vampir/apk/base.apk sdancer@127.0.0.1:/tmp/thered.apk
ssh -p 2223 sdancer@127.0.0.1 "waydroid app install /tmp/thered.apk 2>&1 | tail -10"
ssh -p 2223 sdancer@127.0.0.1 "waydroid app list 2>&1 | grep -iE 'thered|netmarble' | head -5"
```

### Step 5: tcpdump + launch thered (≤15 min)
```bash
ssh -p 2223 sdancer@127.0.0.1 "
  sudo -n nohup tcpdump -i any -s 0 -w /tmp/waydroid_capture.pcap 'host 183.110.40.34 or host 183.110.205.25' >/tmp/tcpdump.log 2>&1 &
  echo TCPDUMP_PID=\$!
  sleep 2
  waydroid app launch com.netmarble.thered 2>&1 | head -5
  echo waiting 120s for SDK auth flow...
  sleep 120
  sudo -n pkill -INT tcpdump
"
scp -P 2223 sdancer@127.0.0.1:/tmp/waydroid_capture.pcap analysis/artifacts/waydroid_capture.pcap
```

### Step 6: Decode + report (≤10 min)
- Use vampir's `test-wire` binary (already built in `/home/sdancer/games/vampir/proxy-rs/target/release/test-wire`) to extract C→S frames to 183.110.40.34.
- Find opcode 3 (PktLogin) frame. Decode field-by-field.
- Save `analysis/artifacts/waydroid_pktlogin_decoded.json` with each field's value.
- Set fact `waydroid_pktlogin_captured_<short_hash>` with the canonical decoded values.

## Output deliverables
- `analysis/artifacts/host_survey.txt`
- `analysis/artifacts/waydroid_session.log`
- `analysis/artifacts/waydroid_capture.pcap`
- `analysis/artifacts/waydroid_pktlogin_decoded.json`
- Final line `WAYDROID_THERED_DONE`

## Memory + time caps
- 4 GB RSS hard. Stream pcap with tshark, don't load whole capture in Python.
- 50 min wall cap total.
- If any step takes longer than its budget → stop, write a fact describing the blocker, exit cleanly.

## Why this turn exists (one-liner)
12+ off-device PktLogin probes returned Result=22; on-device cached session never re-authenticates; Waydroid fresh install is the only path to capture real Android-1.8.12 PktLogin bytes that aren't already known.
