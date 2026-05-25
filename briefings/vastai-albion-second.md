# vastai-albion-second — A/B substrate test

## Role & workdir
Provision a SECOND vast.ai instance and bring up Albion on it to test whether the Albion freeze observed on instance `37014838` (ssh8.vast.ai:14838) is substrate-specific or reproduces on a fresh box.

**Workdir**: `/home/sdancer/vastai-albion-second`

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-second-substrate-ab-test`

## Hypothesis & falsification
**Hypothesis**: The Unity netcode post-auth wedge / pre_init_spin observed on instance 37014838 is substrate-specific (display stack / driver / kernel-side). A second freshly-provisioned vast.ai instance of similar specs will NOT exhibit the freeze, demonstrating the issue is local.

**Falsification**: Albion freezes the same way on the second instance — substrate hypothesis killed, problem is server-side or account-side or universal.

## Success criteria
- Second vast.ai instance up and reachable (SSH or KasmVNC).
- Albion installed and launched on that instance.
- Within 15 min of launch, classify behavior: `frozen-pre-init` / `frozen-post-auth` / `login-screen-renders-clean` / `in-world`.
- Write a 1-page comparison memo at `/home/sdancer/orchestrator/analysis/albion_substrate_ab_compare_2026-05-20.md`.
- Set fact `albion_ab_test_result_2026_05_20` with the classification.

## Reference: current instance specs (match these)
- Instance ID: `37014838`
- Image: `nvidia/cuda:12.6.0-base-ubuntu22.04`
- GPU: GTX 1660 S (or anything with `/dev/dri/renderD128`-equivalent)
- Cost: $0.086/hr
- SSH: ssh8.vast.ai:14838 root, key `~/.ssh/id_ed25519` (vast.ai key 612169)
- Display stack: Xvnc :1 + xfce4-session, KasmVNC `kasmvnc.yaml` has `gpu.hw3d: false` (llvmpipe software rasterizer).
- Init: `/.launch` → `/root/onstart.sh` (no systemd as PID 1 in container)
- Albion install path on first instance: `/home/albion/albion-online/` running as user `albion`

See `/home/sdancer/orchestrator/analysis/vastai_albion_instance_setup.md` for the full first-instance writeup — replicate that setup on the second instance as closely as possible.

## Next concrete tasks (in order)

1. **Rent the second instance** — use `~/.local/bin/vastai` (api key already configured at `/home/sdancer/.config/vastai/vast_api_key`). Find a similar offer with `vastai search offers 'cuda_max_good >= 12.0 num_gpus = 1 inet_down >= 100 dlperf >= 5'` (adjust filters as needed), then `vastai create instance <ASK_ID> --image nvidia/cuda:12.6.0-base-ubuntu22.04 --disk 32 --label albion-ab-second --ssh`. Budget cap: pick the cheapest under $0.30/hr. Wait for `actual_status: running`.

2. **Bootstrap the display stack** — SSH in, install: `apt-get update && apt-get install -y xfce4 xfce4-terminal kasmvncserver xfce4-session dbus-x11`. Create user `albion`, set up Xvnc :1 with the same flags as instance 37014838 (check via `ps -ef | grep Xvnc` on the first instance for the exact args — DO NOT alter the first instance, just read). Start Xvnc + xfce4-session as `albion`.

3. **Install Albion** — Albion installer lives at https://live.albiononline.com/ (Linux launcher). Download to `/home/albion/`. Run installer. Disposable email account ok if needed but for the A/B test we just need to launch the client — no need to log in if freeze occurs at the login screen.

4. **Launch and observe** — `runuser -l albion -s /bin/bash -c "cd /home/albion/albion-online && DISPLAY=:1 XAUTHORITY=/home/albion/.Xauthority setsid nohup ./Albion-Online > /tmp/albion1.log 2>&1 < /dev/null &"`. Poll: every 30s, capture CPU%, RSS, etime, `xdotool search --name 'Albion Online'`. Record observations to `/tmp/ab_observations.log`.

5. **Classify and write up** — after 15 min (or sooner if classification is clear), kill the new instance via `vastai destroy instance <ID>` to stop billing UNLESS the result is "freezes the same" — in that case keep it alive for further inspection. Write the comparison memo. Set fact.

## Constraints & gotchas
- **DO NOT** touch instance 37014838 — it is the current production substrate driving loop #5 on PID 3401178.
- **Budget**: keep the second instance at $0.30/hr or less, total session cap $5. Destroy when done.
- **No Frida**, no kernel modules on the rented box (it's a container, not a host).
- **No software-GL toggle** on the second instance either — match the first instance's stack (llvmpipe via Xvnc default).
- Vast.ai containers have **no systemd as PID 1**. Init is `/.launch` running a bash script. Don't try `systemctl start xvnc`.
- The Albion launcher will likely auto-update — bandwidth caveat (could be 5-10 GB download).
- If the rented instance has too-different GPU/driver, that's a confound for the A/B test — try once with a similar GPU, and if the result is ambiguous, retry on a CPU-only instance.

## Relevant files / references
- `/home/sdancer/orchestrator/analysis/vastai_albion_instance_setup.md` — first-instance setup writeup
- `/home/sdancer/.vastai_api_key` — API key (mode 600)
- `/home/sdancer/.config/vastai/vast_api_key` — vastai CLI config
- `/home/sdancer/.ssh/id_ed25519` — SSH key (vast.ai key 612169)
- Memory: `[[albion-client-wedge-class]]`, `[[albion-substrate]]`, `[[albion-waydroid-works]]`, `[[vastai]]`
- First-instance Albion creds (for reference, NOT to commit): `/home/albion/.albion_credentials.txt` on 37014838 (but A/B test doesn't need to log in — freeze classification at login screen is sufficient)

## Reporting cadence
Update `/home/sdancer/orchestrator/analysis/talk_channels/albion.jsonl` (`{"ts":"<ISO>","from":"vastai-albion-second","text":"<short status>"}`) every 5 min while bring-up is active, then on classification.
