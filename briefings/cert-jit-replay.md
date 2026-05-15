# cert-jit-replay — NMSS cert path: forward emulation past xor-fold

## Role & workdir
Drive the native Rust replay harness in `/home/sdancer/nmss-emu/native-replay-rs/` (`aeon_jit_replay` binary) to actually reach the MD5 worker forward in execution and capture donor frag1 input bytes. Sister agent: `cert-re-5` (static dataflow).

## Current goal / sub-goal
- Goal: `nmss_cert_re` — produce certs matching donor for `7BDA93D2F45D36C0` and `AABBCCDDEEFF0011`.
- Sub-goal: `cert_forward_md5_capture` — get past the xor-fold corridor performance wall, reach MD5 worker forward, capture exact input buffer.

## Success criteria
- `nmss_cert_replay_correct` fact set when `aeon_jit_replay` produces matching certs end-to-end.
- Intermediate win (sets `donor_md5_input_captured_2026_04_27`): a JSON checkpoint with the exact byte buffer passed to MD5 worker entry (0x78c686ccd4 or 0x78c6940d50) during a forward replay/emulation run that completes the cert flow.

## Why we're not actually blocked
The previous "RE final closure" framing was wrong. Re-reading the facts:
- `xor_fold_blocks_reaching_formatter_2026_04_26`: replay stops at xor-fold corridor 0x78c67e3d1c after 454k INTERP iterations, never reaches cert+0x68 producer at 0x78c6927f88. **Performance ceiling, not impossibility.**
- `formatter_admission_gates_2026_04_26`: outer formatter gates are upstream-cold because we never get there in time, not because the gate logic is unreachable.
- `xor_fold_output_capture_offline_md5_2026-04-26.json`: 0x78c67e3fa8 (xor-fold helper) IS reached and returns once in callback-bypass; we captured candidates and checked MD5 of the prefix recipe — only x1 buffer changed, MD5=`02ada684c6bb85601aa7020700541005`, didn't match donor `90237f0e03df6993a54669aa7cf27e36`. That's a single sample under the wrong upstream state, not proof the algorithm's wrong.
- We have **full forward emulation via aeon** — we can put a hook at the MD5 worker entry, run forward, and record the input buffer. That's the user's correct framing.

## Two concrete unblocks (pick whichever lands faster — likely B)

### Path A: JIT-promote 0x78c67e3d1c
Currently the hottest block (454k iters) but stays in the interpreter. Add it to the JIT promotion eligible set (or force compile via the JIT cache priming hook). Goal: drive iterations from interp speed to JIT speed; full xor-fold completes within the run window.
- Look in `native-replay-rs/src/bin/aeon_jit_replay.rs` for the JIT cache / forced-promotion hook (the same mechanism already used for SHA/MD5 fastpaths).
- Add a `--force-jit-pc 0x78c67e3d1c` flag, or hardcode it.
- Verify via `aeon_jit_result.json` that `stop_pc` advances past 0x78c67e3d1c and we reach the formatter (look for non-empty `cert+0x68` or `final_combiner_entry_lrs` containing 0x78c68f2694).

### Path B: Skip-fold trampoline (preferred — faster to iterate)
Bypass the xor-fold loop synthetically. Pre-compute the xor-fold output buffer offline (we already have the partial in `analysis/checkpoints/xor_fold_output_capture_offline_md5_2026-04-26.json`), seed it into the post-fold state, and resume execution at 0x78c68ef7d8 (formatter root, identified in cert-re's writer-chain checkpoint).
- Implement `--skip-xor-fold` flag in `aeon_jit_replay.rs`:
  - When PC reaches 0x78c67e3c80 (fold init, per recv chain in cert-re briefing fact `bug_root_cause_loop_at_0x78c6904900_2026_04_26`) or 0x78c67e3d1c, write the precomputed fold result into the destination buffer and jump to 0x78c68ef7d8.
  - The buffer location is whatever the fold's `materialize` (0x78c67e3fa8 → caller 0x78c686b108) writes to. Capture from a 60s natural run, save to a `.bin` file, replay it as the seed.
- Run forward; if MD5 worker is reached, **add an entry hook at 0x78c686ccd4 and 0x78c6940d50 that dumps x0 (input ptr) and x1 (length) plus 0x4041 bytes of the buffer** to `analysis/checkpoints/donor_md5_input_capture_<vector>.bin` + `.json`.

### After the buffer is captured
1. MD5(captured_buffer) — does it match donor frag1 `90237f0e03df6993a54669aa7cf27e36` for 7BDA?
2. If yes: the algorithm is `frag1 = MD5(<captured buffer>)`, and we just need to feed that buffer's source bytes correctly. Walk back from the captured buffer to find which donor inputs feed it (extractor input, slot writes, etc.).
3. If no: the captured buffer is one of the `frag2`/intermediate MD5 inputs — keep walking forward to other MD5 callers, and cross-check against frag2=last 16 hex chars of donor cert.

## Next 2–3 concrete tasks
1. **Read** `analysis/checkpoints/xor_fold_output_capture_offline_md5_2026-04-26.json` and `analysis/checkpoints/cert_writer_chain_0x78c68ef7d8_to_0x78c6927f88_2026-04-26.json` to confirm the fold-output materialize destination and the formatter resume PC.
2. **Implement Path B** (`--skip-xor-fold`) in `aeon_jit_replay.rs`. Capture fold output from a baseline 60s run, replay with the trampoline, hook MD5 entries.
3. **Save** the captured MD5 input buffer + length + caller PC to `analysis/checkpoints/donor_md5_input_capture_7BDA_2026-04-27.json` and run MD5 over it to compare to donor frag1. Report match/mismatch + first 64 bytes hex.

## Constraints & gotchas
- **NO Frida on `libUnreal.so`** — anticheat. Aeon emulation only.
- Don't get stuck looking for the perfect upstream state. The skip-fold trampoline is a known shortcut that gets us forward — that's the point.
- Both MD5 worker addresses must be hooked: 0x78c686ccd4 and 0x78c6940d50. Per `md5_implementations_inventory_2026-04-26.json`, `0x78c68c45e8 → 0x78c6940d50` is the cert-side path.
- The JIT cache lookup ABI is in aeon — check existing fastpath code for the registration pattern, don't invent a new one.
- If formatter actually completes and writes cert+0x68, dump it to compare against donor — that's the ultimate ground truth.

## Relevant files / references
- `/home/sdancer/nmss-emu/native-replay-rs/src/bin/aeon_jit_replay.rs` — replay binary, where to add `--skip-xor-fold` and MD5-entry hooks.
- `/home/sdancer/nmss-emu/analysis/checkpoints/xor_fold_output_capture_offline_md5_2026-04-26.json` — most recent fold-output sample.
- `/home/sdancer/nmss-emu/analysis/checkpoints/cert_writer_chain_0x78c68ef7d8_to_0x78c6927f88_2026-04-26.json` — formatter resume PC.
- `/home/sdancer/nmss-emu/analysis/checkpoints/md5_implementations_inventory_2026-04-26.json` — MD5 worker PCs.
- Donor expected certs: 7BDA → `90237F0E03DF6993A54669AA7CF27E36304273143AD6A030`, AABB → `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`.

## Operating mode
Codex agent (gpt-5.4 xhigh). Long-running runs are fine. Save partial work as JSON checkpoints early and often. Cross-pollinate with `cert-re-5` via `harness fact-set <key> <value>`.
