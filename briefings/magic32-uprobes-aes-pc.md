# magic32-uprobes-aes-pc — eBPF uprobe at AES PC to capture the runtime key

## Role & workdir
Kernel-side instrumentation analyst. Workdir: `/home/sdancer/nmss-emu-magic32-uprobes-aes-pc` (worktree of `/home/sdancer/nmss-emu`, branch `magic32-uprobes-aes-pc`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: capture the **AES-128 key** at runtime when libUnreal.so calls the MAGIC32 producer, using a kernel-installed software breakpoint (uprobe) — invisible to Hercules' userspace .text-hash integrity check.

## Success criteria
Closing fact: `nmss_magic32_numerically_reproduced`. Concrete deliverable:
- Captured (key_bytes, plaintext_bytes) at the AES PC `0x195b9f8` in libUnreal.so during a `pm clear`-fresh PGS sign-in.
- Verification: `AES-128-ECB(captured_key, captured_plaintext)` == `2FCF997702C244969BFEAF7F0D6AAA1C`.
- The captured pair tells us the **derivation** — if `captured_plaintext == utf8("a_1408633172786630918")` (or a known transform of it) we know the plaintext encoding; the key tells us the secret/derivation.
- Pure-Rust `compute_magic32(pgs_player_id: &str) -> [u8; 16]` in `cert-rust-repro` + passing test.

## Why this is distinct from the 5+1 falsified paths

The 5 prior userspace paths all triggered or got detected by Hercules' anti-instrumentation:
- `magic32-hw-bp`: ptrace HW-breakpoints — Hercules detects attached ptracer.
- `magic32-java-blackbox`: Frida Java.use() — Hercules inspects ART method tables.
- `magic32-disasm`, `magic32-apk-strings-sweep`, `magic32-snapshot-mining`: pure-static / offline — couldn't bridge the runtime-derived key.
- `magic32-gms-state-read`: Got the playerId (`a_1408633172786630918`) but exhaustive 22,272-entry derivation sweep produced 0 matches → the key derivation is more complex than simple-hash-truncate.

**uprobes** sit BELOW Hercules:
- The kernel stores the original instruction byte in a uprobe descriptor, replaces the in-memory text page only at fetch-time, and returns the ORIGINAL byte to anyone calling `process_vm_readv` or hash-checking the .text from inside the process. So Hercules' .text-page-hash check passes.
- The BRK trap is handled in kernel space; the userspace anticheat doesn't see a SIGTRAP delivered to a userland tracer (because there is no userland tracer — the perf-event ring buffer reader is in a different process).
- No `ptrace` ATTACH, no Frida agent loaded.

## Progress so far (closing artifacts from siblings — read first)

- `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md` — confirmed AES PCs `{0x195b9f8, 0x195be04}`, key at PGSClientSecret+0x78 in serializer 0x57f6900.
- `/home/sdancer/nmss-emu-magic32-gms-state-read/analysis/task1_gms_state_inventory_2026-05-14.md` — playerId `a_1408633172786630918` for `ktion23@gmail.com`, also alt `a_8735521947955340805`.
- `/home/sdancer/nmss-emu-magic32-gms-state-read/analysis/task23_sweep_results_2026-05-14.json` — 22272 simple AES derivations swept, 0 matches → derivation is NOT trivial.
- `/home/sdancer/nmss-emu-magic32-hw-bp/aes_hwbp_capture.c` — helper showing the AES-instruction PC layout if useful for cross-checking.

## Next 2–3 concrete tasks

1. **Verify kernel capability on Waydroid.** `adb root` confirmed. Test:
   - `adb shell ls /sys/kernel/debug/tracing/uprobe_events` — confirm uprobes available.
   - `adb shell zcat /proc/config.gz | rg -i 'CONFIG_UPROBES|CONFIG_BPF|CONFIG_PERF_EVENTS'` — confirm kernel built with uprobes + eBPF.
   - `adb shell which bpftrace perf` — bpftrace is the easiest probe-spec language, but raw `perf record -e uprobe_<addr>` also works.
   - If neither bpftrace nor perf-events is built into the Waydroid kernel: escalate (resource ask) — note this needs a custom kernel.

2. **Set uprobe at the AES PC.**
   - Get libUnreal.so load base: `adb shell cat /proc/$(pidof com.netmarble.thered)/maps | rg libUnreal.so | head -1` → take first column (the load base, e.g. `0x78c6680000`).
   - Compute file-offset for `0x195b9f8`: that's relative to libUnreal.so's text section. From magic32-disasm artifact, confirm the disk-file offset matches.
   - Install probe via uprobe_events:
     ```bash
     adb shell 'echo "p:aes_key_capture /data/local/tmp/libUnreal.so:0x<file_offset>" > /sys/kernel/debug/tracing/uprobe_events'
     adb shell 'echo 1 > /sys/kernel/debug/tracing/events/uprobes/aes_key_capture/enable'
     adb shell 'cat /sys/kernel/debug/tracing/trace_pipe' &
     ```
   - Alternative (preferred if bpftrace is available): write a `.bt` script that captures `arg0..arg3` (x0..x3 on aarch64) at the uprobe event, with the process filter `pid == pidof com.netmarble.thered`.

3. **Force a fresh PGS sign-in to trigger the producer.**
   - `adb shell pm clear com.netmarble.thered` (clears cached `I_PID`).
   - `adb shell am start -n com.netmarble.thered/<launcher activity>` — get from `aapt dump badging` or `cmd package resolve-activity`.
   - The first-launch path runs the PGS handshake → AES producer → MAGIC32 write. The uprobe should fire when execution reaches `0x195b9f8`.
   - Capture: x0..x3 register values at the trap. On aarch64 calling convention: x0 = first arg, x1 = second arg, x2 = third arg, x3 = fourth arg. For an AES wrapper like `AES_ECB_encrypt(key, plaintext, output, len)`, x0 = key pointer, x1 = plaintext pointer.
   - Also need to read the bytes at those pointers. Use bpftrace's `kptr_t` / `usermem` or do a follow-on `process_vm_readv` from the kernel module (advanced).

4. **Verify the capture against captured MAGIC32 = `2FCF997702C244969BFEAF7F0D6AAA1C`**.
   - Take captured key and plaintext bytes, run AES-128-ECB in pure Rust (`aes` crate) or Python (`cryptography`).
   - If `AES-128-ECB(captured_key, captured_plaintext)` == MAGIC32: derivation captured. Now interpret:
     - Is `captured_plaintext` the raw `a_1408633172786630918` UTF-8 bytes? Or `1408633172786630918` (int → bytes)? Or `g<digits>`?
     - Is `captured_key` derived from a static string (e.g. PGSClientSecret bytes) by some transformation? Look for the captured key bytes near libUnreal.so rodata.
   - Code up `compute_magic32(pgs_player_id: &str) -> [u8; 16]` in `cert-rust-repro/src/magic32.rs`, add test asserting result equals MAGIC32 for `"a_1408633172786630918"`.

5. **Write artifact** `<workdir>/analysis/uprobes_aes_capture_2026-05-14.md`:
   - Kernel capability inventory (uprobes/bpftrace/perf availability).
   - The uprobe spec used.
   - Captured (key, plaintext, ciphertext) triple from the trap event.
   - Rust derivation + passing test.
   - Fact set: `nmss_magic32_numerically_reproduced` on success.

## Constraints & gotchas

- **No Frida on libUnreal.so** — uprobes are NOT Frida. They're a kernel facility.
- **AppGuard / Hercules detection vectors to be aware of**:
  - Some advanced AC checks `/sys/kernel/debug/tracing/uprobe_events` for entries matching its own .so name. If detected, the trap-handler will fire but the app may suicide. Mitigation: install probe AFTER pre-load of the .so but BEFORE the producer runs — small window, requires timing.
  - PMU-based fault-rate detection is RARE at this anticheat tier; if Hercules has it, this path falsifies.
- **adb root + SELinux permissive** confirmed.
- **Cached login skip** is the same blocker as magic32-hw-bp. `pm clear` is the workaround but it triggers Google account picker → on Waydroid the picker auto-completes if there's only one account (`ktion23@gmail.com` is the sole account per gms-state-read sibling).
- This worker runs under systemd `harness-worker@magic32-uprobes-aes-pc.service` (system.slice, MemoryMax=24G).

## Relevant files / references

- libUnreal.so on device: `/data/app/~~<hash>~~/com.netmarble.thered-<id>/lib/arm64/libUnreal.so` (find via `find /data/app -name libUnreal.so 2>/dev/null`).
- Captured MAGIC32: `2FCF997702C244969BFEAF7F0D6AAA1C`.
- Known PGS playerId: `a_1408633172786630918` (UTF-8 bytes: `0x61 0x5f 0x31 0x34 0x30 0x38 0x36 0x33 0x33 0x31 0x37 0x32 0x37 0x38 0x36 0x36 0x33 0x30 0x39 0x31 0x38`).
- Android ID: `54c7b43cb642e8d3` (potential salt input).
- cert-rust-repro at `/home/sdancer/nmss-emu/cert-rust-repro/`.
- Tools: `adb`, `bpftrace`, `perf`, `python3 (cryptography)`, `cargo (aes crate)`.

## Falsification

- Waydroid kernel lacks `CONFIG_UPROBES` or `CONFIG_BPF` → escalate (need a different kernel; resource ask).
- uprobe installs but firing event never observed across 3 `pm clear` + launch cycles → producer is not reached (data-flow blocker, same as magic32-hw-bp; would need to find a different trigger event).
- Hercules detects uprobe and the app self-suicides on launch → escalate to `magic32-kernel-mod-singlestep` (path H).
- Capture succeeds but `AES(key, plaintext) != MAGIC32` even with cleanly-extracted register values → the AES PC `0x195b9f8` is NOT the MAGIC32 producer (revisit disasm assumption).
