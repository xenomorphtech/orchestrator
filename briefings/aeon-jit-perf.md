# aeon-jit-perf — ARM live capture of selector-8 IO via adb (UNBLOCKED 2026-05-01)

## Role & workdir
You own ARM-side capture at `/home/sdancer/nmss-emu/native-replay-rs/`. **Substrate change 2026-05-01 evening**: the rk3588_s ARM target is now reachable as an adb device at `127.0.0.1:5558` (SSH-forwarded). Drive captures directly on hardware via `adb -s 127.0.0.1:5558 shell ...`. Local unicorn capture is no longer needed — use the real binary on the real device.

## Current goal / sub-goal
- Goal: `nmss_cert_replay_correct_pure_algo` — algorithmic Rust port of cert.
- Sub-goal: `cert_phase_d_selector8_arm_capture_5x` — capture selector-8 entry/exit state across 5 ground-truth challenges using the on-device static harness.

## Why this matters
cert-re-6 has spent 14+ cycles statically peeling the cert chain. The reduced model is:

```
sp+0x980 = selector8_output || package_name_bytes      (combine = concat)
selector8_output is the productive 64-char ASCII payload (per-row)
selector-8 = dispatch(0x78c6905424→0x78c693cccc, w0=8, x8=sp+0x6e0)
```

Static disasm has 14+ negative narrowings — selector-8 is heavily obfuscated state-machine, no direct crypto primitive calls, no rodata literal. **Live ARM capture is the fastest way to close the gap** now that the device is online.

## Success criteria
- For each of the 5 ground-truth challenges, dump entry+exit state at `0x78c6905424`/`0x78c693cccc` (selector-8 boundary): all GP regs, the 256-byte window at sp+0x6e0, exit value of x0/x21.
- Save to `analysis/checkpoints/selector8_io_arm_capture_<chal>_2026-05-01.json` (one file per challenge).
- Set fact `cert_phase_d_selector8_arm_io_<CHAL>_2026_05_01` per challenge.
- When 5/5 captured, set `cert_phase_d_selector8_arm_io_5x_complete_2026_05_01`.

## Substrate facts (set 2026-05-01)
- `arm_adb_device_online_2026_05_01` — device live, harness executes end-to-end.
- `arm_substrate_unblocked_2026_05_01` — replaces the blocked 162.244.80.97 path.

## On-device assets (under /data/local/tmp/nmss/)
- `native_jit_harness.static.aarch64` — static harness, takes `<jit_module.bin> <snapshot.bin>` plus optional flags (`--x9`, `--tls-base`, `--seed-*`, `--dump-x1`, `--dump-x2`, `--poke{8,32,64}`, etc.).
- `live_jit_manual7.bin` (4.5MB), `live_jit_snapshot_manual7_v2.bin` (10.4MB) — manual7 dev pair (single-challenge); produces a 48-hex artifact end-to-end.
- `nmsscr.dec` (5.1MB) — decrypted nmsscr.

`adb -s 127.0.0.1:5558 shell` is `shell` user (no su). All work has to live under `/data/local/tmp/`.

## Next 2–3 concrete tasks

1. **Sanity baseline** — run the harness on the existing manual7 pair and capture full stdout to `analysis/checkpoints/arm_baseline_manual7_2026-05-01.log`. Confirms substrate is sound before per-challenge work.
   ```
   adb -s 127.0.0.1:5558 shell 'cd /data/local/tmp/nmss && ./native_jit_harness.static.aarch64 live_jit_manual7.bin live_jit_snapshot_manual7_v2.bin'
   ```

2. **Locate / build a per-challenge jit_module + snapshot pair on the device**. The manual7 pair is single-challenge. For 5-vector capture, the device needs either (a) 5 distinct snapshot.bin files, one per challenge, or (b) the harness needs `--challenge`-style flag. Inspect the harness binary args and `live_jit_*` files; if no per-challenge support exists, ship a minimal one-shot wrapper (push to /data/local/tmp/) that `--poke8`-injects challenge bytes at the right offset before stage 0. cert-re's static map identifies the offset.

3. **Wrap the harness with `--dump-x2`/region-dump flags** to spill the 256-byte window at sp+0x6e0 at selector-8 boundary. Pull each dump back over adb (`adb pull`) into `analysis/checkpoints/selector8_io_arm_capture_<chal>_2026-05-01.json` (or `.bin` + sidecar metadata).

## Constraints & gotchas
- Use `python3` not `python`.
- Device is `shell` user — no root, no ptrace-attach to live processes; everything runs in /data/local/tmp/.
- Build local Rust replay scaffolds in `/home/sdancer/nmss-emu/native-replay-rs/` if needed for post-processing, but don't rebuild the unicorn replay path — substrate has shifted.
- Don't push huge files to device blindly; check free space with `adb -s 127.0.0.1:5558 shell df /data` first.
- Snapshot inputs (`*.bin`) are large (several MB); prefer `adb pull` over re-hashing.

## Relevant files / references
- `/home/sdancer/nmss-emu/analysis/cert_re_high_level_facts_2026-04-30.md` — current state map.
- `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json` — 5 ground-truth (challenge → cert) pairs.
- Facts: `harness facts | rg 'arm_substrate|arm_adb|cert_phase_d'`.

## Operating mode
Codex agent. ARM is unblocked; the campaign moves again. Save partial JSON checkpoints early. Cross-pollinate via `harness fact-set` so cert-rust-reimpl can begin reverse-modeling selector-8 from the captured IO pairs.
