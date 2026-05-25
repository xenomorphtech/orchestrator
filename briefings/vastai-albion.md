# vastai-albion — RELOCATE: Albion Online on vast.ai, NOT in China

## Role & workdir
Codex worker, workdir `/home/sdancer/vastai-albion`. You own the vast.ai instance lifecycle, OS setup, Albion install, and Cloudflare-tunnel web exposure.

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-on-vast-relocated-non-cn`

## User request (verbatim, cycle 1546)
> "our vast.ai instance is in china, delete and run on some other place"

The orchestrator already destroyed the prior CN instance (contract `37007244`, machine 68196) and killed the orchestrator-side SSH forward (PID 104758). **You start from zero — 0 active instances.**

## Success criteria
- Brand-new vast.ai instance running in a **non-CN** geography (prefer EU first — DE/NL/FR — then US; reject CN, RU, IR).
- SSH reachable from the orchestrator host: `ssh -i ~/.ssh/id_ed25519 -p <port> root@<ssh-host>`.
- Cloudflare tunnel from `~/.cloudflared_albion_token` running inside the container; KasmVNC (or similar) accessible via the tunnel URL.
- Optional stretch: Albion Online Linux client installed and launchable in the VNC desktop.
- Done = fact `vastai_albion_web_live_relocated_2026_05_18` set with the public URL + instance ID + country.
- Verdict at `analysis/vastai_albion_relocate_verdict.md`. Final line `VASTAI_ALBION_RELOCATE_DONE`.

## Substrate / credentials (already provisioned)
- **vast.ai API key:** `/home/sdancer/.vastai_api_key` (mode 600). CLI at `~/.local/bin/vastai`. If not configured: `~/.local/bin/vastai set api-key $(cat ~/.vastai_api_key)`.
- **Cloudflare tunnel token:** `/home/sdancer/.cloudflared_albion_token` (mode 600). Read host-side, pass via env, do NOT bake into onstart-cmd.
- **SSH key:** `~/.ssh/id_ed25519` (`sdancer@navi`) is registered with vast.ai key id 612169.

## Concrete tasks (do in order)

1. **Search for offers in non-CN geographies.** Target specs: >=8 cores, >=16GB RAM, GPU with >=6GB VRAM (RTX 3060/A2000-class fine), Linux, public direct port range. Prefer geolocation in {DE, NL, FR, GB, US, CA}; explicitly exclude CN, RU, IR. Target dph_total <= $0.20/hr. Verified machines preferred.
   ```bash
   ~/.local/bin/vastai search offers \
     'reliability > 0.95 dph<0.20 num_gpus=1 gpu_ram>=6 cpu_cores>=8 cpu_ram>=16 cuda_max_good>=12.0 verified=true geolocation!=CN' \
     -o 'dph+'
   ```
   Pick the cheapest acceptable offer. **Verify the country field on the chosen offer is not CN/RU/IR** before committing.

2. **Create instance** with the same image pattern that worked before (`nvidia/cuda:12.6.0-base-ubuntu22.04`) and onstart-cmd to install desktop pieces and start sshd:
   ```bash
   ~/.local/bin/vastai create instance <offer-id> \
     --image nvidia/cuda:12.6.0-base-ubuntu22.04 \
     --disk 32 \
     --ssh \
     --onstart-cmd 'apt-get update && apt-get install -y openssh-server curl wget xfce4 xfce4-goodies xfce4-terminal dbus-x11 mesa-utils && service ssh start'
   ```
   Record the new contract id.

3. **Wait for SSH ready.** Poll `vastai show instances --raw` until `actual_status==running` AND `ssh_host`+`ssh_port` are populated. Then test:
   ```bash
   ssh -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -i ~/.ssh/id_ed25519 -p <port> root@<host> 'uname -a; cat /etc/os-release | head -3'
   ```

4. **Install + start cloudflared inside the container** (read token host-side, env-pass it):
   ```bash
   CF_TOKEN="$(cat ~/.cloudflared_albion_token)"
   ssh -i ~/.ssh/id_ed25519 -p <port> root@<host> "CF_TOKEN='$CF_TOKEN' bash -s" <<'REMOTE'
     curl -fL https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o /usr/local/bin/cloudflared
     chmod +x /usr/local/bin/cloudflared
     nohup /usr/local/bin/cloudflared tunnel --no-autoupdate run --token "$CF_TOKEN" >/var/log/cloudflared.log 2>&1 &
     sleep 5; tail /var/log/cloudflared.log
REMOTE
   ```

5. **Install KasmVNC** (or equivalent web-VNC) inside the container so the tunnel has something to serve. Default KasmVNC port 3000 or 6901 depending on install — confirm whichever the prior Cloudflare ingress was set to (try both). If KasmVNC's apt path is awkward, fall back to `noVNC + Xvfb + x11vnc + xfce4` (heavier).

6. **Verify the public tunnel URL** routes to the in-container web UI. The Cloudflare tunnel ingress is pre-configured server-side (per the prior cycle 1436 setup); you don't need to add new DNS routes. Just confirm the local listener exists on the port the tunnel expects.

7. **Set up orchestrator-side ssh forward** (so localhost:38080 -> container:3000 even without Cloudflare) — but use a systemd unit, NOT a bare nohup, per memory `worker-artifact-isolation`. If creating a new systemd unit is too invasive in your turn, document the manual `ssh -fN -L 38080:127.0.0.1:3000 ...` command and have the orchestrator run it after the worker completes.

8. **Set fact + write verdict.** `harness fact-set vastai_albion_web_live_relocated_2026_05_18 "instance=<id> country=<XX> tunnel=<url>"` + verdict at `analysis/vastai_albion_relocate_verdict.md`. Final line `VASTAI_ALBION_RELOCATE_DONE`.

## Constraints & gotchas
- **NO CN/RU/IR instances.** User explicitly asked for relocation away from CN. Verify country before `create instance`.
- **No money waste:** if you have to recreate the instance, destroy the old one first (`vastai destroy instance <id>`).
- **Image-tag verification BEFORE create:** before `vastai create instance`, check the tag exists on Docker Hub. Wrong tag = manifest unknown error and wasted destroy/recreate.
- **Tunnel token is secret.** Don't paste into git/PRs.
- **Don't burn nmss device:** This worker has nothing to do with the RK3588 at `localhost:5558`.
- **Anti-cheat consideration:** Albion has anti-cheat. Cloud IPs sometimes get flagged. Install + launch is the spec; if banned, document and stop.

## Relevant files / references
- `/home/sdancer/.vastai_api_key` — API key
- `/home/sdancer/.cloudflared_albion_token` — Cloudflare tunnel token
- Memory: `[[reference-vastai]]`, `[[worker-artifact-isolation]]`, `[[project-albion-substrate]]`
