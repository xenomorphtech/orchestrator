# NMSS Overview

Last consolidated from harness facts on 2026-04-08.

## Goal

The active top-level goal is:

- fix the NMSS cert emulator so it produces the same cert token as the live device for a given challenge

## Current State

The project is past the earliest "all-zero output" and "wrong code image" failures. The emulator is now doing real work, consuming live-ish state, and producing structured output, but it still does not reliably match the live device because several inputs are session-specific and process-specific.

The strongest current model is:

1. The cert path depends on live JIT state and owner/container state captured from the running process.
2. The computation is process-dependent because address-bearing values leak into the hash path.
3. Same-session capture is required. Mixing a session key, JIT dump, detection vectors, and cert outputs from different spawns gives misleading results.

## What We Know

### Object model

- `owner` and `manager` are the same object.
- The important owner fields observed so far are:
  - `+0x210`: SSO string `"FERUN"`
  - `+0x288`: UDID-like value `7b0d26cdc87d42ea`
  - `+0x314`: score, seen live as `0x0f`
  - `+0x340`: device id, seen live as `0x00000001`
  - `+0x390`: detection-state container

### Detection container

The current best understanding is that the detection container lives at `owner+0x390` and contains 0x140-byte records in at least these vector pairs:

- `container+0x10/+0x18`: source vector, 12 records
- `container+0x588/+0x590`: compare vector 1, 5 records
- `container+0x5a0/+0x5a8`: compare vector 2, 4 records

### Cert behavior

- Challenge input is consumed. Different challenges do change the cert.
- Session key matters, but it is not the only thing that matters.
- Cert values vary across process spawns.
- Emulator output has moved from trivial fallback values to real computed values, but same-session parity is still not established.

## Best Current Root Cause

The most useful current root-cause statement is:

- the cert path is hashing process-specific values, including address-shaped data and session-specific state, and the emulator still diverges from the device on those inputs

That conclusion is supported by several independent findings:

- cert values change across process spawns
- `_sprintf_fast` / S1 formatting leaks address-like values into the hash path
- stale or mixed-session captures produce consistent but wrong certs
- stale snapshots and hook-heavy runs can perturb the computation

## Active Blockers

- Need a complete same-session capture that includes session key, JIT base, SHA input, detection buffer, S1 args, and cert values from the same process.
- Need to eliminate remaining stale-snapshot contamination in emulator inputs.
- Need to determine whether the emulator can be patched into parity faster than the native harness can be stabilized.

## Recommended Reading

- [Cert Emulator](Cert-Emulator.md)
- [Native Harness](Native-Harness.md)
- [Artifacts](Artifacts.md)
- [Timeline](Timeline.md)

