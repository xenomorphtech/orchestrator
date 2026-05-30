# Lateral: bypass Frida-attach instant kill

**Wall**: `walls/frida-attach.md` — `frida -p <pid>` kills game + nmcore on attach.

## Experiments

1. `[x]` **Spawn mode + attach-before-resume + RPC agent shape** — confirmed working cycle 692, see `findings/paths/spawn-mode-rpc-survives.md`. This is the active production lane.

2. `[ ]` **`SIGSTOP` first, then attach** — send SIGSTOP to game pid before frida -p. Process is suspended; anti-debug timer reads can't fire while stopped. Worth testing once. Cost: 1 minute.
   ```
   adb shell 'kill -STOP 5588 && frida -p 5588 ...'
   ```

3. `[ ]` **`/proc/<pid>/mem` direct write via root** — bypass ptrace entirely, write Frida agent shellcode by mmap'ing /proc/<pid>/mem. Used by some advanced anti-anti-debug tools. Cost: significant tooling, but ptrace-free.

4. `[ ]` **Patch anti-debug check in libnmsssa.so** before runtime — disassemble the anti-debug routine (likely a TracerPid check or syscall hook) and NOP it out. Then `frida -p` may work. Requires identifying the check (search for `/proc/self/status` or `prctl(PR_SET_DUMPABLE)` or the watchdog thread that calls `tgkill`). Cost: ~half-day RE.

5. `[ ]` **Run inside an emulator with kernel control** — Android Studio AVD or genymotion + custom kernel that returns fake ptrace results. Anti-debug can't detect what isn't there. Cost: significant setup, but generic bypass.

6. `[ ]` **`process_vm_readv/writev` for memory access (no ptrace)** — these syscalls don't show up in `prctl(PR_SET_DUMPABLE)` checks the way ptrace does. Used for stealthy memory inspection. Cost: custom small C tool.

## Recommended next: experiment 1 is already production. If we ever need attach-mode (e.g. for an already-authenticated session), try experiment 2 (SIGSTOP) first — 1-minute test.
