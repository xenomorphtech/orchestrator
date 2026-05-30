# Lateral approaches — bypass experiments per wall

Each leaf lists 3–5 concrete experiments to bypass a documented wall, varying in effort/risk. Status legend: `[ ]` untried, `[~]` tried partially, `[x]` tried fully (move outcome to walls/ or findings/), `[?]` requires user decision.

| Wall | Lateral file | Best untried lateral |
|---|---|---|
| Frida-attach kills | [frida-attach.md](frida-attach.md) | LD_PRELOAD via repackaged APK |
| Frida-spawn probe-load kills | [frida-spawn-probe-load.md](frida-spawn-probe-load.md) | RPC agent shape (CONFIRMED bypass cycle 692) |
| cert_client no listener | [cert-client-listener.md](cert-client-listener.md) | strace nmcore to capture protocol |
| nmss-probe empty | [nmss-probe-empty.md](nmss-probe-empty.md) | retry post-tap once auth state present |
| adb input tap doesn't work | [adb-input-tap.md](adb-input-tap.md) | direct uinput / sendevent |
| live cert empty pre-tap | [pre-tap-cert-empty.md](pre-tap-cert-empty.md) | inject auth state from authenticated dump |

## Worth picking up next (cycle 704)
Now that spawn-mode RPC is confirmed working, the next active blockers are:
1. **Title-tap gate** — see `lateral/adb-input-tap.md`. Pick the cheapest bypass (sendevent or /dev/input write).
2. **cert_direct_ptrace_service.so** path (cert-ptrace exploring this cycle 704) — its symbols suggest a non-Frida injection lane already shipped on device. If it works, sidesteps the entire Frida wall family.
