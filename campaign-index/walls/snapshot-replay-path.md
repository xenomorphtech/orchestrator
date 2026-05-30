# Wall: snapshot-replay arm_ptrace_helper does not capture live cert path

**Confirmed**: cycle 633, 2026-05-02

## Symptom
The campaign reverse-engineered an algorithmic chain ending in a writer cluster at PCs `0x78620dad04..dad14` (writes to `sp+0x968`). When this site was instrumented on the **live game**, it fired ZERO times during normal cert generation. The Java `nmssNativeGetCertValue` call returned a real cert (`3763E9656BF1116EAB35AF137F59F72689ACFAD286EEB7AE` for 7BDA) without ever touching that PC.

## Why
The reverse-engineered chain corresponds to the path the **`aeon_jit_replay` snapshot binary** takes when replaying a captured trace on-device. The **live game** uses a different code path entirely for `nmssNativeGetCertValue`, which we have NOT yet reversed.

## Implication
- The "expected witnesses" table (e.g. `90237F0E03DF6993...` for 7BDA) in `cert_phase_d_sha256_reframe_check_2026-05-02.md` came from the snapshot path and is **stale** for live ground truth
- The only valid live cert observed is `3763E965...` for 7BDA from the cycle-633 fluke
- Existing `certobj_root_*` and `formatter_chain_*` dumps under `/data/local/tmp/nmss/` came from snapshot replay, not live game

## What to try instead
- Capture live `nmssNativeGetCertValue` outputs (the campaign's current goal) — this is what the cert-ptrace agent is doing
- Once we have 5 live captures, RE the actual live path (different starting point than the writer cluster)

## Forbidden
- Do NOT use snapshot-derived expected witnesses to validate live captures
- Do NOT try to instrument the writer cluster PCs for live capture
