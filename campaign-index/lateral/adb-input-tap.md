# Lateral: bypass `adb input tap` filtered by Unreal

**Wall**: `walls/adb-input-tap.md` — `input tap 960 540` does not advance Vampir title screen.

## Experiments (in order of cheap → heavy)

1. `[x]` **`sendevent` to `/dev/input/event*`** — **DEAD END on this device.** Cycle 710 confirmed `getevent -lp` and `/proc/bus/input/devices` list ONLY keys, headset, HDMI, and power devices. **No touchscreen event node is exposed.** Evidence: `analysis/tmp/proc_bus_input_devices_2026-05-02.txt`. Raw sendevent / uinput-direct are therefore not actionable on this build. Original plan (kept for reference): feed raw input event packets directly to the kernel input driver, bypassing InputManager. Required figuring out the device file (`getevent -lp` lists devices) and event format (X/Y/PRESSURE/SYNC). Concrete:
   ```
   adb shell getevent -lp   # find touchscreen device, e.g. /dev/input/event2
   adb shell 'sendevent /dev/input/event2 3 53 100 ; sendevent /dev/input/event2 3 54 200 ; ... ; sendevent /dev/input/event2 0 0 0'
   ```
   Cost: ~10 min to figure out the right event triplet for the device.

2. `[ ]` **uinput synthetic touchscreen** — open `/dev/uinput`, register a virtual touch device, emit MT_SLOT/MT_TRACKING_ID/X/Y events. More involved than sendevent, but creates a separate input device that some apps treat as legit hardware. Use the `uinput` Python package or write a small C tool.

3. `[ ]` **`adb shell input swipe`** with a real distance — swipe is a different InputManager codepath than tap; sometimes Unreal's tap-detector requires a small touch+release motion that `tap` doesn't generate cleanly.
   ```
   adb shell input swipe 960 540 961 540 50
   ```
   Cost: 30 seconds to test.

4. `[ ]` **AccessibilityService click** — write a tiny Android accessibility service that dispatches `AccessibilityService#dispatchGesture()`. Bypasses normal input-event filtering because gestures arrive through a different pipe. Cost: ~1 hour to write/sideload, but durable.

5. `[ ]` **Reverse-engineer the Unreal input filter** — find what timestamp/source/flag check rejects the synthesized tap, patch it via Frida (now that spawn-mode RPC works). Highest cost, most surgical.

## Recommended next (cycle 710 update)
- Experiment 3 (`input swipe`): **tested, no effect.**
- Experiment 1 (`sendevent`): **dead end** — no touchscreen event node on this build.
- Experiment 4 (AccessibilityService): probably the cheapest remaining lateral — gestures use a different pipe than InputManager that's harder to filter.
- Experiment 5 (RE the Unreal input filter): now feasible since the native service lane gives us in-process code execution. Once the native service is deployed, it can patch the input filter directly.
- The native-service lane (see `tools/built/INDEX.md`) sidesteps the title-tap entirely if it can drive `nmssNativeGetCertValue` directly without UI advancement — worth testing first.
