# Tools we've built / adapted

| Tool | Path | Status | One-liner |
|---|---|---|---|
| Phase 1 reproducer | `cert-rust-repro/src/bin/cert_rust_repro.rs` | working | takes `sp+0x968` 64B → SHA-256 → bswap32 → digest[4..28] = 48-char cert; verified vs donor |
| Stealth keepalive Frida script | `analysis/frida/stealth_spawn_keepalive_2026-05-02.js` | available | no-op script for `frida -U -f` clean spawn (only safe Frida script) |
| nmss_java_probe.js | `analysis/frida/nmss_java_probe_2026-05-02.js` | walled | once-positive probe; now triggers fast-kill on script load (cycle 666) |
| nmss_native_cert_single_template.js | `analysis/frida/nmss_native_cert_single_template_2026-05-02.js` | walled | batch collector for 5 challenges; same fast-kill |
| nmss_java_attach_live.js | `analysis/frida/nmss_java_attach_live_2026-05-02.js` | walled | attach mode, kills process |
| nmss_java_attach_minimal.js | `analysis/frida/nmss_java_attach_minimal_2026-05-02.js` | walled | minimal attach probe, kills |
| live_sp968_spawn.js | `analysis/frida/live_sp968_spawn_2026-05-02.js` | walled | sp+0x968 capture, kills |
| getcert_trampoline_fork_ptrace_5558.py | `/home/sdancer/nmss-emu/frida/getcert_trampoline_fork_ptrace_5558.py` | reusable pattern | host-side RPC pattern (source for the new RPC lane) |
| nmss_live_rpc_agent.js | `analysis/frida/nmss_live_rpc_agent_2026-05-02.js` | built cycle 688, not yet live-tested | exports: `ping/status/waitforready/initnmss/getcert/getall` |
| nmss_live_rpc_5558.py | `frida/nmss_live_rpc_5558.py` | built cycle 688, not yet live-tested | spawn-owned, attach-before-resume; interactive STATUS/INIT/GETCERT/GETALL/PING/QUIT |
| **nmss_live_cert_service.c** | `scripts/nmss_live_cert_service.c` | built cycle 710, validated, NOT YET DEPLOYED | native .so source — JNI_OnLoad captures JavaVM, start_nmss_live_cert_service receives NmssSa+Activity refs and serves status/init/getcert/getall over native pthread TCP. **Drops Frida after handoff** — way smaller anti-debug footprint than per-call agent. |
| **build_nmss_live_cert_service.sh** | `scripts/build_nmss_live_cert_service.sh` | built cycle 710, cross-build OK | builds the .so via NDK at `/home/sdancer/android-sdk/ndk` |
| **nmss_live_cert_service.so** | `scripts/nmss_live_cert_service.so` | built cycle 710 | the compiled aarch64 .so, exports JNI_OnLoad + start_nmss_live_cert_service |
| **nmss_live_cert_service_loader.js** | `analysis/frida/nmss_live_cert_service_loader_2026-05-02.js` | built cycle 710, node --check OK | minimal Frida loader: System.load() from Java.perform, hands NmssSa+Activity to native start function, then can disconnect |
| **nmss_live_cert_service_5558.py** | `frida/nmss_live_cert_service_5558.py` | built cycle 710, --help OK | Python host that drives the loader + talks to the in-process TCP service |

## Reproducer reference inputs
- `cert-rust-repro/donor_session_2026-04-29.json` — working 7BDA donor (snapshot path)
- `cert-rust-repro/captures/<challenge>.json` — target output location for the 5-challenge live captures (when we get them)

## Phase D doc trail (analysis/checkpoints/)
- `cert_phase_d_campaign_final_2026-05-02.md` — algorithm structural doc
- `cert_phase_d_sha256_reframe_check_2026-05-02.md` — stale "expected witnesses" table (snapshot-derived)
- `cert_phase_d_full_pipeline_test_2026-05-02.md` — Rust E2E mismatch trace
- `cert_transform_bridge_serializer_static_2026-05-02.md` — 0x2d7284 spec source
- `cert_transform_helper_semantics_2026-05-02.md` — Ed25519/SHA-512 identification source
- `cert_0x240f3c_sig_source_dispatch_2026-05-02.md` — arg2 source dispatch trace
- `cert_sp_968_writer_cluster_static_2026-05-02.md` — writer cluster offsets
- `live_cert_capture_wall_summary_2026-05-02.md` — full dead-end matrix (auto-updated)
