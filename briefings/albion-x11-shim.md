# albion-x11-shim — LD_PRELOAD shim, XI2 cookie extension (Task 5)

## Role & workdir
Existing Codex worker (codex_app_server, durable thread). Workdir: `/home/sdancer/albion-x11-shim`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **make synthesized XTest input events reach Albion's Unity TMP_InputField focus predicate by intercepting Unity's XI2 GenericEvent cookie path** — the actual event-read mechanism per the cycle-3240 falsification of the libX11-XEvent send_event hypothesis.

## Hypothesis (one-line)
Unity reads its XInput2 input via `XGetEventData(display, &cookie)` to decode `GenericEvent` cookies for XI_RawButtonPress / XI_ButtonPress / XI_RawMotion. An LD_PRELOAD shim hooked on `XGetEventData` (and the adjacent `XFreeEventData`, `XNextEvent` for cookie-bearing events) can normalize the synthesized/raw-vs-cooked event flags inside the cookie's `XIDeviceEvent` payload — making XTest-synthesized events indistinguishable from physical input at the XI2 cookie consumer level.

## Falsification criterion (one-line)
Shim hooks `XGetEventData` and confirms (via `/tmp/x11_send_event_shim.log` queue-call records) that Unity calls it during 2FA modal interactions, BUT a cancel click + filled+OK click test on the modal STILL produces zero pixel diff → mechanism is past the XI2 cookie consumer (e.g., raw evdev fd in Unity, IInputCallback in IL2CPP, etc.) → close path with "raw-input or app-layer beyond XI2 GenericEvent."

## Already achieved (do not re-falsify, do not re-do)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `analysis/x11_link_audit.md` | Unity uses libX11 Xlib/XInput2 dynamically (no libxcb-direct, no libSDL2) | ✅ DONE |
| 2 | `src/shim.c` + `scripts/build_shim.sh` + `build/x11_send_event_shim.so` (16608B) | Build infrastructure works; 8 libX11 hooks compile cleanly with dlsym(RTLD_NEXT) | ✅ DONE — extend this file, do NOT rewrite from scratch |
| 3 | `/opt/albion-x11-shim/x11_send_event_shim.so` on container + patched `spawn_preload.sh` (backup at `.bak.20260522`) | Dual LD_PRELOAD chain photon_tap.so + shim.so loads into live Albion (lsof verified PID 273236) | ✅ DONE |
| 4 | `xinput test-xi2 --root` evidence | XTEST pointer/key events confirmed REACHING the X server + Albion child window 0x200007 at (782,687) | ✅ DONE |
| 5 | `analysis/shim_verdict.md` Test A + Test B | send_event-clear-on-XEvent hypothesis FALSIFIED: zero pixel diff on cancel-click AND field-type-submit; shim log NEVER fired | ✅ DONE — falsified.md entry written |

## Success criteria (refined for XI2 cookie hypothesis)
1. **Telemetry confirmation**: Extended shim's `/tmp/x11_send_event_shim.log` shows `XGetEventData` being called by Albion during a 2FA modal cancel-click test (proves Unity uses XI2 cookies for modal input). If shim log STAYS empty even with the extension → falsification applies and we close to "raw-input or beyond" hypothesis.
2. **Modal change under shimmed XI2 events**: After XI2-cookie-aware shim is deployed + Albion restarted, a `xdotool` cancel click on the 2FA modal produces non-zero pixel diff on the dialog. (`mean_absdiff > 0` is acceptable bar; perfect dismissal not required for the path to validate.)
3. **No regression**: 5 production daemons (cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest, Albion-Online) remain healthy. The token-watcher (sister path) is unmodified and unaffected.

## Tasks (sequential, fast turnaround)

### Task 1 — extend the shim
Edit `src/shim.c` to add wrappers for the XI2 cookie API. Keep all existing 8 libX11 hooks; ADD:
- `Bool XGetEventData(Display *d, XGenericEventCookie *cookie)` — call real, then on success scrub the cookie data: if cookie->extension == XInput2_opcode, treat cookie->data as XIDeviceEvent or XIRawEvent and clear any flag bit that distinguishes synthetic from physical (the XIDeviceEvent's `flags` field contains XIPointerEmulated for synthesized events — clear this); also clear send_event on the XEvent wrapper if applicable.
- `void XFreeEventData(Display *d, XGenericEventCookie *cookie)` — call real; log count.
- Optional: `int XNextEvent(...)` already exists from prior version — make sure it logs when ev.type == GenericEvent (35) so we can correlate.

XInput2 struct definitions are in `<X11/extensions/XInput2.h>` — `cookie->data` is a `XIDeviceEvent*` for `XI_ButtonPress` (event type 4) / `XI_ButtonRelease` (5) / `XI_KeyPress` (2) / `XI_KeyRelease` (3); `XIRawEvent*` for `XI_RawButtonPress` (15) / `XI_RawButtonRelease` (16) / `XI_RawKeyPress` (13) / `XI_RawKeyRelease` (14). The `XIDeviceEvent.flags` field contains `XIPointerEmulated` (bit 0x1) for synthesized events.

Rebuild via `scripts/build_shim.sh`. The build script must dlsym XGetEventData/XFreeEventData from libXi.so.6 (since they're in libXi, not libX11).

### Task 2 — deploy + restart
1. `scp build/x11_send_event_shim.so` to `/opt/albion-x11-shim/x11_send_event_shim.so` on the container (overwriting the prior version).
2. `rm /tmp/x11_send_event_shim.log` on the container to clear stale state.
3. Restart only the `albion-client` tmux session per the existing recipe — `spawn_preload.sh` already has the LD_PRELOAD chain.
4. Confirm via `lsof -p $(pidof Albion-Online)` that BOTH photon_tap.so AND x11_send_event_shim.so are mapped to the new PID.

### Task 3 — retest
Reproduce the exact 2FA modal from cycle 3240 (login form → submit → 2FA prompt). Run the same two tests:
- **Test A**: cancel click at template-confirmed `(782, 687)`. Capture pre/post via xwd → convert. Diff via cv2.
- **Test B**: field-click `(981, 632)` → `xdotool type --clearmodifiers --delay 30 -- "PRV6T7E"` → OK click `(1153, 702)` → Return. Capture pre/filled/post. Diff all pairs.

**Crucial new instrumentation step**: After both tests, `tail -100 /tmp/x11_send_event_shim.log` on the container. Look for `fn=XGetEventData` lines. If they appear → Unity DID use XI2 cookies for modal input → cookie-scrubbing scrub is the right target. If they DON'T appear → falsification, Unity uses raw evdev or higher-layer path.

### Task 4 — verdict
Rewrite `analysis/shim_verdict.md` (rewrite, don't append) with the same Achievement-levels-+-gaps framing as the prior verdict file. Mark each new level. If hypothesis confirmed → declare path closed, success, set `albion_xi2_shim_works_2026_05_22` fact. If falsified → declare it cleanly and suggest the NEXT hypothesis (raw evdev? Unity input-system internal queue? IL2CPP-level intercept?).

## Constraints & gotchas
- **DO NOT modify** `/home/sdancer/albion-token-capture/` or systemd unit `albion-token-watcher.service`. Sister path still in standby.
- **Preserve `-DDISABLE_SEND_HOOKS`** invariant on photon_tap.so per `[[albion-send-hooks-break-client]]`.
- **NEVER hook libUnreal.so** — but Albion uses Unity not Unreal; libX11/libXi LD_PRELOAD is fine.
- **Reuse existing build infrastructure** — extend `src/shim.c`, don't rewrite. The deploy + restart recipe in your prior turns is already proven; just rerun it.
- **Idempotent shim deploy**: scp atomic, then restart `albion-client` only. Other 4 daemons stay up.
- **Verify with screenshot diff AND shim-log evidence** — both required. If only one disambiguates, the test is incomplete.
- **The watcher polls /state every 30s** — if the XI2 shim works and Albion auto-progresses past 2FA, the token-watcher will autonomously capture. You do NOT need to think about token capture.

## Relevant files / references
- Live container: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Existing shim source: `/home/sdancer/albion-x11-shim/src/shim.c` (EXTEND, don't replace)
- Existing deploy point: `/opt/albion-x11-shim/x11_send_event_shim.so` on container
- Existing spawn_preload.sh patch (backed up): `/opt/albion-frida-capture/spawn_preload.sh.bak.20260522`
- Sister-path watcher (DO NOT TOUCH): `/home/sdancer/albion-token-capture/scripts/` + systemd `albion-token-watcher.service`
- Verdict file (prior): `/home/sdancer/albion-x11-shim/analysis/shim_verdict.md` — rewrite with this turn's results
- Memory pointers: `[[no-frida]]`, `[[albion-send-hooks-break-client]]`, `[[orchestrator-role]]`, `[[macromanage-workers]]`.

## Reporting
Final deliverable: rewritten `analysis/shim_verdict.md` with Achievement-levels-+-gaps framing. Either declare hypothesis confirmed (XI2 cookie scrub works) with a fact + zone-change observation, or declare it falsified with the next-hypothesis suggestion. Keep it under 100 lines.
