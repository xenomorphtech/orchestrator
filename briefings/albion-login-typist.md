# albion-login-typist — get past Albion Sign In via any working mechanism

## Role & workdir
Fresh Codex worker (codex_app_server, new thread). Workdir: `/home/sdancer/albion-login-typist`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **fill Albion's Login form Email + Password and click Sign In**, observable via screenshot showing the form-vanished or "Logging In..." spinner, leading to `/state self.zone != null`.

The orchestrator has spent cycles 3133-3141 mapping the failure surface. Read the **Already-falsified** table below carefully — do NOT re-test those mechanisms unless you have a specific reason to believe the test was flawed.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| 1 | Action-emitter daemon killed (no Escape spam) | Clean substrate, no competing dispatcher | vast.ai | ✅ DONE |
| 2 | `xdotool key Escape` works — state transitions Login (4144528) → entry (4159224) → quit-dialog (4138972) | Global keyboard events DO reach Albion | vast.ai | ✅ DONE |
| 3 | `xdotool mousedown+sleep+mouseup` at native (880,520) activates the Login button (entry-menu → Login form) | Button click mechanism works | vast.ai | ✅ DONE |
| 4 | `/home/albion/.albion_credentials.txt` (mode 600) has `EMAIL=...\nPASSWORD=...` from May 18 saved creds | Credentials available on box | vast.ai | ✅ DONE |
| 5 | xclip 0.13-2 installed; python3-pycryptodome, python3-evdev also installed | Userspace toolchain ready | vast.ai | ✅ DONE |
| 6 | Albion window: id 33554438, name "Albion Online Client", geometry 1920x1080 at (0,0), focused | Window topology known | vast.ai | ✅ DONE |
| 7 | Screenshot scale: native 1920×1080 → `/screenshot.png` 1280×720 (×1.5 from screenshot to native) | Coordinate math known | vast.ai | ✅ DONE |

## Falsified mechanisms (DO NOT re-test without new insight)
| Mechanism | Result | Cycle |
|---|---|---|
| xdotool `click 1` (fast) on field | hover-only, diff <500 | 3138-3139 |
| xdotool `mousedown+sleep+mouseup` on field | hover-only | 3137-3138 |
| xdotool `click --repeat 2 1` (double) on field | diff=178 (hover) | 3139 |
| xdotool `click --repeat 3 1` (triple) on field | diff=0 | 3139 |
| Tab walk 0-7 times + `xdotool type` | diff=0 every step | 3140 |
| xclip + xdotool `key ctrl+v` paste | diff <=27 | 3141 |
| `/dev/uinput` injection | EPERM at open() (no CAP_SYS_ADMIN) | 3128 |
| Raw RFB TCP at :8444 | port is HTTPS+WSS only | 3127 |

xdotool field coords tested (all native 1920x1080) with `mousedown+sleep+mouseup`:
- (800-1120) × (500-700) grid — all hover-only or opened "Open external website" link at (1024-1040, 580-600)
- (920,615), (960,580), (880,600), (960,510), (960,490): hover only

**Conclusion:** Albion's TMP_InputField rejects synthesized XInput events. Buttons accept them. The orchestrator gave up on X11-synthesized paths.

## Success criteria
1. `/state` returns `self.zone != null` within ≤10 min after starting your turn — or at minimum the screenshot shows the form gone / "Logging In..." spinner.
2. Audit your dispatch with `/screenshot.png` size diff: Login form = ~4144700, quit dialog = 4138972, entry menu = 4159611. Anything outside these = state moved.
3. **No regression**: 4 surfaces 200, no daemon respawned.

## CRITICAL NEW FINDING (cycle 3142)
There IS a working login script in the existing worktrees — find it and STUDY it:
- `/home/sdancer/vastai-albion-navmesh-integrate/scripts/steps/10_login_full.py` and `10_login_enter_email.py`
- Pattern: `xdotool windowfocus --sync $WID` → 2-step `mousemove` (offset by -100x, sleep 0.3, then to target, sleep 0.8) → `click 1` → `xdotool key ctrl+a` → `xdotool type --clearmodifiers --delay 30 -- "$TEXT"`
- Uses template matching (`templates/email_field.png`) with OpenCV to find field coords — orchestrator could not run because cv2 not on local; you have python3-pycryptodome on container, install opencv with `pip install opencv-python` if needed
- **Critical:** scripts use `DISPLAY=:1` (Xtigervnc, older substrate). Current container has `DISPLAY=:2` (Xkasmvnc). Orchestrator replicated the EXACT pattern at :2 → diff=0 (no input landed). **Hypothesis: Xkasmvnc filters XTEST events that Xtigervnc accepted.**

## Tasks — pick the lowest-cost untried mechanism first

### Task 0 — Investigate Xkasmvnc vs Xtigervnc input filtering (NEW HIGHEST-PRIORITY)
Test hypothesis: maybe a fresh Xtigervnc on :3 (or any other free display) would accept input where Xkasmvnc :2 doesn't. Quick experiment:
1. Start `Xtigervnc :3 -geometry 1920x1080 -depth 24 -SecurityTypes None` on a free display.
2. Re-target Albion to :3 (kill + relaunch with DISPLAY=:3, or use `xdotool keydown`/`mousemove` while moving the window).
3. Run the proven login_full.py pattern at :3.
4. If text field accepts input on :3 → Xkasmvnc is the blocker; user/orchestrator runs Albion on Xtigervnc going forward.

OR simpler: see if there's a way to make Xkasmvnc accept XTEST. Check Xkasmvnc flags — `xdpyinfo -display :2` to see extensions. Maybe `-AcceptKeyEvents` or similar flag needs to be set on Xkasmvnc startup.

### Task 1 — Try `login.auto=1` prefs hack (cheapest)
1. Read current prefs: `cat "/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs"`.
2. Edit the `<pref name="login.auto" type="string">0</pref>` to `1`, similarly `login.auto.decision`.
3. Also write a `<pref name="login.email" type="string">BASE64(EMAIL)</pref>` and same for password if Albion supports saved-creds prefs (search the XML for "login.email" / "saved" / "credentials" / "email" patterns — Albion may store these in PlayerPrefs already).
4. Kill + relaunch Albion via `/opt/albion-frida-capture/spawn_preload.sh` or the equivalent supervisor wrapper.
5. Screenshot after relaunch — does the form auto-fill or auto-submit?
6. If yes: success, record. If no: tabe paths.

### Task 2 — Investigate Unity TMP_InputField focus predicates
Check what xdotool's events look like vs what Unity expects:
- `runuser -u albion -- env DISPLAY=:2 xinput list` to list devices.
- `xinput test-xi2 --root` while you click, screenshot. Compare event types.
- Maybe Unity requires `XI_RawButtonPress` (from physical device) rather than `XI_ButtonPress` (synthesized). xdotool emits the latter.
- Possible fix: there's a small C program using `XTestFakeButtonEvent` with `XI_ButtonPress` AND `is_synthetic=False` flag — Unity may accept it if synthetic flag is unset.
- Or use `xinput map-to-output` to bind a virtual device that produces "physical" events.

### Task 3 — LD_PRELOAD on Albion's input function
Albion uses Unity's `Input` API. Specific functions like `UnityEngine.Input.GetKey` are called from C# but the underlying input read is via libc or X11. If you can `LD_PRELOAD` a shim that injects fake key events into Albion's call stack:
- Use Unity's IL2CPP function table; Albion-Online_Data/il2cpp_data has the metadata.
- Hook `XQueryKeymap` or similar to return synthesized state.
- Complexity is high; do this only if Tasks 1 and 2 both fail.

### Task 4 — Milestone
Append to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` with: mechanism used, pre/post screenshots committed to `analysis/cycle3142_login_complete/{pre,post}.png`, `/state` post-login output.

## Constraints & gotchas
- **No new pip deps** beyond pycryptodome/evdev/xclip (already on box).
- **No LD_PRELOAD on libUnreal.so** — per `[[no-frida]]` memory (anticheat). LD_PRELOAD on Unity libs is OK but be surgical.
- **Do NOT restart the 5 production daemons** (cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest, Albion-Online itself — wait Albion you may have to restart for prefs hack).
- **For Albion restart**, use the same spawn_preload.sh pattern with LD_PRELOAD photon_tap.so `-DDISABLE_SEND_HOOKS` invariant per `[[albion-send-hooks-break-client]]`.
- **Credentials never go into git** — read from `/home/albion/.albion_credentials.txt` at runtime.
- **No "structurally impossible" closure** — there are still untried mechanisms. The autonomy rule says keep searching.
- **Verify with screenshot diff**, not dispatch_result.

## Relevant files / references
- Live container: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Albion config: `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/`
- Credentials: `/home/albion/.albion_credentials.txt`
- Albion install: `/home/albion/albion-online/` (binary + Albion-Online_Data)
- Spawn script: `/opt/albion-frida-capture/spawn_preload.sh`
- Memory pointers: `[[xdotool-unity-albion-blocked]]` (RETRACTED but lessons preserved), `[[albion-vastai-daemon-stack]]`, `[[albion-send-hooks-break-client]]`, `[[orchestrator-role]]`.
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`.

## Reporting
(a) Milestone with screenshot showing post-Sign-In state + `/state` self.zone value, OR (b) precise blocker description. Not "I think it works".
