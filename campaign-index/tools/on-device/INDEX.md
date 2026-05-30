# Tools on device — `/data/local/tmp/`

| Tool | Status | One-liner | Leaf |
|---|---|---|---|
| xerda-server | available | 52 MB modified Frida server, listens TCP 27042 | [xerda-server.md](xerda-server.md) |
| cert_client | dead end | argv[1]=challenge probe binary; needs cert_hook.so injected | [cert-client.md](cert-client.md) |
| cert_hook.so | unused | the listener-side of cert_client (would need Frida to inject) | (covered in cert-client.md) |
| cert_service.so | unexplored | name suggests companion to cert_client | — |
| cert_direct_ptrace_service.so | unexplored | 24 KB ptrace service | — |
| nmss_probe.sh | dead end | sends 16 payloads to /files/nmss socket | [nmss-probe.md](nmss-probe.md) |
| arm_ptrace_helper | snapshot-only | for aeon_jit_replay, NOT live game | [arm-ptrace-helpers.md](arm-ptrace-helpers.md) |
| arm_ptrace_helper_bp512 | snapshot-only | variant of helper | (in arm-ptrace-helpers.md) |
| arm_ptrace_lane_watch | snapshot-only | lane watcher | (in arm-ptrace-helpers.md) |
| aeon_dump_process | available | userland process dump utility | — |
| **kernel module dump producer** | available | kmod that produced the `trampoline_proc_memdump_5558` snapshots — captures full process memory + register state from kernel space (kernel_dump_records.tsv lists per-region dumps incl. `[anon:dalvik-...]` mappings normally protected from userspace). This is what made the 5/5-replay snapshot possible. | — (deployed on device; remember it exists) |

## Existing artifact directories
- `/data/local/tmp/nmss/` — older certobj_*/formatter_chain_* dumps from snapshot-replay path
- `/data/local/tmp/aeon_capture/` — older snapshot captures (May 01–02)
- `/data/local/tmp/nmss_probes/log.txt` — output of nmss_probe.sh runs
- `/data/local/tmp/cert_client_rev/` — extracted cert_client binary (pulled by cert-ptrace cycle 648)
