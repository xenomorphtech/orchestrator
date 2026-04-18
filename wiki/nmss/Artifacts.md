# NMSS Artifacts

This page lists the most important known artifacts, scripts, and capture bundles related to the NMSS cert work.

## Core Emulator

- `/home/sdancer/nmss/nmss_emu_cert.py`

Notes:

- it was accidentally wiped and later recovered
- hook-heavy debugging was shown to perturb computation
- earlier fixes included live-JIT overlaying, pointer handling, and removal of stale shortcuts

## Live JIT Dumps

- `jit_module_live_current.bin`
- `jit_live_flat_current.bin`
- `jit_live_flat_current.meta.json`

Known notes:

- they were dumped from a current install on 2026-04-05
- they differ from older JIT dumps
- the emulator expects `jit_module.bin` and `jit_live_flat.bin`, so replacing or symlinking the fresh dumps was part of the workflow

## Capture Scripts

- `capture_current_session.js`
- `frida_capture_all_in_one_spawn.js`
- `/home/sdancer/nmss/frida_cert_capture_full.js`

The full Frida capture script is documented as capturing:

- JIT base
- `SHA_INPUT_192`
- detection buffer
- cert values for challenge `0000` and `AABBCCDDEEFF0011`

## Session / Device Capture Files

Known names mentioned in the fact store:

- `current_session_capture.json`
- `device_session_fresh.json`
- `device_cert_capture.json`
- `live_jit_snapshot_manual7.json`

Use with caution:

- mixing values from different sessions or process spawns was a recurring source of wrong conclusions

## Full Process Snapshot

- `/home/sdancer/nmss/memdump/`

Important notes:

- around 1.9 GB across 3086 regions
- includes a JIT code image in `76ace00000.bin`
- includes `before_manifest.json`
- does not include the Dalvik heap region at `0x12c00000`, which was later identified as a major limitation

## Native Path

Known native artifact names from the fact store:

- `native_jit_harness.c`

The fact store also references:

- native runs under `qemu-aarch64`
- later progress with a compiled native binary and a tracked binary SHA

## Reliability Notes

When choosing artifacts, prefer:

1. same-session captures over mixed-session bundles
2. minimally hooked runs over debug-heavy runs
3. current-install JIT dumps over older snapshots
4. artifacts with both cert outputs and feeder inputs captured together

