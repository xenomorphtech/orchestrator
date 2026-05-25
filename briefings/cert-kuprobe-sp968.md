# cert-kuprobe-sp968 — extend LKM v1 substrate with uprobe at libnmsssa+0x17ee1c

## Role & workdir
Kernel-substrate extension worker. Workdir: `/home/sdancer/nmss-emu-cert-kuprobe-sp968` (create via `git worktree add` from `/home/sdancer/nmss-emu` based on branch `lkm-injector` so the v1 module sources are right there). Target device: RK3588 at `adb -s localhost:5558`. This is general-purpose RE-tooling work: extend the proven `inject.vendor.ko` v1 (fact `lkm_injector_v1_complete_2026_05_18` cycle 1050) with userspace-probe registration so an analyst can read register/stack state at an arbitrary user-text address.

## Goal of this turn
The LKM v1 ships parse + log + VMA-locate. v2 needs **`register_uprobe()` at a user-virtual offset inside a target binary**, with a handler that:
1. Reads the breakpoint thread's `pt_regs->sp` and dereferences `sp+0x968` for 64 bytes.
2. Captures a fixed window of thread context (regs x0-x30, sp, pc, comm, pid, tid, ts_ns).
3. Drains samples to userspace via a debugfs ring buffer or `tracefs` event (the simplest reliable mechanism on vendor 5.10.160).

Concrete probe target this turn: `libnmsssa.so` runtime base + `0x17ee1c` inside `com.netmarble.thered` (PID lookup at insmod-time via `/proc/<pid>/maps`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay`
- **sub_goal_key**: `cert-kuprobe-sp968`

## Hypothesis
A kernel `register_uprobe()` at `libnmsssa+0x17ee1c` (the cert function entry, per fact `cert_algorithm_recovered`) fires precisely on each cert function call, lets us read `sp+0x968` at function entry (i.e. before the function has moved its own locals), and the resulting 64-byte block is the SHA-256 single-block pre-image documented in `cert-rust-repro/README.md`. Combined with the live op901 Token captured concurrently via tcpdump, this validates or falsifies the algorithm against live data.

## Falsification criterion (any one of)
- `register_uprobe()` returns -EEXIST/-EINVAL/-ENOENT for the libnmsssa.so inode — vendor kernel's uprobe support is absent or restricted.
- Uprobe registers cleanly but the handler never fires across a full login flow (0 events in 120s) — function not exercised at that offset.
- Handler fires N≥3 times in a login flow and every captured 64-byte block fails to produce `cert==Token` via `cert_rust_repro` — algorithm spec or offset wrong, NOT capture mechanism.
- NMSS terminates the process or emits a poisoned op902 (Result≠0) within 30s of insmod — kuprobe IS detected by AC.

## Hard rules
- **adb target**: `localhost:5558` only.
- **One commit per logical step.** Don't accumulate untracked artifacts past one tcpdump/login flow.
- **Branch off `lkm-injector`** so the v1 baseline is preserved. Don't rewrite v1 sources in place — add `inject_uprobe.c` / new procfs entry like `/proc/uprobe_capture`.
- **Memory cap 512 MB** worker RSS. **Disk cap 200 MB** on device.
- **40 min wall cap.**
- The vendor task_struct offset map from cycle 1048 still applies: `task_struct {flags=0x44, mm=0x4a0, comm=0x718, alloc_lock=0x7f0}`. Use the same `inject.vendor.ko`-style patching if any offsets are referenced.

## Step 1 — confirm uprobe substrate on vendor kernel
1. Verify `CONFIG_UPROBES=y` and `CONFIG_UPROBE_EVENTS=y` in the on-device config:
   ```bash
   adb -s localhost:5558 shell "su 0 sh -c 'zcat /proc/config.gz | grep -E \"UPROBE|TRACEFS\"'"
   ```
2. Verify `/sys/kernel/debug/tracing/uprobe_events` is writable (root):
   ```bash
   adb -s localhost:5558 shell "su 0 ls -la /sys/kernel/debug/tracing/uprobe_events"
   ```
3. If either is missing/disabled → fact `kuprobe_substrate_vendor_kernel_unsupported_2026_05_18=true` and STOP.
4. Otherwise proceed.

## Step 2 — write `inject_uprobe.c`
Module exports `/proc/uprobe_capture` (write-only). Write format: `<pid> <so_path> <offset_hex>`. Example: `9942 /data/app/.../libnmsssa.so 17ee1c`.

On write:
1. Open the so_path via kern_path() / vfs_path_lookup() to get the inode.
2. Call `uprobe_register(inode, offset, &consumer)` with a consumer whose `handler` callback:
   - reads `regs->sp` (arm64 pt_regs->sp at index 31).
   - calls `copy_from_user_nofault(buf, (void __user *)(regs->sp + 0x968), 64)`.
   - timestamps with `ktime_get_ns()`.
   - pushes a fixed-size sample `{pid, tid, sp, pc, ts, block[64]}` into a kfifo.
3. A debugfs file `/sys/kernel/debug/uprobe_capture/events` exposes the kfifo via `simple_read_from_buffer()` per-event.

Reference: kernel 5.10 `samples/uprobe_events.c` for inode lookup + uprobe_register pattern. Do NOT use `samples/bpf` — eBPF is unrelated and adds dependencies.

## Step 3 — build + load + smoke test on a synthetic target
1. Cross-compile against the SAME upstream-tagged linux-5.10.160 source at `/home/sdancer/kernel-modules-rk3588/linux-5.10.160/` + on-device `/proc/config.gz`, same as cycle 1047. Reuse the `Makefile` + `build_aux/patch_module_for_vendor.py` from inject.vendor.ko.
2. Push `inject_uprobe.vendor.ko` to device, insmod.
3. Smoke test: probe a known-firing user-text address in `test_host` (the same arm64 cross-compiled binary at `/data/local/tmp/test_host` from re-tool-lib cycle 1054). Confirm at least 1 sample lands in the debugfs file.

## Step 4 — point at libnmsssa, drive login, collect samples
1. Get current `com.netmarble.thered` PID after fresh app launch:
   ```bash
   adb -s localhost:5558 shell pidof com.netmarble.thered
   ```
2. Get libnmsssa.so file path + load base:
   ```bash
   adb -s localhost:5558 shell "su 0 sh -c 'grep libnmsssa /proc/$(pidof com.netmarble.thered)/maps | head -1'"
   ```
3. Register uprobe: `echo "$PID $LIB_PATH 17ee1c" > /proc/uprobe_capture`.
4. Concurrently start tcpdump on LOBBY (183.110.205.25:12000) per cert-algo-validate Step 3 pattern.
5. Drive login UI (force-stop + relaunch + tap login button, then tap character if Character-Selection appears — per cycle 1022 pattern). Wait 120 s.
6. Pull `/sys/kernel/debug/uprobe_capture/events` to host. Count samples.

## Step 5 — correlate + verdict
1. Decode op901 from pcap → live Token + sessionID (reuse `decode_op901.py` from cert-algo-validate).
2. For each captured 64-byte block (expect 1-5 samples in 120 s), synthesize `session.json` and run `cert_rust_repro --session <file>`. Compare output to Token.
3. Match → SUCCESS fact `cert_algo_validated_via_kuprobe_2026_05_18=true` with block + offset + tid + ts. Goal stage moves 5/6→6/6.
4. No match → set `cert_kuprobe_block_captured_but_no_match_2026_05_18` with diagnostic (block hex, what algorithm variant might fit, whether SHA-padding signature is present this time).
5. 0 samples in 120s → set `cert_kuprobe_function_not_exercised_at_17ee1c_2026_05_18` (consistent with cycle 372/381 wire-cipher 0-sample experience on a different lane).

## Outputs
- `inject_uprobe.c`, `Makefile` extensions, `inject_uprobe.vendor.ko` (built).
- `analysis/uprobe_substrate_check.md` (Step 1 verdict).
- `analysis/artifacts/uprobe_events.bin` (drained debugfs samples).
- `analysis/artifacts/cav_kuprobe.pcap` + decoded op901 JSON.
- `analysis/cert_kuprobe_validation.md` (verdict).
- One commit per step on branch `cert-kuprobe-sp968`. Worktree NOT removed on completion — substrate is reusable for other paths.
- Final line: `CERT_KUPROBE_SP968_DONE`.

## Constraints & gotchas
- **NMSS detection of kuprobe is UNTESTED.** Per fact `nmss_xerda_readonly_likely_safe_2026_05_17`: read-only passive observation has been validated 4× via dd /proc/PID/mem. Kuprobe is heavier — it patches a single 4-byte instruction at the probe site (UPROBE_SWBP). If AC verifies code integrity (e.g. via mprotect-readback or CRC), it WILL detect. Watch for: op902 Result≠0, fast process termination, or warning messages in tcpdump traffic shortly after insmod.
- **Process boundary.** Uprobe is per-inode, not per-pid. ALL processes mapping libnmsssa.so will trigger — but only com.netmarble.thered should be running.
- **Stacks grow DOWN on ARM64.** `sp+0x968` reads ABOVE current SP, which is the active frame's variable area. Correct direction (cf. cycle 1030 stack-direction bug).
- **Reboot recovery is fine** (fact `lkm_injector_v1_kernel_panic_on_write_2026_05_18`). If kuprobe causes a panic, the device boots clean.
- **Frida is OFF-LIMITS** for this lane (memory `feedback_no_frida` + fact `nmss_poisons_cert_under_frida_attach_2026_05_17`). This path uses kernel-only mechanisms.
- **No Stalker / no Interceptor.attach.** Kuprobe IS a hook in the kernel, but it's not Frida's code-patching hook — it's the kernel's `uprobe_register()` which is the standard mechanism since 3.5.

## Relevant files / references
- `/home/sdancer/orchestrator/briefings/lkm-injector.md` — v1 substrate this builds on
- Branch `lkm-injector` on `xenomorphtech/nmss_2` — v1 sources + vendor patch
- `/home/sdancer/nmss-emu-cert-algo-validate/analysis/artifacts/decode_op901.py` — op901 decoder (reusable)
- `/home/sdancer/nmss-emu/cert-rust-repro/` — algorithm reference impl + tests
- `/home/sdancer/nmss-wiki/05-cert-algorithm.md` — algorithm doc
- `/home/sdancer/nmss-wiki/06-ground-truth-pairs.md` — 5 ground-truth (sessionID, cert) pairs
- Kernel docs: `Documentation/trace/uprobetracer.rst`, `kernel/events/uprobes.c`
- Fact `lkm_injector_v1_complete_2026_05_18` — v1 substrate proven
- Fact `lkm_injector_vendor_task_struct_offsets_2026_05_18` — offset map
- Fact `cert_algo_block_not_at_sp_0x968_2026_05_18` — what 20Hz polling proved (capture timing problem)
- Fact `cert_algorithm_recovered` — cert function at 0x17ee1c is the orchestrator entry
