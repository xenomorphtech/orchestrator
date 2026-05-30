# Findings — confirmed campaign facts

| Topic | One-liner | Leaf |
|---|---|---|
| **Native cert service end-to-end** (cycle 730→731 FULL SUCCESS) | Frida CLI spawn + top-level loader + scheduleOnMainThread + in-process synthetic touch yields all 5 non-empty live `nmssNativeGetCertValue` captures | [paths/native-service-end-to-end.md](paths/native-service-end-to-end.md) |
| **Spawn-mode RPC agent SURVIVES** (cycle 692 BREAKTHROUGH) | spawn-owned + attach-before-resume + rpc.exports shape escapes the script-load wall | [paths/spawn-mode-rpc-survives.md](paths/spawn-mode-rpc-survives.md) |
| **Java bridge visible at 501ms via event-loop, not sync RPC** (cycle 719) | sync rpc.exports starves Frida JS event loop; setInterval+Promise pattern works | [paths/event-loop-vs-sync-rpc.md](paths/event-loop-vs-sync-rpc.md) |
| Algorithm = Ed25519 + SHA-512 | crypto identification at 0x2863f8/0x284e3c/0x2d6538/0x2e08bc | [algorithm/ed25519-sha512.md](algorithm/ed25519-sha512.md) |
| 0x2d7284 serializer spec | exact bridge serializer append semantics | [algorithm/serializer-bridge.md](algorithm/serializer-bridge.md) |
| sp+0x968 writer cluster PCs | 0x78620dad04..dad14 stores (snapshot path only) | [algorithm/writer-cluster.md](algorithm/writer-cluster.md) |
| Live vs snapshot cert paths differ | writer cluster fires 0x times on live | [paths/live-vs-snapshot.md](paths/live-vs-snapshot.md) |
| 7BDA → 3763E965 (live, fluke) | only live ground truth captured | (in `paths/live-vs-snapshot.md`) |
| Phase 1 algorithm decoded | sp+0x968 64B → SHA-256 → digest[4..28] → 48-char cert | (in `algorithm/ed25519-sha512.md`) |
| Cert is session-id dependent | same challenge → same hash within session | (memory: `nmss_cert_session_id_dependence.md`) |
| Anti-debug timing reads exist | wall-clock reads happen but don't feed algo | (memory: `nmss_cert_anti_debug_timing.md`) |
| nmcore owns cert socket | `/files/nmss` listener is nmcore not game | (in `paths/live-vs-snapshot.md`) |

## Cross-pollinated facts (in harness)
- `algorithm_is_ed25519_sha512`
- `serializer_append_spec_exact`
- `cert_transform_helper_semantics_2026_05_02`
- `cert_socket_owner_is_nmcore`
