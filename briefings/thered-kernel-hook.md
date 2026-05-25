# thered-kernel-hook — Capture thered's GS PktLogin via kernel hooks (kprobe / Frida on libcrypto)

## Role & workdir
Kernel-instrumentation worker. Workdir: `/home/sdancer/nmss-emu-device-webview-signin` (reuse). Targets thered on RK3588 at `adb -s localhost:5558`.

## Why this path (cycle 998 user feedback)
12+ off-device PktLogin replays returned Result=22. Random heap-grep for JWT strings found nothing in 3 regions. **Stop the grep-and-pray; use a directed hook on the producer.**

Per memory `feedback_kernel_instrumentation`: uprobes/kprobes/eBPF available, invisible to anticheat. Per `feedback_no_frida`: no Frida on libUnreal.so, but **system libcrypto/libssl is fine**. Per `feedback_no_stall_kernel_aggressive`: when userspace stuck, go kernel.

## Three attack vectors — execute in this priority order

### Vector A — UPDATED 2026-05-17: LKM with register_kprobe(), NOT tracefs

**Update**: device kernel lacks `CONFIG_KPROBE_EVENTS` so `/sys/kernel/debug/tracing/dynamic_events` rejects kprobe entries. **But we can write an out-of-tree kernel module** that calls `register_kprobe()` directly — this works on any kernel with `CONFIG_KPROBES=y` (RK3588 kernel 5.10.160 has it; `/proc/kallsyms` exposes `tcp_sendmsg`). Per memory `feedback_kernel_instrumentation` + `feedback_lkm_library_injection` — LKM substrate is proven on this device.

LKM sketch (`tcp_send_hook.c`):
```c
#include <linux/module.h>
#include <linux/kprobes.h>
#include <linux/inet.h>
#include <net/sock.h>
#include <net/inet_sock.h>

static struct kprobe kp = { .symbol_name = "tcp_sendmsg" };

static int pre(struct kprobe *p, struct pt_regs *r) {
    struct sock *sk = (struct sock *)r->regs[0];
    struct msghdr *msg = (struct msghdr *)r->regs[1];
    size_t len = (size_t)r->regs[2];
    struct inet_sock *isk = inet_sk(sk);
    __be32 daddr = isk->inet_daddr;
    __be16 dport = isk->inet_dport;
    // filter 183.110.40.0/24 dport 12000 (be 0x2EE0)
    if ((daddr & htonl(0xFFFFFF00)) == htonl(0xB76E2800) && dport == htons(12000)) {
        u8 buf[256] = {0};
        size_t copy = min_t(size_t, len, sizeof(buf));
        // iov_iter copy — kernel-mode safe
        if (!copy_from_iter(buf, copy, &msg->msg_iter))
            return 0;
        printk(KERN_INFO "thk: len=%zu hex=", len);
        for (int i = 0; i < copy; i++) printk(KERN_CONT "%02x", buf[i]);
        printk(KERN_CONT "\n");
    }
    return 0;
}

static int __init m_init(void) { kp.pre_handler = pre; return register_kprobe(&kp); }
static void __exit m_exit(void) { unregister_kprobe(&kp); }
module_init(m_init); module_exit(m_exit);
MODULE_LICENSE("GPL");
```

Build steps (worker):
1. Find/build the matching arm64 kernel headers. Check `/lib/modules/$(uname -r)/build/` on device OR look in `/home/sdancer/kernel-modules-rk3588/` on orchestrator host.
2. Cross-compile if device-side build doesn't work — use the existing aarch64 kernel-module build infrastructure if present (`find / -name "*.ko" -path "*5.10*"` to find prior modules).
3. `adb push tcp_send_hook.ko /data/local/tmp/`, `adb shell "su 0 insmod /data/local/tmp/tcp_send_hook.ko"`.
4. `adb shell "su 0 dmesg -w | grep 'thk:'"` to stream hex captures.
5. Drive UI past title screen; capture all bytes thered sends to 183.110.40.x:12000.

The original tracefs Vector A entry below is kept for reference but is **falsified** on this device.

### Vector A (FALSIFIED): kprobe on `tcp_sendmsg` filtered to GS — via tracefs dynamic_events
The lobby + GS protocol is plaintext TCP with vampir XOR-encoded body (XOR key already known: `9A A7 84 20 D0 C9 78 B3`). A kprobe captures bytes *post-app-XOR but pre-network*, which is what tcpdump showed too — but kprobe is more reliable for short bursts that connection-cycle past tcpdump's start.

```bash
adb -s localhost:5558 shell "su 0 sh -c '
  cd /sys/kernel/debug/tracing
  # disable other tracing first
  echo 0 > tracing_on
  echo > trace
  # kprobe: tcp_sendmsg(sk, msg, size); dump 256 bytes of msg->msg_iter base
  echo \"p:tcp_send_p tcp_sendmsg sk=%di msg=%si size=%dx\" > kprobe_events
  echo 1 > events/kprobes/tcp_send_p/enable
  echo 1 > tracing_on
  sleep 60
  cat trace > /sdcard/kprobe_trace.txt
  echo 0 > tracing_on
'"
```
Limitation: kprobe `%di/%si` aren't simple bytes — we get pointers. Better: use `bpftrace` if installed OR write a small uprobe-via-perf. If bpftrace absent on device, install it (`adb push` from a cross-compiled binary in `~/aeon` if present) OR fall back to Vector B.

### Vector B: Frida on libcrypto SSL_write (BoringSSL) — captures all HTTPS plaintext incl. cpp-auth + any TLS GS traffic
libcrypto.so at `/system/lib64/libcrypto.so` is loaded in thered (mapping at 0x6d94e0b000). Frida is allowed on libcrypto (system, not libUnreal).

```bash
adb -s localhost:5558 shell "su 0 /data/local/tmp/frida-server &"
sleep 3
# from orchestrator host (frida client):
pip install frida-tools --user || true
frida -U -n com.netmarble.thered -l hook_ssl_write.js
```

`hook_ssl_write.js` content:
```js
const SSL_write = Module.getExportByName("libcrypto.so", "SSL_write")
                  || Module.getExportByName("libssl.so", "SSL_write");
Interceptor.attach(SSL_write, {
  onEnter(args) {
    const buf = args[1];
    const len = args[2].toInt32();
    if (len > 0 && len < 4096) {
      const bytes = buf.readByteArray(Math.min(len, 1024));
      console.log("SSL_write len=" + len + " hex=" + hexdump(bytes, {length: Math.min(len,256)}));
    }
  }
});
```

This captures cpp-auth POST bodies + any HTTPS the SDK sends, with full plaintext.

### Vector C (escalation): uprobe on libUnreal lobby_send function
ONLY if A and B both yield nothing. Requires identifying the function address that builds PktLogin payload. Find via vampir captures + libUnreal disasm OR by tracing function entries.

## Trigger the flow

After hooks installed, drive UI past title:
```bash
adb -s localhost:5558 shell "input tap 960 540"  # dismiss "Please tap the screen"
sleep 3
adb -s localhost:5558 shell screencap -p > analysis/artifacts/screen_after_tap.png
# Continue tapping through character/login as screenshots dictate. Add tap delays.
```

While tapping, hooks should fire on:
- HTTPS to apis.netmarble.com (cpp-auth — SToken refresh)
- TCP to 183.110.205.25:12000 (lobby)
- TCP to 183.110.40.52:12000 (Hector 2 — GS for this account)
- Opcode 901 traffic if user gets in-game

Capture for at least 60s past last UI tap.

## Output
- `analysis/artifacts/kprobe_trace.txt` (Vector A)
- `analysis/artifacts/ssl_write_capture.txt` (Vector B)
- `analysis/artifacts/screen_*.png` (UI progression)
- `analysis/artifacts/kernel_hook_decoded.json` — final extracted PktLogin field values

## Constraints
- 30 min wall
- 1 GB RSS worker cap
- adb -s localhost:5558 (NOT Waydroid — that's AO substrate)
- Do not destroy thered app data
- If frida-server is already running per `pidof`, don't start another
- Right now device is at title screen "Please tap the screen" with Hector 2 selected (ServerId=206), ClientAssetVersion=212665 (per fact thered_1812_constants_real_2026_05_17)

## Success criterion
A decoded PktLogin (opcode 3 destined to 183.110.40.52:12000) with all 25 field values populated — that's the byte-for-byte truth we've been guessing at. Set fact `thered_real_pktlogin_captured_2026_05_17`.
