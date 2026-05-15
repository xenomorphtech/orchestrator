# dark-december-kernel-hide-frida — LKM that filters /proc/self/maps to hide Frida from Hercules

## Role & workdir
Kernel-module author. Workdir: `/home/sdancer/dark-december-kernel-hide-frida` (worktree of `/home/sdancer/dark-december`, branch `kernel-hide-frida`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump`.
- Sub-goal: write + load a Linux kernel module that intercepts `/proc/<pid>/maps` reads from the Dark December process and **filters out** lines mentioning `frida-agent`, `memfd:`, and other Frida artifacts. This lets Frida-on-Java guest-bypass (per task1 in [[dark-december-hive-guest-bypass]] sibling) succeed because Hercules' anti-Frida scan returns clean.

## Standing rule (memory [[feedback-no-stall-kernel-aggressive]])
Stale/parked is never acceptable. Userspace AC walls are defeated with kernel modules. Do not escalate this to user as a resource ask — write the kernel module.

## Success criteria
Concrete deliverable:
- Compiled LKM `frida_hide.ko` for Waydroid kernel (Linux 5.10.160 aarch64).
- Loadable via `insmod frida_hide.ko` on the device.
- After insmod: spawn-owned xerda no-op script survives DD startup (process does NOT die before Java bridge initializes).
- Demonstrated via re-running the prior `hive_guest_signin.js` script against a Frida-hidden Dark December → guest sign-in callback fires → `:10001` traffic captured.
- Sets fact `dark_december_frida_hidden_via_lkm`.

## Cross-pollination facts
- `dark_december_anti_frida_detected_2026_05_14`: Hercules kills DD process if Frida is detected at startup. Mechanism inferred: memfd/library-list scan in /proc/self/maps.
- `magic32-uprobes-aes-pc` task 1 closure (`/home/sdancer/nmss-emu-magic32-uprobes-aes-pc/analysis/task1_kernel_capability_2026-05-14.md`): kernel is Linux 5.10.160 aarch64, CONFIG_UPROBES/BPF/MODULES all =y, /sys/kernel/debug/tracing writable as root, tracefs mounted.
- `dark_december_guest_auth_api_live_2026_05_14`: ProviderType.GUEST is compiled in.
- `dark_december_ingame_socket_target_2026_05_14`: live :10001 to 130.162.236.3 ESTABLISHED (when logged in).

## Next concrete tasks (long single turn, 3h budget)

1. **Kernel build environment.**
   - Verify kernel headers / build dir available on the Waydroid host:
     - `adb shell ls /lib/modules/$(adb shell uname -r)/build 2>/dev/null || echo missing`
     - If no on-device build env: cross-compile from host with the Waydroid kernel source.
   - Find / fetch Waydroid kernel source for `Linux 5.10.160 #57 SMP PREEMPT Wed Dec 20 14:01:38 CST 2023 aarch64`:
     - Look at `/proc/version`, `/lib/modules/$(uname -r)/source` on device.
     - If Waydroid uses the RK3588 vendor kernel (per `product:rk3588_s`), pull from Rockchip's open-source kernel tree or Waydroid's `kernel-rk3588`.
     - On host: `aarch64-linux-gnu-gcc` (already verified available per sibling work).

2. **Write the LKM (`frida_hide.c`).**

   Two implementation approaches — pick whichever is simpler:

   **Approach A — kprobe on proc_pid_maps_op .show:**
   - Hook `show_map_vma()` (or `m_show()` in `fs/proc/task_mmu.c`) via kprobes.
   - Inspect the vma's backing file/anon name. If it matches blocklist patterns (`/memfd:frida-agent-64.so`, `/data/local/tmp/re.frida.server`, `frida-agent`, any path containing `frida`), call `kprobe.return_handler` that skips the line.
   - Tricky because the show callback writes to a seq_file — need to track state.

   **Approach B — ftrace-style hook on the syscall:**
   - Hook `vfs_read` / `__arm64_sys_read` with check on the caller's file → if reading from `/proc/<pid>/maps` AND pid matches darkdecember → filter the returned buffer in-place before copy_to_user completes.
   - Cleaner but requires `ftrace_replace_code` or `kallsyms_lookup_name` access.
   
   **Approach C — userspace-mountable proc overlay (FUSE-style):**
   - NOT a kernel module — a userspace daemon that mounts a tmpfs overlay on `/proc/<pid>/maps` with filtered content. Loaded via setns into the app's mount namespace.
   - Simpler than LKM but might be detected if AC checks the inode origin.

   Recommended: **start with Approach B** (vfs_read hook). Sample skeleton:

   ```c
   #include <linux/module.h>
   #include <linux/kprobes.h>
   #include <linux/kallsyms.h>
   #include <linux/file.h>
   #include <linux/fs.h>
   #include <linux/string.h>

   static const char BLOCKLIST[][32] = {
       "frida-agent", "memfd:frida", "re.frida", "frida-server", ""
   };

   /* Hook vfs_read return; if file is /proc/.../maps and current's comm starts with
      "darkdecember" or matches DD's task tree, scan the returned buffer and zero out
      any line that contains a blocklist substring. */

   static struct kprobe kp = {
       .symbol_name = "vfs_read",
   };

   static int kp_pre(struct kprobe *p, struct pt_regs *regs) { return 0; }
   static void kp_post(struct kprobe *p, struct pt_regs *regs, unsigned long flags) {
       /* In aarch64 syscall ABI, x0 = struct file *, x1 = user buf, x2 = count, x3 = pos */
       struct file *f = (struct file *)regs->regs[0];
       /* ... */
   }
   ```

   Iterate until it compiles + loads + filters correctly. Test on a simple `cat /proc/self/maps` first to confirm filtering works for the orchestrator's own process, then for DD.

3. **Compile + transfer + load.**
   ```bash
   # Cross-compile
   make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- -C /path/to/waydroid-kernel-source M=$(pwd) modules
   # Transfer
   adb push frida_hide.ko /data/local/tmp/
   # Load
   adb shell insmod /data/local/tmp/frida_hide.ko
   # Verify loaded
   adb shell cat /proc/modules | grep frida_hide
   # Verify hooking
   adb shell cat /proc/self/maps | grep -i frida   # should now hide if module is filtering
   ```

4. **Test re-running the hive-guest-signin Frida script with the LKM loaded.**
   - First confirm baseline (without insmod): DD process dies on xerda attach.
   - Then `insmod frida_hide.ko` and re-run the same `hive_guest_signin.js` script from `/home/sdancer/dark-december-hive-guest-bypass/scripts/`.
   - If DD process survives → LKM is working; Java bridge initializes; guest sign-in callback fires.
   - Once logged in via guest: capture `:10001` (delegate to dark-december-live-capture worker if it's still running, or do it inline).

5. **Write artifact `<workdir>/analysis/kernel_frida_hide_2026-05-14.md`:**
   - The C source of the LKM.
   - Build command + cross-compile artifacts.
   - insmod + dmesg evidence of successful load.
   - Test: `cat /proc/self/maps | grep frida` → empty after insmod.
   - Frida attach success + guest sign-in callback log.
   - Set fact `dark_december_frida_hidden_via_lkm` on success.

## Constraints & gotchas
- The Waydroid kernel is Linux 5.10.160; kallsyms may be enabled but `kallsyms_lookup_name` was removed from public export in 5.7+ — use kprobe-based symbol resolution as the workaround.
- Kernel modules need a `MODULE_LICENSE("GPL")` declaration or some kernel symbols are unexported.
- SELinux: `setenforce 0` was set / confirmed permissive in prior sibling work; the module load may still need `CAP_SYS_MODULE` (root has it).
- AC may also check kernel module list (`/proc/modules`) — name the module innocuously (e.g., `tcp_accel.ko`) to reduce visibility.
- Module signing: Waydroid kernel may have `CONFIG_MODULE_SIG_FORCE=y` requiring signed modules. If insmod fails with `key was rejected by service`, look at how Waydroid distributes its own out-of-tree drivers.

## Falsification
- Kernel headers / source unavailable for the exact Waydroid build → escalate (need kernel source artifact or build env). Even then, ftrace+kprobe via /sys/kernel/debug/tracing might work without recompiling — fall back to that.
- Module signing enforced AND no way to sign without vendor key → escalate (resource ask: need Waydroid kernel build that doesn't enforce signing OR vendor signing key).
- LKM loads + filters /proc/maps correctly but Hercules also scans `/proc/self/task/.../maps` per-thread or uses a different detection vector → expand the hook to cover those paths.
- After 3 cycles of iteration: if no detection-mechanism works, file a real falsification documenting which AC check we couldn't defeat.

## Relevant files / references
- prior hive-guest-bypass scripts: `/home/sdancer/dark-december-hive-guest-bypass/scripts/{hive_guest_signin.js,run_hive_guest_signin.py,noop.js}` — REUSE these post-insmod.
- prior hive-guest-bypass closing artifact: `/home/sdancer/dark-december-hive-guest-bypass/analysis/hive_guest_bypass_2026-05-14.md` — describes the detection mechanism in detail.
- Linux kernel kprobe docs: `Documentation/trace/kprobes.rst` in kernel source.
- Linux module-build doc: `Documentation/kbuild/modules.rst`.
- Tools: `aarch64-linux-gnu-gcc`, `make`, `adb`, `insmod`, `dmesg`.
