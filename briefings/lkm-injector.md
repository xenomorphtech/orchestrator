# lkm-injector — kernel-module library injector for arm64 Android

## Role & workdir
Kernel tool worker. Workdir: `/home/sdancer/nmss-emu-lkm-injector` (create via `git worktree add` from `/home/sdancer/nmss-emu`). Target device: RK3588 at `adb -s localhost:5558`.

## Goal of this turn
Build a minimal arm64 kernel module that loads a user-supplied `.so` into a running user process. **General-purpose RE/security tool.** The module is the substrate; what gets injected is policy.

Deliverable: `inject.ko` that, once `insmod`'d on the device, exposes `/proc/inject`. Writing `<pid> <so_path>` to it causes the target process to `dlopen(<so_path>)` on its next return to user space.

## Hard rules
- **adb target**: `localhost:5558` only.
- **Read-only on system code** — only target *processes* you control or have explicit permission to instrument.
- **No CFI/SELinux bypass tricks** — if the kernel build rejects the module, fail loudly. Don't hammer.
- **40 min wall cap.** ≤ 500 MB worker RSS, ≤ 200 MB device disk.

## Step 1 — get kernel build headers for the target
1. Read kernel version on device:
   ```bash
   adb -s localhost:5558 shell "uname -a"
   adb -s localhost:5558 shell "ls /lib/modules/$(uname -r)/build/ 2>/dev/null"
   ```
2. If `/lib/modules/.../build/` exists on device, copy it to host:
   ```bash
   adb -s localhost:5558 pull /lib/modules/$(adb -s localhost:5558 shell uname -r | tr -d '\r')/build/ analysis/kernel_headers/
   ```
3. If headers are absent, check host:
   - `find /home/sdancer -maxdepth 3 -name "Makefile" -path "*5.10*kernel*"`
   - `ls /home/sdancer/kernel-modules-rk3588/ 2>/dev/null`
   - `ls /home/sdancer -name "rk3588*" 2>/dev/null`
4. Document what you found in `analysis/kernel_build_env.md`.
5. If neither is found: emit fact `lkm_injector_kernel_headers_absent_2026_05_17` and STOP — needs user resource. Otherwise continue.

## Step 2 — write the kernel module source
Single file `inject_lkm/inject.c` with the following sections:

```c
// SPDX-License-Identifier: GPL-2.0
#include <linux/module.h>
#include <linux/init.h>
#include <linux/proc_fs.h>
#include <linux/uaccess.h>
#include <linux/sched.h>
#include <linux/task_work.h>
#include <linux/pid.h>
#include <linux/string.h>

// max pending requests
#define MAX_PATH 256

struct inject_req {
    struct callback_head work;
    char so_path[MAX_PATH];
};

// task_work callback runs in target task on return to user space.
// We CAN'T call dlopen from kernel mode. Instead: use the
// kernel-mode side to install a uprobe at libc's __libc_init that
// then resolves dlopen and calls it with our path.
//
// SIMPLER mechanism for v1: just write the .so path to a file
// /data/local/tmp/inject_req_<pid>.txt and rely on a watchdog
// thread inside the target (pre-installed via app-side stub) to
// honor it. For v1, also accept that this isn't fully kernel-driven
// — it's a kernel-controlled file-drop trigger.
//
// For v1 we limit to: enumerate VMAs of the target, log them to
// dmesg, and write the so_path to a known file in target's cwd.
// Document the limitation in README; v2 will add real dlopen
// scheduling via uprobe.

static struct proc_dir_entry *proc_entry;

static ssize_t inject_write(struct file *f, const char __user *ubuf,
                             size_t len, loff_t *ppos) {
    char buf[MAX_PATH + 32];
    if (len > sizeof(buf) - 1) return -EINVAL;
    if (copy_from_user(buf, ubuf, len)) return -EFAULT;
    buf[len] = '\0';

    // Parse "<pid> <so_path>"
    char *space = strchr(buf, ' ');
    if (!space) return -EINVAL;
    *space = '\0';
    pid_t pid;
    if (kstrtoint(buf, 10, &pid)) return -EINVAL;
    char *so = space + 1;
    // strip trailing newline
    char *nl = strchr(so, '\n'); if (nl) *nl = '\0';

    // Find task by pid
    struct pid *kpid = find_get_pid(pid);
    if (!kpid) return -ESRCH;
    struct task_struct *tsk = pid_task(kpid, PIDTYPE_PID);
    if (!tsk) { put_pid(kpid); return -ESRCH; }

    pr_info("inject: pid=%d so=%s comm=%s\n", pid, so, tsk->comm);
    // v1: log VMA range count
    pr_info("inject: vma count for pid %d\n", pid);

    put_pid(kpid);
    return len;
}

static const struct proc_ops inject_ops = {
    .proc_write = inject_write,
};

static int __init inject_init(void) {
    proc_entry = proc_create("inject", 0220, NULL, &inject_ops);
    if (!proc_entry) return -ENOMEM;
    pr_info("inject: loaded; write \"<pid> <so_path>\" to /proc/inject\n");
    return 0;
}

static void __exit inject_exit(void) {
    proc_remove(proc_entry);
    pr_info("inject: unloaded\n");
}

module_init(inject_init);
module_exit(inject_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("RE-tooling library injector (v1)");
```

And a `Makefile`:
```makefile
obj-m += inject.o
KDIR ?= /home/sdancer/nmss-emu-lkm-injector/analysis/kernel_headers
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
```

## Step 3 — build, push, smoke-test
```bash
cd inject_lkm
make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu-
file inject.ko
adb -s localhost:5558 push inject.ko /data/local/tmp/inject.ko
adb -s localhost:5558 shell "su 0 sh -c 'insmod /data/local/tmp/inject.ko && dmesg | tail -5'"
adb -s localhost:5558 shell "su 0 sh -c 'echo 1 /data/local/tmp/foo.so > /proc/inject; dmesg | tail -5'"
adb -s localhost:5558 shell "su 0 sh -c 'rmmod inject && dmesg | tail -5'"
```

Expected dmesg lines: `inject: loaded`, `inject: pid=1 so=/data/local/tmp/foo.so comm=init`, `inject: unloaded`.

## Step 4 — document v1 limitations and v2 roadmap
Write `analysis/inject_README.md` covering:
- What v1 does (parse + log + file-drop). Honest scope.
- What v2 needs to do for real `dlopen` scheduling — task_work + uprobe approach OR signal-handler-based approach.
- How to load a payload .so (next briefing — `re-tool-lib.md`).

## Outputs
- `inject_lkm/inject.c`, `inject_lkm/Makefile`, `inject_lkm/inject.ko` (built).
- `analysis/kernel_build_env.md`
- `analysis/inject_README.md`
- Facts:
  - SUCCESS: `lkm_injector_v1_loaded_2026_05_17` = true (with dmesg evidence)
  - FAIL on kernel-headers: `lkm_injector_kernel_headers_absent_2026_05_17` = true
- Final line: `LKM_INJECTOR_DONE`

## References
- Linux kernel doc: `Documentation/kernel-hacking/hacking.rst`, `Documentation/filesystems/proc.rst`
- task_work API: `include/linux/task_work.h` (kernel ≥5.0)
- Memory: `feedback_lkm_library_injection` — LKM substrate proven on this device
