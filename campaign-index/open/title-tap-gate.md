# Title-screen tap gate

**Status**: blocked, awaiting manual physical tap from user.

## Background
Clean monkey-launched `com.netmarble.thered` session (current: pid 31790) sits on the Vampir title screen with `請輕觸畫面。` ("please touch the screen") and does not auto-advance.

`adb shell input tap 960 540` (screen center on 1920×1080) does NOT clear the gate — see `walls/adb-input-tap.md`. Likely Unreal-engine input filtering rejects synthesized OS input events.

## What unblocks this
A physical tap on the device screen by the user. Once dismissed, cert-ptrace polls for in-app state and starts the RPC agent probe.

## Things to try if user can't tap
1. Direct uinput injection via `/dev/input/event*` (not yet tried, may also be filtered)
2. Send a swipe gesture instead: `adb shell input swipe 540 960 540 1500 100`
3. Find the actual tap-target UI element coordinates from a logcat decompilation

## Things NOT to do
- Don't loop `input tap` retries (confirmed ineffective)
- Don't restart the session — current session is the most stable we've had (clean monkey launch, ~50min alive)
