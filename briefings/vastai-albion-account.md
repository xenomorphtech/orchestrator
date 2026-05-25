# vastai-albion-account — create Albion account + character

## Role & workdir
Codex worker, workdir `/home/sdancer/vastai-albion`. The actual work happens on the remote FR vast.ai instance via SSH and adb-equivalent VNC interaction.

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-account-created-character-created`

## User directive (verbatim, /talk#vastai-albion-web 20:18)
> "create an account on albion and create a character"

The Albion Online client is currently running on the FR vast.ai instance and visible via KasmVNC (xvnc :1 desktop). User wants you to register a fresh Albion Online account in the launcher, log in, and create a character in-game.

## Substrate already provisioned (DO NOT recreate)
- **vast.ai instance**: FR `37014838`, SSH `ssh -i ~/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai`
- **Albion Online**: launched at PID 8603 (per `vastai_albion_game_running_2026_05_18` fact) on the `albion` user account
- **Albion launcher window**: "Albion Online Client" 1920x952+0+0 rendered on Xvnc `:1`
- **albion user**: `uid=1000(albion)`, password set to `welcome to my w0rld`
- **xfce4 session** running, KasmVNC at container :3000
- **Cloudflare tunnel**: routed to wrong port (`:3030` instead of `:3000`) — direct SSH access works

## Success criteria
- Albion Online account registered with email + password (use a disposable inbox — see thered-fresh-account session 20260518T181249Z for a working pattern with `com.android.adbkeyboard` IME analog, but this is desktop Linux so use directly-typed input via `xdotool` or VNC mouse/keyboard events).
- Account successfully logged into in the Albion launcher.
- Character created in-game (name, race, faction, gender choices — pick any sensible defaults).
- Set fact `vastai_albion_account_created_2026_05_18` with the account email + character name.
- Verdict at `/home/sdancer/vastai-albion/analysis/albion_account_verdict.md`. Final line `VASTAI_ALBION_ACCOUNT_DONE`.

## Concrete tasks (do in order)

1. **Take a screenshot of current Albion state**:
   ```bash
   ssh -i ~/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai \
     "su - albion -c 'DISPLAY=:1 XAUTHORITY=/home/albion/.Xauthority xwd -root | convert xwd:- /tmp/albion_current.png'"
   scp -i ~/.ssh/id_ed25519 -P 14838 root@ssh8.vast.ai:/tmp/albion_current.png /tmp/albion_current.png
   ```
   Inspect via Read tool. Confirm launcher is on the login screen.

2. **Provision a disposable inbox** on the host (the `albion` Linux user). Options:
   - Use a temp-mail service via curl (mail.tm, 1secmail.com, getnada.com).
   - Generate a random username, fetch inbox URL/API endpoint, store both for polling.
   - Save the inbox credentials to `/home/albion/.albion_inbox.json` on the remote so the worker can re-read them.

3. **Drive the launcher's "Sign Up" / "Create Account" flow**:
   - Find the create-account button via screenshot inspection (XY pixel coords).
   - Use `xdotool` (install via `apt-get -y install xdotool` if missing) to click and type:
     ```bash
     ssh ... "DISPLAY=:1 xdotool mousemove X Y click 1 sleep 0.3 type --delay 30 'email@temp'"
     ```
   - Fill: email, password, confirm password, accept terms, submit.
   - Screenshot after each step to verify.

4. **Receive email verification + confirm**:
   - Poll the disposable inbox API for the Albion verification email (usually 30s–2min lag).
   - Extract the confirmation link / code from the email body.
   - If a link: open in xdg-open or curl-GET on the device, OR follow the verification code field in the launcher.

5. **Log in and reach character creation**:
   - After verification, return to launcher login screen.
   - Enter credentials, submit.
   - Click "Play" / "Enter Game".
   - Wait for character creation screen.

6. **Create a character**:
   - Pick a race (any), gender (any), name (random sensible 6-12 char name).
   - Confirm and enter the world.

7. **Set fact + verdict**:
   - `harness fact-set vastai_albion_account_created_2026_05_18 "email=<x>, password=<y>, character_name=<z>, in-world=true"`
   - Verdict at `/home/sdancer/vastai-albion/analysis/albion_account_verdict.md` (≤80 lines). Final line `VASTAI_ALBION_ACCOUNT_DONE`.

## Falsification criteria
- **Launcher requires phone-verification or CAPTCHA we cannot solve** → screenshot the prompt, save it, mark as escalation. Do NOT spam-retry.
- **Account creation requires payment / paid client** → screenshot the gate, mark as user-resource ask.
- **Anti-cheat (BattlEye/EAC) bans the cloud IP on character creation** → record the ban message, mark goal CLOSED-blocked-by-AC. **Do NOT spam re-launches.**
- **Account created but launcher shows no character-create button** → take screenshot, dump what's on screen, escalate.

## Constraints & gotchas
- **All Albion commands run as `albion` user** via `su - albion -c '...'`, NOT as root. Game runs in user's HOME with user's DISPLAY.
- **DO NOT** modify Cloudflare tunnel config — that's a separate fix already escalated.
- **Use `xdotool` for keyboard input**, not raw X events — more reliable across launcher's input widgets.
- **Screenshot after EVERY major UI transition**. The state machine is brittle on cloud-VNC.
- **DO NOT** click anywhere outside the Albion launcher window — may close it or trigger weird interactions.
- **Disposable email**: pick a service with a stable API (1secmail is simple — `https://www.1secmail.com/api/v1/?action=getMessages&login=X&domain=Y`).
- **Save credentials** to `/home/albion/.albion_credentials.txt` on the remote (mode 600) so future sessions can re-log in.

## Relevant files / references
- `~/.ssh/id_ed25519` — host SSH key (registered with vast.ai key id 612169)
- Prior verdict: `/home/sdancer/vastai-albion/analysis/albion_launch_verdict.md`
- Memory: `[[reference-vastai]]`, `[[project-albion-substrate]]`, `[[feedback-worker-artifact-isolation]]`, `[[feedback-inapp-webview-cdp]]` (similar webview-input problem pattern from netmarble signup)
- Facts: `vastai_albion_game_running_2026_05_18` (PID 8603, window rendered), `vastai_albion_fr_local_url_live_2026_05_18` (KasmVNC + tunnel topology)
