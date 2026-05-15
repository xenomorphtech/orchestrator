# aeon-jit-x86 — Local x86/unicorn JIT replay capture (parallel lane)

## Role & workdir
You own local unicorn-emulator JIT replay at `/home/sdancer/nmss-emu/native-replay-rs/`. **Parallel lane** to aeon-jit-perf (which now owns ARM-direct ptrace capture). You stay on x86: drive the local `aeon_jit_replay` Rust binary against the trampoline_proc_memdump_5558 corpus to capture selector-8 IO that cert-rust-reimpl can reverse-model.

## Current goal / sub-goal
- Goal: `nmss_cert_replay_correct_pure_algo` — algorithmic Rust port of cert.
- Sub-goal: `cert_phase_d_selector8_unicorn_capture` — capture selector-8 entry/exit state across 5 ground-truth challenges using the local unicorn JIT replay infra.

## Why this matters
cert-re-6 spent 14+ cycles statically mapping the cert chain. The reduced model is:

```
sp+0x980 = selector8_output || package_name_bytes      (combine = concat)
selector8_output is the productive 64-char ASCII payload (per-row)
selector-8 = dispatch(0x78c6905424→0x78c693cccc, w0=8, x8=sp+0x6e0)
```

The unicorn replay is challenge-insensitive at cert-emit (carrier-refresh bug), but selector-8 at its boundary still computes per-input bytes. Capturing those across challenges gives cert-rust-reimpl differential signal even if the final cert doesn't change.

## Success criteria
- For each of 5 ground-truth challenges, capture entry+exit registers + memory at sp+0x6e0 (256-byte window) at PCs `0x78c6905424` (call site) and `0x78c693cccc` (callee entry).
- Save `analysis/checkpoints/selector8_io_unicorn_capture_<chal>_2026-05-01.json`.
- Set fact `cert_phase_d_selector8_unicorn_io_<CHAL>_2026_05_01` per challenge; `cert_phase_d_selector8_unicorn_io_5x_complete_2026_05_01` when all 5 done.

## Cross-pollination from ARM lane (aeon-jit-perf)
- `arm_snapshot_challenge_offset_2026_05_01` — challenge field is 16 ASCII bytes at snapshot offset 0x2150 in NJITSNP2 snapshots (the ARM substrate).
- `arm_postret_challenge_invariant_2026_05_01` — manual7_postret produces 4 hex48 surfaces, all challenge-invariant. Carrier mixing happens between snapshot capture and these surfaces. Same problem the unicorn replay has at cert-emit.
- `arm_ptrace_native_harness_pivot_2026_05_01` — ARM lane is now ptracing the harness mid-run to bypass the snapshot wall. Cross-pollinate findings.

## Concrete next 2-3 tasks

1. **Build & sanity-check** `cargo build --release --bin aeon_jit_replay` in `/home/sdancer/nmss-emu/native-replay-rs/`. Run once with default args + `--challenge 0000000000000000` and confirm the replay completes.

2. **Add probe at selector-8 boundary**: Instrument `aeon_jit_replay.rs` with capture at PCs `0x78c6905424` (call site) and `0x78c693cccc` (callee entry). Capture x8 input pointer dump (256B at sp+0x6e0 before/after — selector-8 may write back via x8), w0 (should be 8), all general regs entry/exit, return x0. Pattern-match existing probes (e.g., `frag2_record_iter_head` capture in prior session work).

3. **Run for all 5 challenges**:
   ```
   for CHAL in 0000000000000000 0123456789ABCDEF 1111111111111111 7BDA93D2F45D36C0 AABBCCDDEEFF0011; do
     env AEON_CAPTURE_UNICORN_PC_SNAPSHOTS=1 \
         AEON_UNICORN_EXTRA_PCS='selector8_call:0x78c6905424,selector8_entry:0x78c693cccc' \
         AEON_UNICORN_PC_SNAPSHOT_MAX_HITS=4 \
         AEON_UNICORN_PC_SNAPSHOTS_OUTPUT_DIR=/home/sdancer/nmss-emu/analysis/checkpoints/selector8_io_unicorn_capture_2026-05-01 \
         AEON_STOP_AFTER_EXECUTED_BLOCKS=1700000 \
         target/release/aeon_jit_replay ../trampoline_proc_memdump_5558 \
         --skip-cff-jump-to-callback --challenge $CHAL \
         --cert-recipe ../analysis/checkpoints/cert_full_recipe_2026-04-25.json \
         --wall-timeout-sec 300 --prealloc-output \
         --out /tmp/sel8_$CHAL.json > /tmp/sel8_$CHAL.log 2>&1
   done
   ```
   Even if final cert is challenge-insensitive (carrier-refresh bug), selector-8's input/output bytes should still vary.

## Constraints & gotchas
- Use `python3` not `python`.
- Build: `cd /home/sdancer/nmss-emu/native-replay-rs && cargo build --release --bin aeon_jit_replay`.
- Probe machinery already in `aeon_jit_replay.rs` from prior `frag2_record_iter_head` capture work — pattern-match.
- Don't break existing flags.
- Don't reach for ARM/adb — that's aeon-jit-perf's lane.
- The unicorn replay produces same `90237F0E…` cert for all 5 challenges (carrier-refresh bug); that's expected. We're capturing pre-cert state at selector-8, not the final cert.

## Relevant files / references
- `/home/sdancer/nmss-emu/native-replay-rs/src/bin/aeon_jit_replay.rs` — replay binary
- `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json` — 5 ground-truth pairs
- `/home/sdancer/nmss-emu/analysis/cert_re_high_level_facts_2026-04-30.md` — current state map
- Facts: `harness facts | rg 'cert_phase_d|arm_'`

## Operating mode
Codex agent. Save partial JSON checkpoints early. Cross-pollinate via `harness fact-set` so cert-rust-reimpl can begin reverse-modeling selector-8 from the captured IO pairs. Stay on x86 even if ARM lane gets exciting — parallel coverage is the point.
