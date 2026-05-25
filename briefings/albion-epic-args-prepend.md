# albion-epic-args-prepend

## Role & workdir

Continuation-divergence path after `albion-native-password-mode` was falsified. That sibling proved `+AUTH_TYPE password +AUTH_LOGIN ... +AUTH_PASSWORD ...` argv form trips the password login branch (`LoginScreen → GameLoading`) but Albion then crash-loops in `EpicOfflineDialog.Update() NullReferenceException` because the Epic-published build expects Epic SDK init context that bare `password` mode doesn't provide.

Your hypothesis: prepending the **Epic SDK argv keys** (`+epicapp +epicdeploymentid +epicsandboxid` — also in the IL2CPP token cluster at metadata offsets ~330718) with values pulled from the binary's own embedded defaults satisfies Epic SDK init, lets GameLoading advance to WorldEntry, and zone flips non-null.

Workdir: `/home/sdancer/albion-epic-args-prepend` (git worktree, branch `albion-epic-args-prepend`).

## Already proven (do NOT re-falsify)

| Level | Artifact | Status |
|---|---|---|
| L1 | argv selector identified | ✅ `AUTH_TYPE`, `AUTH_LOGIN`, `AUTH_PASSWORD` keys at IL2CPP offsets 330718/330729/330743 |
| L2 | plus-prefix form works | ✅ `+AUTH_TYPE password ...` moves Albion LoginScreen → GameLoading |
| L3 | injection point known | ✅ `exec ./Albion-Online` in `/opt/albion-frida-capture/spawn_preload.sh` |
| L_a | real-creds run blocker | ❌ Epic SDK NRE in `EpicOfflineDialog.Update()` — your target |

## Hypothesis

**H**: Albion's IL2CPP metadata contains `epicapp`, `epicdeploymentid`, `epicsandboxid`, `epicrefreshtoken`, `epicusername` argv keys adjacent to the AUTH_* keys (see `/home/sdancer/albion-native-password-mode/analysis/native_password_mode_task1_2026-05-23.md`). If we extract the embedded default Epic identifiers (sandbox/deployment GUIDs are typically embedded in the binary) and pass them via plus-prefix argv along with `+AUTH_TYPE password +AUTH_LOGIN <email> +AUTH_PASSWORD <pw>`, the Epic SDK initializes cleanly, the NRE path is skipped, and the client advances past GameLoading.

## Current goal / sub-goal

- `goal_key`: `albion_action_loop`
- `sub_goal_key`: `epic_args_satisfy_sdk_init`
- Success: dashboard `https://albion.orch.run/state` `zone != null` after launching with `+epicapp ... +epicdeploymentid ... +epicsandboxid ... +AUTH_TYPE password +AUTH_LOGIN <email> +AUTH_PASSWORD <pw>`.

## Falsification

If, after 3 turns of argv combination testing, Albion still hits `EpicOfflineDialog.Update() NullReferenceException` OR fails to advance from GameLoading to WorldEntry, this path is closed. Write `/home/sdancer/albion-epic-args-prepend/analysis/epic_args_blocked.md` and exit.

## Next 3 concrete tasks

1. **Extract Epic identifier defaults from binary.** Inspect `/home/albion/albion-online/Albion-Online_Data/il2cpp_data/Metadata/global-metadata.dat` AND `/home/albion/albion-online/Albion-Online_Data/Managed/Assembly-CSharp.dll` (or the IL2CPP .so) for:
   - GUID-shaped strings adjacent to `epicapp`, `epicdeploymentid`, `epicsandboxid` literals
   - Public Epic app IDs for Albion Online (these are publicly published — `prod-fn` shape) — if not embedded as defaults, the canonical IDs may be derivable from public Albion launcher docs/forums; do NOT contact external services, just inspect the binary
   - Document exact byte offsets of any candidate values

2. **Patch spawn_preload.sh with prepended epic args.** Backup baseline (existing simple `exec ./Albion-Online` form) before patching. Modify to:
   ```
   exec ./Albion-Online +epicapp <val> +epicdeploymentid <val> +epicsandboxid <val> +AUTH_TYPE password +AUTH_LOGIN $ALBION_LOGIN +AUTH_PASSWORD $ALBION_PASSWORD
   ```
   Source `$ALBION_LOGIN` and `$ALBION_PASSWORD` from `/home/albion/accountportal-headed/accountportal.env`. DO NOT echo secrets.

3. **Trigger child restart, watch Player.log for 5 min.** Look for:
   - `LastKnownState` transitions: GameLoading → WorldEntry (or similar non-null state)
   - Absence of `EpicOfflineDialog.Update()` NRE lines
   - `Attempting to log in via 'password'` log line
   
   Poll `https://albion.orch.run/state` via `curl` every 10s. If `zone != null`: fact-set `albion_epic_args_prepend_success_2026-05-23` with timestamp + arg combo used. Else write `epic_args_blocked.md` with sanitized evidence + restore launcher.

## Constraints & gotchas

- **DO NOT** echo Albion credentials or Epic identifiers in stdout/logs/chat.
- **DO NOT** create chromium browsers; this path is binary-args-only.
- **DO NOT** kill `albion-supervise` (PID 443749) — only the Albion-Online child.
- **DO NOT** test against external Epic Online Services endpoints; Albion's bundled Epic auth should be sufficient for client init.
- **Backup launcher** before any patch; restore on path-close.
- Substrate: `ssh -p 14838 -o StrictHostKeyChecking=no root@ssh8.vast.ai`.
- Time budget: **45 min per turn**.
- Sibling photon-sdk-research and photon-login-fuzz are running concurrently on different angles — don't touch their worktrees.

## Relevant files / references

- Closed sibling artifact: `/home/sdancer/albion-native-password-mode/analysis/native_password_mode_task1_2026-05-23.md` (full argv key inventory + injection point evidence)
- Closed sibling postmortem: `/home/sdancer/albion-native-password-mode/analysis/password_mode_blocked.md` (NRE diagnosis)
- Closed sibling attempt log: `/home/sdancer/albion-native-password-mode/analysis/password_mode_attempt_log.md` (3 argv forms tested)
- Fact: `albion_password_mode_epic_sdk_gap_2026-05-23` (root cause)
- Albion binary: `/home/albion/albion-online/Albion-Online`
- Game assembly: `/home/albion/albion-online/Albion-Online_Data/Managed/Assembly-CSharp.dll`
- IL2CPP metadata: `/home/albion/albion-online/Albion-Online_Data/il2cpp_data/Metadata/global-metadata.dat` (token cluster at offset 330718+)
- Spawn preload: `/opt/albion-frida-capture/spawn_preload.sh`
- Creds env: `/home/albion/accountportal-headed/accountportal.env`
- Player.log: `/home/albion/.config/unity3d/Sandbox Interactive GmbH/Albion Online Client/Player.log`
- Dashboard: `https://albion.orch.run/state`
