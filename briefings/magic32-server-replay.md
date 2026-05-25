# magic32-server-replay — move snapshot replay to the arm64 server

## Role & workdir
Codex worker, workdir `/home/sdancer/nmss-emu-magic32-server-replay` (branch `magic32-server-replay`, forked from fee16ae = Lane G).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `magic32-server-replay-via-arm64-host`

## Why this path
Lane G (commit fee16ae) proved the cert function runs end-to-end on a fresh live thered snapshot — but produces deterministic-wrong cert A1A54291 for challenge A80CFDEF30357F22 (expected Token 82B9E000299075AA…). The SIGILL trampoline returns 0 where the cert pipeline expects MAGIC32-derived data.

Lane H spent 9 cycles trying to *instrument* the trampoline from inside the corrupted-libc replay process. The recording infrastructure keeps breaking libc invariants after cert success. The structural fix is to **move replay off the RK3588 device onto the arm64 server**, where:
- `gdb` works freely (no NMSS-by-proximity, no thered to disturb — snapshot is just `.bin` files + `maps.txt`)
- 62 GB RAM + 44 GB free disk (vs RK3588's ~8 GB) — full snapshot easily resident
- Standard Linux toolchain (no Android constraints)

The replay binary (`tooling/lane_a_call_site`) is `aarch64-linux-gnu-gcc -static`. It runs on any aarch64 Linux box. Substrate-wise the device-vs-server transition is free.

## Hypothesis
Running Lane G's replay binary on the arm64 server **with gdb attached** lets us single-step from `br x3` and identify which of the 7 SIGILL trampoline call-sites is the PGS-player-ID / MAGIC32 retrieval path by reading instructions and symbols *before* the fault, not after. Substituting the correct value at that call site produces cert == Token for ≥10/14 captured wire pairs.

## Falsification criteria (any one)
- Replay on arm64 server runs and reproduces correct cert for ≥10/14 wire pairs (after identifying the MAGIC32-fetch call and substituting the right value) → **MAGIC32-equivalent recovered, goal closes 5/6 → 6/6**.
- Replay on arm64 server reproduces same A1A54291 deterministic wrong cert as on-device (substrate consistency confirmed), AND gdb walk through cert function identifies that ≥1 of the 7 trampolined dispatches crosses a process boundary (Binder IPC to GMS) — meaning no in-snapshot value will satisfy it → escalate to user: need cross-process snapshot (thered + GMS).
- Server-replay diverges from device-replay (different fault, different cert output) → snapshot-relocation bug; debug and retry.

## Hard rules
- **arm64 server: `root@162.244.80.97`** (62 GB RAM, aarch64, 44 GB disk free). Already runs `oracle-service` on port 9876 — don't disturb that.
- **Snapshot source: RK3588 device at adb localhost:5558**, path `/data/local/tmp/live_snapshot_20260518_103654/` (2.7 GB, 4528 regions, manifest 1.9 MB, fact `magic32_live_snapshot_captured_2026_05_18`).
- **NO modifications to live thered.** Snapshot is read-only artifact. Replay is offline.
- **NO `pm clear`** on thered (standing rule).
- **Lane G binary parent**: commit fee16ae on this branch. Build with `aarch64-linux-gnu-gcc -O2 -static`. Source: `tooling/lane_a_call_site.c`.
- **Wall budget**: ~3h. This path is the de-blocked successor to Lane H.

## Step 1 — relocate snapshot
1. `adb -s localhost:5558 pull /data/local/tmp/live_snapshot_20260518_103654 /tmp/live_snapshot_20260518_103654` (or pull individual files if it's faster — `manifest.json` is essential, `*.bin` files referenced by manifest).
2. `rsync -av --info=progress2 /tmp/live_snapshot_20260518_103654/ root@162.244.80.97:/data/snapshots/live_20260518_103654/`.
3. Verify manifest + a sample region file SHA on both ends.

## Step 2 — port lane_a_call_site to the server
1. `scp tooling/lane_a_call_site root@162.244.80.97:/data/snapshots/`. The binary is statically linked aarch64, should run as-is.
2. SSH into server. Quick smoke: `/data/snapshots/lane_a_call_site --help` (or check argv parsing). Confirm it reads `maps.txt` + `.bin` files from the snapshot dir.
3. Run end-to-end: same args worker has been using on the RK3588 (`. A80CFDEF30357F22 --fptr 0x746fff... [other flags]`). Confirm cert output. Expected: A1A54291CAFB468DF7764B6BB0759D5BC0B3D880C780AD66 (same wrong-but-deterministic value as on-device). If different → snapshot-portability bug; debug.

## Step 3 — gdb walk of cert function
1. `apt install gdb-multiarch` if needed (server is aarch64 Linux, so `gdb` should work natively).
2. Run `lane_a_call_site` under gdb. Set a breakpoint at the `br x3` site (just before cert entry). The address comes from `cert_fn_entry_in_rwxp_gap_region_2026_05_18` fact (entry 0x7468d2d75c on RK3588 mapping; relocated address on server).
3. Single-step `stepi` from cert entry. When the first SIGILL would fire, examine the actual target address (`x16`, `x17`, or the value loaded into the dispatch register). Use the snapshot's `maps.txt` to identify what should have been mapped at that address.
4. Cross-reference snapshot's libnmsssa.so file-cache: use `addr2line` against the *snapshot's* `r-xp` regions of libnmsssa to get a symbol name for the caller (the LR at SIGILL time).
5. Repeat for each of the 7 SIGILL trampoline sites. Build a table: `(call_index, caller_LR, caller_sym, target_addr, target_should_be)`.

## Step 4 — classify each call
For each of the 7 trampolined calls, determine which of:
- (a) **C++ vtable dispatch** to a method inside a stale heap object. Look for the dispatch source: `blr x16` after `ldr x16, [x0, #imm]`. The caller's frame holds the object pointer.
- (b) **JNI/ART bridge** — `Java_com_*` function pointer table. These cross into ART memory which our snapshot doesn't have. Symbol prefix `_JNIEnv_*` or `art::`.
- (c) **PGS / Google Play Services** — `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` per fact `magic32_pgs_jni_entry_located_2026_05_18`. This is THE MAGIC32 source.
- (d) **AES decrypt / cached blob unwrap** — operations on libnmsssa rw segment data. v17 confirmed no plaintext MAGIC32 in libnmsssa rw, but the encrypted form might be there.

Goal: identify which of the 7 calls is (c) or (d).

## Step 5 — substitute and validate
1. If (c) identified: a known PGS player ID for this install is needed. May be retrievable from app data on RK3588 (Android `SharedPreferences` in `/data/data/com.netmarble.thered/...`) or from a captured login response.
2. If (d) identified: the cached blob is somewhere in snapshot. Find via static analysis of the caller (which pointer is being dereferenced).
3. In `lane_a_call_site.c`, replace the SIGILL trampoline's default `mov x0,#0` with a return-the-right-value path for the identified call. Rebuild, run, check cert.
4. If cert == Token for challenge A80CFDEF30357F22 → run all 14 captured wire pairs (see `cert_hw_bp_v*_verdict.md` in `/home/sdancer/nmss-emu-cert-hw-bp/analysis/`). ≥10/14 match → success.

## Step 6 — verdict + commit
Single commit on `magic32-server-replay`, message `magic32-server-replay: <verdict>`. Verdict at `analysis/magic32_server_replay_verdict.md`. Final line `MAGIC32_SERVER_REPLAY_DONE`. On success: emit MAGIC32 substitution recipe, document the cert-replay-as-service architecture for downstream consumers, set `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`.

## Constraints & gotchas
- The relevant fact tray (cross-pollination):
  - `lane_g_committed_2026_05_18` — cert pipeline runs end-to-end on this snapshot, output A1A54291 for challenge A80CFDEF30357F22.
  - `lane_g_trampoline_is_magic32_callsite_2026_05_18` — ablation shows trampoline IS in MAGIC32 path.
  - `cert_algorithm_matches_cycle_141_142_recipe_2026_05_18` — cert recipe documented and 5/5 verified on old snapshot.
  - `cert_hw_bp_v13_magic32_install_keyed_2026_05_18` — MAGIC32 is install-keyed (AES-encrypted PGS player ID).
  - `magic32_pgs_jni_entry_located_2026_05_18` — `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` at libUnreal.so file offset 0x0581609c.
  - `aeon_trace_kernel_toolkit_found_2026_05_18` — sg.orch.run:~/aeon-trace-kernel/kernel has libchild_seccomp.so + fork_helper.js + nmss_helper.ko. If this server-replay path falsifies on (c)-cross-process, that toolkit is the backup architecture (fork from live thered, inherit perfect state, run cert in child).

## Relevant files / references
- `/home/sdancer/nmss-emu-magic32-server-replay/` — this worktree
- `/home/sdancer/nmss-emu-magic32-live-snapshot-replay/analysis/lane_g_verdict.md` — Lane G's 225-line write-up of the SIGILL trampoline mechanism
- `/home/sdancer/nmss-emu/cert-rust-repro/` — the 5/5 verified Rust reproducer (recipe reference)
- `/home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v*_verdict.md` — 14 wire pairs
- Ground truth challenge=`A80CFDEF30357F22`, expected Token=`82B9E000299075AA2C2A070A370BFA01…`
- Memory: `feedback_kernel_instrumentation` (kernel-side OK), `proc_mem_read_substrate_validated_nmss_undetected_2026_05_18` (the snapshot capture path is NMSS-safe)
