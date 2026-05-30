# Walls — confirmed dead ends

Each leaf here documents a path that was tried and failed, with the verbatim error and the reason why. **Do not retry without reading the leaf first.**

| Path | Result | Leaf |
|---|---|---|
| `frida -p <pid>` attach | "Process terminated" instantly | [frida-attach.md](frida-attach.md) |
| `frida -U -f ... -l <probe-script>` spawn (any non-trivial script) | "Process terminated" on script load | [frida-spawn-probe-load.md](frida-spawn-probe-load.md) |
| `cert_client <challenge>` | "connect: Connection refused" | [cert-client-listener.md](cert-client-listener.md) |
| `nmss_probe.sh` 16 payloads against /files/nmss | 16/16 zero bytes | [nmss-probe-empty.md](nmss-probe-empty.md) |
| `adb shell input tap` on title screen | tap registered, screen unchanged | [adb-input-tap.md](adb-input-tap.md) |
| Snapshot-replay arm_ptrace_helper for live capture | wrong code path | [snapshot-replay-path.md](snapshot-replay-path.md) |
| Frida Java bridge invisible on spawn+attach-before-resume | Java.available never true (111 polls, 90s) — both RPC and native-service loader stall | [frida-java-bridge-invisible.md](frida-java-bridge-invisible.md) (lateral: [lateral/frida-java-bridge-invisible.md](../lateral/frida-java-bridge-invisible.md)) |

## Cross-pollinated facts (in harness)
- `live_capture_fully_walled_cycle_666` — Frida hardening
- `cert_client_dead_end` — cert_client + cert_hook is a probe pair
- `os_tap_insufficient` — adb input tap doesn't clear title
- `live_cert_path_different_from_writer_cluster` — campaign reverse-engineered the wrong PCs for live path
