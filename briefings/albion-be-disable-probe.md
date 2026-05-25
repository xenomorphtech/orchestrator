# albion-be-disable-probe — env/CLI probe to neutralize BattlEye input filter

## Role & workdir
Fresh Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-be-disable-probe`. Live target: vast.ai container via `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`.

## Current goal / sub-goal
**goal_key:** `albion_action_loop`. **sub-goal:** **determine whether documented BattlEye/Unity launch flags or env vars can disable the BattlEye-mediated TMP_InputField input filter without modifying GameAssembly.so**, so synthesized XTest events at the 2FA modal become accepted, 2FA auto-submitted, and self.zone → non-null.

## Big-picture rationale
Five autonomous-bypass classes have now been rigorously falsified for this goal (cycles 3225-3268): (1) input-substrate LD_PRELOAD on libX11+libXi; (2) prior-session photon-recv token mining; (3) prefs config-flip; (4) launcher CLI flags; (5) UnityPlayer/boot.config/GameAssembly login-mode RE. The cycle-3247 mechanism conclusion was that **Unity reads input via a path past BOTH libX11 XEvent dequeue AND libXi XI2 cookies** — most likely raw evdev, IL2CPP-internal queue, OR BattlEye-mediated interception. BattlEye is present on the container (`BEClient_x64.so`). The K=6 planner at cycle 3268 ranked this hypothesis as **score 13** (top), cost 2h, reversibility 5 (env vars only, no binary tamper, no ban risk).

Hypothesis: If BattlEye IS the actual TMP_InputField filter, neutralizing its init via documented launch flags or env vars will restore XTEST acceptance on the 2FA modal. A positive result delivers self.zone via the cheapest possible substrate. A negative result cleanly eliminates the BattlEye-mediated branch and frees the next planner-ranked path (`albion-egress-ip-pinning-proxy`, score 12) to take primacy with higher confidence.

## Already achieved (do not re-falsify)
| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `/home/sdancer/albion-x11-shim/analysis/shim_verdict.md` (cycle 3247) | XTEST events DO reach the X server, libX11+libXi LD_PRELOAD doesn't help, filter is past Xlib/XI2 | ✅ DONE |
| 2 | `/home/sdancer/albion-token-capture/analysis/unityplayer_seams_audit.md` (cycle 3268) | No CLI/env/boot.config/GameAssembly auth-selector seam exists | ✅ DONE |
| 3 | Photon login server is `loginserver.live.albion.zone:5055` UDP; Albion supports 4 login modes (`password`, `refreshToken`, `exchangecode`, `accountportal`) | from cycle-3212 binary recon | ✅ DONE |
| 4 | `BEClient_x64.so` is mapped into live Albion process per lsof inventory | BattlEye is structurally present | ✅ DONE (verify via lsof first) |

## Success criteria
1. **Positive verdict**: Env var `X` and/or launch flag `Y` applied + Albion restart → 2FA modal accepts XTEST input → 2FA code typed + submitted → `self.zone != null` on `/state` within 5 min OR token-watcher fires. Capture fact `albion_be_disable_env_works_2026_05_22 = <env-var-name>`.
2. **Negative verdict** (more likely): all documented BE-disable env vars enumerated and tested → either BE refuses to launch (Albion returns to login screen) OR launches but XTEST pixel-diff still 0 on 2FA modal. Write `analysis/be_disable_verdict.md` with Achievement-levels-+-gaps framing. Cleanly eliminates BattlEye-mediated branch.
3. **No regression**: 5 production daemons (cloudflared, gamestate_service, photon-pcap-send, albion-frida-ingest, Albion-Online) come back up healthy after each restart.

## Tasks (sequential, 2h hard timeout)

### Task 1 — sense the substrate
1. `ssh` to container. Confirm `BEClient_x64.so` is currently mapped into `pidof Albion-Online`: `lsof -p $(pidof Albion-Online) | grep -i battl`. Capture lsof output to `analysis/be_lsof_baseline.txt`.
2. `strings /home/albion/albion-online/Albion-Online_Data/Plugins/x86_64/BEClient_x64.so | head -200` (or wherever BEClient lives — find first via `find /home/albion -iname 'BEClient*'`). Save to `analysis/be_strings.txt`. Look for documented BE-disable / launcher-skip strings.
3. Check the current `/usr/local/bin/run-albion-client` wrapper for any BE-related env vars or launch flags already set. Document what's there.

### Task 2 — enumerate candidate env vars + launch flags
From research on Unity-BattlEye games (known by community), enumerate the candidate set:
- Env vars: `BE_DISABLE=1`, `BATTLEYE_DISABLE=1`, `NO_BATTLEYE=1`, `DISABLE_BATTLEYE=1`, `BE_BYPASS=1`, `BattlEye_Disable=1`, `STEAM_BATTLEYE_DISABLE=1`, `UNITY_DISABLE_ANTICHEAT=1`
- Launch flags (passed to `./Albion-Online`): `-nobattleye`, `--no-battleye`, `-norestriction`, `--no-anticheat`, `-noBattlEye`, `-nobe`, `--no-easyanticheat-launcher`, `EAC_BYPASS=1`

Save the FULL list with sources/justification to `analysis/be_candidates.md`. Then for each candidate, classify whether it can be applied:
- (a) at wrapper level (modify `spawn_preload.sh` to set env), OR
- (b) at launch-arg level (modify `run-albion-client` to add CLI flag)

### Task 3 — controlled experiments (one at a time)
For each top-3 candidate from Task 2:
1. Note current `/state` baseline.
2. Modify `spawn_preload.sh` OR `run-albion-client` to apply the candidate. **Back up** original to `.bak.<timestamp>` first.
3. Restart only `albion-client` tmux session (per the existing recipe). DO NOT restart other 4 daemons.
4. Wait 30s. Capture screenshot from Xtigervnc :3 via `xwd -display :3 -root | convert ... png:` (best-effort — known black-frame on DRI per fact `screenshot_503_root_cause_2026_05_22`, so use xdotool getactivewindow + window-id-specific capture if needed).
5. Run a controlled XTEST input attempt at the 2FA modal: cancel-click at known coords (782, 687) OR field-click + type "PRV6T7E" + OK click. Capture pre/post screenshots. Diff via cv2.
6. **Crucial diagnostic**: `lsof -p $(pidof Albion-Online) | grep -i battl` — verify BE is or isn't loaded under this env/flag.
7. Restore original wrapper after experiment. Restart `albion-client` to confirm baseline restored.
8. Record per-experiment result in `analysis/be_disable_verdict.md` with template:
   ```
   ## Experiment N: candidate=<name>
   - Applied: <where>
   - BE in lsof after restart: <yes/no>
   - Pixel diff (cancel-click): <value>
   - Pixel diff (field-type-submit): <value>
   - Conclusion: <accepts XTEST | still filtered | BE refused launch>
   ```

### Task 4 — write verdict
Write `analysis/be_disable_verdict.md` with full Achievement-levels-+-gaps framing. Each level marks one of:
- Level 1: BE confirmed in baseline lsof
- Level 2: enumeration of candidates complete
- Level 3: experiments run (≥3 candidates tested)
- Level 4: any single candidate produced non-zero pixel diff on 2FA modal
- Level 5: full E2E — 2FA submitted, self.zone non-null

If Level 4 hits → set fact `albion_be_disable_env_works_2026_05_22=<name>` and notify orchestrator. If only Level 1-3 hit → declare path closed cleanly, suggest next-best per planner ranking (`albion-egress-ip-pinning-proxy`).

## Constraints & gotchas
- **2-hour hard timeout** — do not extend beyond this. If Task 3 isn't producing positive results after 3 candidates, write verdict-negative and stop.
- **NEVER modify GameAssembly.so or BEClient_x64.so binaries** — pure env/CLI probe only. Binary tamper = BattlEye ban risk.
- **NO Frida** anywhere — per [[no-frida]] memory (anticheat detection).
- **photon_tap.so MUST stay `-DDISABLE_SEND_HOOKS`** per [[albion-send-hooks-break-client]]. Do not touch the photon_tap.so build.
- **Token-watcher must remain armed and untouched** — `systemctl status albion-token-watcher.service` should report active/running throughout. Verify before and after each experiment.
- **Restore wrapper after each experiment** — even if positive result, restore original BEFORE writing verdict, so the production state is back to baseline.
- **Other 4 daemons must stay alive**: cloudflared, gamestate, ingest, pcap. Only restart `albion-client` tmux session.
- **Hard-clean restart hazard**: cycle-3220 history shows `pkill` patterns can match the supervisor process and kill the SSH session. Use specific patterns like `tmux kill-session -t albion-client` only.

## Relevant files / references
- Live container: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- BattlEye install: `/home/albion/albion-online/Albion-Online_Data/Plugins/x86_64/BEClient_x64.so` (TBC via find)
- Wrapper chain: `/usr/local/bin/run-albion-client` → `/opt/albion-frida-capture/spawn_preload.sh` → `./Albion-Online`
- Prior input-substrate falsification: `/home/sdancer/albion-x11-shim/analysis/shim_verdict.md`
- Prior UnityPlayer audit: `/home/sdancer/albion-token-capture/analysis/unityplayer_seams_audit.md`
- Token-watcher (UNTOUCHED): `albion-token-watcher.service` + `/home/sdancer/albion-token-capture/scripts/watch_for_2fa_complete.py`
- Talk channel: `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl`
- Memory pointers: `[[no-frida]]`, `[[albion-send-hooks-break-client]]`, `[[albion-vastai-daemon-stack]]`, `[[orchestrator-role]]`, `[[macromanage-workers]]`.

## Reporting
Final deliverable: `analysis/be_disable_verdict.md` with Achievement-levels-+-gaps. If positive → fact `albion_be_disable_env_works_2026_05_22=<name>` + zone-change observation + episode milestone. If negative → declare path closed cleanly + suggest next-best (egress-IP-pinning per planner ranking). Concise progress at each task boundary; do not narrate every shell command. Hard cap 2h total wall time.
