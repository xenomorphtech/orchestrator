# re-tool-lib — minimal RE-tooling payload .so

## Role & workdir
Native library worker. Workdir: `/home/sdancer/nmss-emu-re-tool-lib` (create via `git worktree add` from `/home/sdancer/nmss-emu`). Target: arm64 Android.

## Goal of this turn
Build a tiny shared library `libretool.so` that, when loaded into a target process (via `LD_PRELOAD`, `System.load`, or the LKM injector from briefing `lkm-injector.md`), reads a config file and performs simple inspection actions on its host process. **General-purpose RE/forensics tool.**

Deliverable: `libretool.so` (arm64) + a small driver app or test shell snippet that proves it works on a known-safe target (the worker's own throwaway test binary, NOT any third-party application).

## Hard rules
- **adb target**: `localhost:5558` only.
- **Only test on workers' own test binaries** in this turn. Do NOT target third-party apps as the proof-of-life.
- **No hooks** in v1 — read-only inspection only. v2 may add function patching with explicit briefing.
- **30 min wall cap.**

## Config file format
The library reads `/data/local/tmp/retool.cfg` on its constructor. One command per line:

```
# comments allowed
LOG /data/local/tmp/retool.log
READ 0x76acaef000 64 hex
READ_BY_LIB libc.so __libc_init 16 hex
DUMP_MAPS
EXIT
```

Commands (v1):
- `LOG <path>` — set output file path (default `/data/local/tmp/retool.log`)
- `READ <hex_addr> <len> hex|ascii` — read `len` bytes from VA `<hex_addr>` in this process; emit to log
- `READ_BY_LIB <lib_name> <symbol> <len> hex|ascii` — resolve `<symbol>` in `<lib_name>` via `dlsym`, read from there
- `DUMP_MAPS` — write `/proc/self/maps` to the log
- `EXIT` — stop the library thread (do NOT call `exit()` — leave the process running)

If a command would touch unmapped/unreadable memory, catch SIGSEGV via `sigaction` and log `READ_FAULT addr=<x>` instead of crashing the host.

## Step 1 — write the library source
Single C file `retool/libretool.c`:

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <signal.h>
#include <setjmp.h>
#include <pthread.h>
#include <dlfcn.h>

static FILE *log_fp = NULL;
static sigjmp_buf safe_read_jmp;
static volatile int in_safe_read = 0;

static void segv_handler(int sig) {
    if (in_safe_read) siglongjmp(safe_read_jmp, 1);
    // else: re-raise default
    signal(SIGSEGV, SIG_DFL);
    raise(SIGSEGV);
}

static void hexdump_or_ascii(const unsigned char *buf, size_t n,
                              const char *fmt) {
    if (strcmp(fmt, "ascii") == 0) {
        for (size_t i = 0; i < n; i++)
            fputc((buf[i] >= 0x20 && buf[i] < 0x7f) ? buf[i] : '.', log_fp);
        fputc('\n', log_fp);
    } else {
        for (size_t i = 0; i < n; i++) fprintf(log_fp, "%02x", buf[i]);
        fputc('\n', log_fp);
    }
}

static void cmd_read(unsigned long addr, size_t n, const char *fmt) {
    unsigned char buf[4096];
    if (n > sizeof(buf)) n = sizeof(buf);
    in_safe_read = 1;
    if (sigsetjmp(safe_read_jmp, 1) == 0) {
        memcpy(buf, (void *)addr, n);
        in_safe_read = 0;
        fprintf(log_fp, "READ 0x%lx %zu\n", addr, n);
        hexdump_or_ascii(buf, n, fmt);
    } else {
        in_safe_read = 0;
        fprintf(log_fp, "READ_FAULT 0x%lx\n", addr);
    }
    fflush(log_fp);
}

static void cmd_dump_maps(void) {
    int fd = open("/proc/self/maps", O_RDONLY);
    if (fd < 0) { fprintf(log_fp, "MAPS_OPEN_FAIL\n"); return; }
    char buf[4096]; ssize_t n;
    fprintf(log_fp, "=== MAPS START ===\n");
    while ((n = read(fd, buf, sizeof(buf))) > 0) fwrite(buf, 1, n, log_fp);
    fprintf(log_fp, "=== MAPS END ===\n");
    close(fd); fflush(log_fp);
}

static void *worker(void *arg) {
    (void)arg;
    signal(SIGSEGV, segv_handler);

    FILE *cfg = fopen("/data/local/tmp/retool.cfg", "r");
    if (!cfg) return NULL;

    // First pass: find LOG path or default
    log_fp = fopen("/data/local/tmp/retool.log", "a");
    if (!log_fp) { fclose(cfg); return NULL; }
    fprintf(log_fp, "=== retool init pid=%d ===\n", getpid());

    char line[1024];
    while (fgets(line, sizeof(line), cfg)) {
        if (line[0] == '#' || line[0] == '\n') continue;
        char cmd[64], a[256], b[64], c[64];
        int n = sscanf(line, "%63s %255s %63s %63s", cmd, a, b, c);
        if (strcmp(cmd, "LOG") == 0 && n >= 2) {
            fclose(log_fp);
            log_fp = fopen(a, "a");
        } else if (strcmp(cmd, "READ") == 0 && n >= 4) {
            cmd_read(strtoul(a, NULL, 0), strtoul(b, NULL, 0), c);
        } else if (strcmp(cmd, "READ_BY_LIB") == 0 && n >= 4) {
            void *h = dlopen(a, RTLD_NOW);
            void *p = h ? dlsym(h, b) : NULL;
            if (p) cmd_read((unsigned long)p, strtoul(c, NULL, 0), n >= 5 ? "hex" : "hex");
            else fprintf(log_fp, "DLSYM_FAIL lib=%s sym=%s\n", a, b);
        } else if (strcmp(cmd, "DUMP_MAPS") == 0) {
            cmd_dump_maps();
        } else if (strcmp(cmd, "EXIT") == 0) {
            break;
        }
    }
    fclose(cfg);
    if (log_fp) { fprintf(log_fp, "=== retool done ===\n"); fclose(log_fp); }
    return NULL;
}

__attribute__((constructor))
static void retool_init(void) {
    pthread_t t;
    pthread_create(&t, NULL, worker, NULL);
    pthread_detach(t);
}
```

## Step 2 — build script `retool/build.sh`
```bash
#!/bin/sh
NDK=${NDK:-$HOME/Android/Sdk/ndk/*/}
CLANG=$(ls $NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android*-clang | tail -1)
$CLANG -shared -fPIC -O2 -Wall -o libretool.so libretool.c -ldl -lpthread
file libretool.so
```
If NDK is absent on host, document the missing dep and emit `re_tool_lib_ndk_absent_2026_05_17` fact and stop.

## Step 3 — write a tiny host-side test program
`retool/test_host.c`:
```c
#include <stdio.h>
#include <unistd.h>
int main() {
    printf("test_host pid=%d\n", getpid());
    sleep(60);
    return 0;
}
```
Cross-compile to arm64, push to device, launch with `LD_PRELOAD=/data/local/tmp/libretool.so`, write a test config that does `DUMP_MAPS` + `READ` of `test_host`'s own `.text` + `EXIT`, verify `/data/local/tmp/retool.log` contains the expected output.

## Step 4 — document
`analysis/retool_README.md`:
- What the library does (read-only inspection at constructor time).
- How to load it (LD_PRELOAD for new processes; LKM injector or `am attach-agent` for running ones — note dependency on briefing `lkm-injector.md`).
- Config file syntax + example.
- v2 ideas (deferred — function-pointer hooks, kprobe-on-libc pattern, etc.) — list, do NOT implement.

## Outputs
- `retool/libretool.c`, `retool/build.sh`, `retool/test_host.c`, `retool/libretool.so` (built).
- `analysis/retool_README.md`
- `/data/local/tmp/retool.log` evidence file from the smoke test.
- Facts:
  - SUCCESS: `re_tool_lib_v1_smoke_passed_2026_05_17` = true
  - FAIL on NDK: `re_tool_lib_ndk_absent_2026_05_17` = true
- Final line: `RE_TOOL_LIB_DONE`

## References
- Briefing `lkm-injector.md` — the upstream tool that injects this .so into running processes
- Android NDK clang toolchain docs
- `/proc/<pid>/maps` format
