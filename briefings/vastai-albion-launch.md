# vastai-albion-launch — extract + launch Albion Online on FR vast.ai VNC

## Role & workdir
Codex worker, workdir `/home/sdancer/vastai-albion`. The actual work happens on the remote FR vast.ai instance via SSH.

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-game-extracted-launched-verified-running`

## User request (verbatim, /talk#general 19:11)
> "yes, run the game"

(Follow-up to my status that the zip was downloaded but not extracted/launched.)

## Substrate already provisioned (DO NOT recreate)
- **vast.ai instance**: FR `37014838`, SSH `ssh -i ~/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai`
- **Albion installer zip**: `/home/albion/Downloads/albion-online-fullgame-linux.zip` (3.07 GB, 3076562098 bytes, completed 15:34)
- **KasmVNC** running at container :3000 (already exposed via Cloudflare tunnel)
- **albion user**: `uid=1000(albion)`, password set to `welcome to my w0rld`
- **xfce4 session** alive (`xfce4-session` PID 7527, `xfwm4` PID 7602)

## Success criteria
- Zip extracted to a sane location (e.g. `/home/albion/AlbionOnline/`) and contents accessible to the `albion` user.
- Albion launcher binary launched as the `albion` user (probably `AlbionLauncher.AppImage`, `Albion-Online`, or `./Launcher`).
- A new game-related process visible in `ps auxf | grep -iE 'albion|launcher'` (the launcher GUI process).
- VNC screenshot or capture demonstrates the launcher window is visible in the xfce4 desktop (a Kasm screenshot helper may exist; otherwise `xwd` + `convert` to PNG and scp back).
- Set fact `vastai_albion_game_running_2026_05_18=true` with the launcher PID and screenshot path.
- Verdict at `/home/sdancer/vastai-albion/analysis/albion_launch_verdict.md`. Final line `VASTAI_ALBION_LAUNCH_DONE`.

## Concrete tasks (do in order)

1. **Inspect the zip layout** without fully extracting (it's 3 GB):
   ```bash
   ssh -i ~/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai \
     "su - albion -c 'unzip -l /home/albion/Downloads/albion-online-fullgame-linux.zip | head -40'"
   ```
   Identify the top-level directory and the launcher binary.

2. **Extract**:
   ```bash
   ssh -i ~/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai \
     "su - albion -c 'cd /home/albion && unzip -q Downloads/albion-online-fullgame-linux.zip -d AlbionOnline 2>&1 | tail -5'"
   ```
   Confirm `/home/albion/AlbionOnline/` populated.

3. **Find launcher**:
   ```bash
   ssh -i ~/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai \
     "find /home/albion/AlbionOnline -maxdepth 4 -type f \\( -name '*.AppImage' -o -name 'Launcher' -o -name 'AlbionLauncher*' -o -name 'Albion-Online*' -o -name 'launcher.sh' \\) -executable 2>/dev/null"
   ```
   If executable bit is missing, chmod +x.

4. **Install runtime deps if needed**. Albion is a Unity game; typical deps:
   ```bash
   ssh -i ~/.ssh/id_ed25729 -p 14838 root@ssh8.vast.ai "apt-get list --installed 2>/dev/null | grep -E 'libgl1|libpulse0|libglu1|libsm6|libxext6|libxinerama1|libxrandr2|libfuse2' | head -10"
   # Install whatever is missing via apt-get -y install <pkg>.
   ```
   If running an `.AppImage`, need `libfuse2`. If running a plain ELF, need `libGL.so`, `libpulse.so.0`, etc.

5. **Launch under the VNC session's DISPLAY**:
   - The xfce4 session is on display `:1` (Xvnc :1 per the running command line: `/usr/bin/Xvnc :1 ...`).
   - Run as `albion` user with `DISPLAY=:1` and `XAUTHORITY=/home/albion/.Xauthority`:
   ```bash
   ssh -i ~/.ssh/id_ed25519 -p 14838 root@ssh8.vast.ai \
     "su - albion -c 'cd /home/albion/AlbionOnline && DISPLAY=:1 XAUTHORITY=/home/albion/.Xauthority nohup ./LAUNCHER_BINARY > /home/albion/albion-launch.log 2>&1 &'; sleep 8; ps -ef | grep -iE 'albion|launcher' | grep -v grep"
   ```
   Capture the launcher PID.

6. **Verify visibility**:
   - Try `xdotool search --name "Albion"` or `xwininfo -root -tree | grep -i albion` under the same DISPLAY.
   - If `xdotool` not installed, install it on the remote.
   - As a screenshot: `DISPLAY=:1 XAUTHORITY=/home/albion/.Xauthority xwd -root | convert xwd:- /tmp/albion_screenshot.png` and scp back to `/home/sdancer/vastai-albion/analysis/albion_screenshot.png`.

7. **Set fact + verdict**:
   - `harness fact-set vastai_albion_game_running_2026_05_18 "launcher PID <pid>, screenshot at <path>, KasmVNC desktop visible"`
   - Verdict at `analysis/albion_launch_verdict.md` (≤80 lines). Final line `VASTAI_ALBION_LAUNCH_DONE`.

## Falsification criteria

- **Zip is corrupted / extract fails** → re-download (`wget` from same source URL — look in the original log).
- **Launcher requires X session that auth fails on `:1`** → log in to xfce4 first via VNC (KasmVNC headers may need explicit user-session start).
- **Anti-cheat (BattlEye/EasyAntiCheat) refuses to start in VNC** → record the error and stop; cloud-VNC Albion may be impossible. Note in verdict.
- **Game launches but ANTI-CHEAT bans the cloud IP within seconds** → record the ban message; mark goal CLOSED-blocked-by-AC. **Do NOT spam re-launches.**

## Constraints & gotchas

- **All commands run as `albion` user via `su - albion -c '...'`**, NOT as root, for game install/launch. Game runs in the user's HOME with the user's DISPLAY.
- **DO NOT** modify the KasmVNC service config (it's already exposing :3000 to the tunnel).
- **DO NOT** disturb anything in `/etc/nmss/` or any nmss-related paths — this instance is purely Albion-side.
- **No nohup `&`** without redirecting stdin/stdout — Codex turn end can kill backgrounded children. Use a proper systemd unit if you want it durable past your turn:
  ```
  /etc/systemd/system/albion-launcher.service
  ```
  with `User=albion, Environment=DISPLAY=:1 XAUTHORITY=/home/albion/.Xauthority, ExecStart=...`.
- **Cloud-IP anti-cheat ban risk**: Albion uses BattlEye + their own AC. If banned, the cloud IP is burned for that account; not the worker's job to recover, just report.
- **Disk space**: 3 GB zip + 3-5 GB extracted = check `df -h /home` before extracting.

## Relevant files / references
- `~/.ssh/id_ed25519` — host SSH key (registered with vast.ai key id 612169)
- `/home/sdancer/.cloudflared_albion_token` — Cloudflare tunnel token (already deployed)
- Memory: `[[reference-vastai]]`, `[[project-albion-substrate]]`, `[[feedback-worker-artifact-isolation]]`
- Prior verdict: `/home/sdancer/vastai-albion/analysis/vastai_albion_relocate_verdict.md`
