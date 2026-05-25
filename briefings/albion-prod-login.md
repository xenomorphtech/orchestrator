# albion-prod-login — production-grade autonomous login pipeline

## Role & workdir
Fresh Codex worker (codex_app_server, new durable thread). Workdir: `/home/sdancer/albion-prod-login`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **fully automate the Albion login pipeline end-to-end so `/state` reports `self.zone != null` from a cold container start with zero user intervention.**

User directive (2026-05-22 cycle 3159): "1 hour is trivial, prefer the best routes rather than cheap routes" — budget multi-hour worker time on the comprehensive solution, not minimal patches.

## Three parallel paths (all required for full closure)

### Path A: production-substrate-swap-to-tigervnc-3 (3-4h)
**Why:** Cycle 3151 breakthrough proved Albion's TMP_InputField rejects synthesized XTEST input on Xkasmvnc :2 but accepts it on Xtigervnc :3. To autologin from cold start, Albion must run under :3.

Implementation:
- Modify `/usr/local/bin/run-albion-client` (or its supervisor wrapper at `albion-supervise albion-client`) so the spawned Albion-Online process launches under `DISPLAY=:3` instead of `:2`.
- Spawn `Xtigervnc :3 -geometry 1920x1080 -depth 24 -SecurityTypes None -localhost` as a supervised daemon (or use existing Xtigervnc binary; see `feedback_xdotool_unity_albion_blocked` retracted memory for context).
- Decide & implement: do you (i) ALSO migrate the dashboard `/vnc/index.html` viewer to :3 (cleanest, but loses KasmVNC's H.264 streaming), or (ii) keep Xkasmvnc :2 alive for the browser viewer AND run Xtigervnc :3 for Albion (parallel-display). Pick whichever is more robust to supervisor restarts.
- Repoint `/screenshot.png` capture in `gamestate_service.py` if needed (note: `import -window root` currently fails because Albion's DRI rendering bypasses X11 framebuffer — see fact `screenshot_503_root_cause_2026_05_22`).
- Verify: `xdotool key Escape` AND a synthesized `xdotool type` both reach Albion when it's on :3.

### Path D: 2fa-email-code-pipeline (1-2h)
**Why:** On every fresh container, Albion's "new device" prompt fires after Sign In. Email arrives at the disposable mailbox `5fswkv6zf4@wshu.net` (wshu.net is a temp-mail service). To advance past the prompt, poll the inbox + extract code + type it.

Implementation:
- Determine the disposable-mailbox API for wshu.net (likely mail.tm-style REST: `GET /messages` after auth). If wshu.net doesn't expose an API, switch to inboxkitten.com / mail.tm with a fresh inbox + re-register the Albion account (avoid this unless wshu.net is dead).
- Build a worker module (Python, stdlib + `requests` if available) that: (1) authenticates to the inbox, (2) polls every 5s for ≤2min for a new message from Albion, (3) regex-extracts the 6-digit code, (4) returns it.
- After Sign In, screenshot the 2FA dialog → identify code-field hit-region (template-match against artifact `/home/sdancer/albion-login-typist/analysis/cycle3142_login_complete/post.png` if needed) → click + `xdotool type --clearmodifiers --delay 30 -- "$CODE"` → click OK button.
- Verify: post-2FA screenshot shows character-select screen (not the security-code dialog anymore).

### Path P: persist-deviceid-across-container-rotation (1-2h, OPTIONAL but recommended)
**Why:** Each vast.ai container rotation regenerates `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/prefs` `DevideId` field, which is part of Albion's device-trust signature. With persistence, the 2FA only fires ONCE ever (vs. every rotation).

Implementation:
- Create a vast.ai persistent volume bind-mount for `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/` (or at minimum the `prefs` file).
- Or, copy the `prefs` file to a host-side persistent location at supervisor-start, and copy back to container's expected path.
- Verify: after a container rotation simulation (kill container + relaunch), the DevideId UUID matches the prior value AND Albion skips the 2FA prompt on Sign In.

Note: Albion may ALSO key on egress IP, so even with DevideId persistence the 2FA might still fire if the rotation also changes egress IP. Treat this as best-effort optimization, not a hard requirement.

## Success criteria
1. From a fresh start (Albion not yet logged in), the worker can drive the whole flow autonomously: spawn substrate → launch Albion → dismiss intro → click Login → fill email/password → submit → if 2FA prompt appears, poll inbox + extract code + fill it + submit → wait until Albion reaches character-select → click character / Enter World.
2. `curl https://albion.orch.run/state` returns `self.zone != null` (some non-null zone name like `"Brecilien Wilds"` or similar) within ≤15 minutes of starting the flow.
3. Audit log JSONL captures every dispatched input with `dispatch_backend=xdotool`, `dispatch_result=ok`, and a state-transition annotation (`login_form → 2fa_prompt → char_select → in_zone`).
4. Repeating the flow on a fresh container (or simulated reset) reproduces the result.
5. Throughout: 5 prior production daemons (cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest, Albion-Online itself) remain healthy. `/state /vnc/index.html /actions.json` still 200. `/screenshot.png` 503 is a known side-issue, NOT a blocker.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | Xtigervnc :3 accepts xdotool TMP_InputField input | Login pattern works on alternative substrate | ✅ DONE (cycle 3151) |
| 2 | `/home/sdancer/albion-login-typist/analysis/cycle3142_login_complete/{pre,filled,post}.png` | Pre/post evidence of credentials filled + 2FA prompt reached | ✅ DONE |
| 3 | Credentials at `/home/albion/.albion_credentials.txt` (`EMAIL=5fswkv6zf4@wshu.net\nPASSWORD=albion260518q9`) | Auth material on-box | ✅ DONE |
| 4 | xclip 0.13-2, python3-pycryptodome 3.11.0, python3-evdev 1.4.0 installed | Toolchain ready | ✅ DONE |
| 5 | Proven login script pattern at `/home/sdancer/vastai-albion-navmesh-integrate/scripts/steps/10_login_full.py` | Tested click+type recipe | ✅ DONE |
| 6 | Memory `[[albion-login-substrate]]` + `[[albion-2fa-container-rotation]]` | Cross-session documentation of breakthrough + 2FA mechanism | ✅ DONE |

## Constraints & gotchas
- **No LD_PRELOAD on libUnreal.so** — anticheat. Other LD_PRELOAD targets (Unity-side X libs) are OK if needed.
- **LD_PRELOAD photon_tap.so MUST stay `-DDISABLE_SEND_HOOKS`** — see `[[albion-send-hooks-break-client]]`.
- **DO NOT log credentials to JSONL or git**. Read at runtime from `/home/albion/.albion_credentials.txt`.
- **Production daemons stay healthy**: cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest must not be disrupted by the substrate-swap. If gamestate_service hangs after restart, kill its child (PID will respawn under the supervisor) — see cycle 3154 reference.
- **Verify with screenshot diff, NOT dispatch_result:ok** — see `[[xdotool-unity-albion-blocked]]` retraction lesson.
- **Use the proven click pattern**: `windowfocus --sync $WID` → 2-step `mousemove (cx-100, cy) sleep 0.3; mousemove cx cy sleep 0.8` → `click 1` → `key ctrl+a` → `type --clearmodifiers --delay 30 -- "$TEXT"`.
- **DO NOT use Xkasmvnc :2 for any text input** — confirmed broken in cycle 3151.
- **Document everything in talk-channel milestone** at `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` with: substrate decision, 2FA flow trace, deviceid persistence approach (if implemented), final `/state` self.zone value.

## Tasks (parallel sequencing)
Execute in this order, but Path D can be developed in parallel with Path A's substrate work:
1. **Reconnaissance.** Read `[[albion-login-substrate]]` + `[[albion-2fa-container-rotation]]` memories. Read `10_login_full.py`. Confirm xclip + pycryptodome + evdev still installed. Confirm credentials file present.
2. **Path A — substrate swap.** Get Albion running under Xtigervnc :3 in production (under the existing albion-supervise wrapper). Verify text input works there. Test with a one-shot `type "X"` into the Email field and screenshot diff.
3. **Path D — 2FA pipeline.** Implement `inbox_poll.py` against wshu.net (or fallback mail.tm). Test it independently of Albion (manually trigger an Albion login from a browser to verify the mail arrives + your code is extracted).
4. **Integration.** Chain everything: substrate ready → spawn Albion → drive intro dialogs → click Login → fill creds → if 2FA, poll inbox + fill code → click OK → wait char-select → click char → Enter World. Each transition verified by screenshot diff.
5. **Path P (optional).** If you have time after the main flow works, implement DevideId persistence.
6. **Milestone post + facts.** Talk-channel JSONL entry with full state-transition trace. Set facts: `albion_autologin_e2e_2026_05_22 = <verbatim self.zone value>`, `albion_2fa_code_pipeline_built = true`, etc.

## Relevant files / references
- Existing scripts: `/home/sdancer/vastai-albion-navmesh-integrate/scripts/steps/10_login_full.py`, `12_login_submit.py`, `lib/click_template.py`.
- Templates: `/home/sdancer/vastai-albion-navmesh-integrate/scripts/templates/{email_field,initial_login_btn,enter_world_btn,character_slot_veldrynx}.png`
- Memories: `[[albion-login-substrate]]`, `[[albion-2fa-container-rotation]]`, `[[albion-vastai-daemon-stack]]`, `[[no-frida]]`, `[[albion-send-hooks-break-client]]`, `[[orchestrator-role]]`.
- Cycle 3151 artifacts: `/home/sdancer/albion-login-typist/analysis/cycle3142_login_complete/`.
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`.

## Reporting
Milestone with verbatim `/state` JSON showing `self.zone != null` is the only acceptance signal. Anything less (e.g. "credentials filled" without zone progression) is partial — keep working. Use the "Achievement levels + gaps" framing if you must report partial progress.
