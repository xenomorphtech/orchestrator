# md5-input-capture (H-N10) — capture the actual 64-byte MD5 input blocks per challenge

**You ARE allowed and expected to write code.** Python (modify unicorn replay), Rust (port once captured).

## Role & workdir

Capture the **actual 64-byte input blocks** fed to MD5 during cert generation, per challenge. The cert algorithm is confirmed-MD5 (inline T-table at PC 0x113700 from md5-sha-fit cycle 80), but the input cannot be derived from static data — it's RUNTIME-DERIVED. With 10-15 captured (challenge, MD5_input_block, MD5_output) tuples across 5 challenges, the input formula becomes trivially recoverable. Workdir: `/home/sdancer/nmss-emu-md5-input-capture/`.

## Why this path

md5-sha-fit (cycle 80) hard-gated (c) with conclusive findings:
- **MD5 IS the primitive** — inline obfuscated T-table at PC 0x113700 (movz/movk+neg pattern). Constants `0xeb86d391`, `0xe8c7b756`, `0x242070db` verified.
- **H-N8's reg interpretation was correct**: `x16=bswap32(state[1])` of `MD5_compress` is real MD5 state, not noise.
- **BUT the 64-byte MD5 input doesn't decode from static data** — it must contain runtime-derived bytes (timestamps, mmap/heap state, earlier-transform output, fd state, etc.).

The md5-sha-fit worker noted that `replay_trampoline_snapshot_unicorn.py` **already has MD5 loop entry points wired**:
- `MD5_LOOP_ENTRY=0x78CD3C0E38`
- `MD5_SCHEDULE_X29_DELTA=-0xA8`
- `CFF_MD5_LOOP_ENTRY=0x78CD478590`
- `CFF_MD5_SECOND_LOOP_ENTRY=0x78CD484A54`

These are from prior campaign work (someone instrumented MD5 before). They give the entry points where the MD5 64-byte block lives at `x29-0xA8`. Adding a dump there per challenge is mechanical.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5 if input formula recovered).
- **Sub-goal:** 10-15 verified (challenge, MD5_input_block, MD5_output) tuples per challenge → derive the input formula by inspection.

## Success criteria

- **Minimum**: 2-3 MD5 input blocks captured per challenge × 5 challenges = 10-15 dumps. Save `analysis/md5_inputs_2026-05-11.jsonl`.
- **Stretch**: Identify the input formula by analyzing the dumps. Test it: predict cert from challenge using the formula. ≥3/5 match → fact `cert_md5_input_formula_2026_05_11` with the spec.
- **Campaign close**: 5/5 → port to Rust, set fact `nmss_cert_5_5_pure_rust_reproduced` ← campaign success-fact-key.

## Inputs you have

- **Existing unicorn replay script with MD5 hooks ALREADY wired**: `/home/sdancer/nmss-emu-trampoline/replay_trampoline_snapshot_unicorn.py` (per md5-sha-fit worker note). Contains constants `MD5_LOOP_ENTRY=0x78CD3C0E38`, `MD5_SCHEDULE_X29_DELTA=-0xA8`, `CFF_MD5_LOOP_ENTRY=0x78CD478590`, `CFF_MD5_SECOND_LOOP_ENTRY=0x78CD484A54`.
- **Patched native-replay-rs (HW-BP)**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` — supports `--trace-call-hw <hex>`. Could alternatively be extended to dump memory at `[x29-0xA8]+0..64` on hit.
- **md5-sha-fit deliverables**: `/home/sdancer/nmss-emu-md5-sha-fit/analysis/md5_part_fit.json` (confirmed-MD5 + recommended H-N10 approach).
- **5 ground-truth (challenge, cert) pairs**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- **80MB of H-N8 captures**: `/home/sdancer/nmss-emu-cert-builder-hw-bp/analysis/chal_*.json` — may already contain memory dumps near x29-0xA8 from earlier BPs; check before re-capturing.
- **Cert orchestrator disasm**: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_builder_0x78c689575c_disasm.txt`. Search for the MD5 loop and verify x29-0xA8 is the block argument.

## Next 3 ordered tasks

1. **Verify the MD5 loop entry points**. Check the disasm at `0x78CD3C0E38`, `0x78CD478590`, `0x78CD484A54` to confirm they're MD5_compress call sites with the 64-byte block at `[x29-0xA8]`. Also: cross-check with H-N8 BP 0x78c68a07b0 — is that the SAME loop as the unicorn script's MD5_LOOP_ENTRY? Determine the runtime->module-rel offset and map them.

2. **Add memory dump at each MD5 loop entry**. EITHER (a) modify the unicorn replay script `replay_trampoline_snapshot_unicorn.py` to print 64 bytes at `x29-0xA8` whenever PC hits any of the 4 MD5_LOOP_ENTRY addresses, OR (b) extend the patched native-replay-rs HW-BP infra to accept a `--dump-mem-deref <reg+offset:size>` flag and run with `--trace-call-hw 0x78CD3C0E38 --dump-mem-deref x29-0xA8:64`. Pick whichever is faster. Run for 5 challenges. Save `analysis/md5_inputs_2026-05-11.jsonl` with `{challenge, md5_hit_idx, pc, block_hex_64, md5_output_state_post}`.

3. **Recover input formula + verify**. Inspect the captured blocks: look for embedded challenge ASCII, secret hex, package name, device_info bytes. Identify the construction (e.g. "block = challenge_ASCII (16B) + secret_hex_ASCII (32B) + counter (4B) + padding (12B)"). Implement in Rust at `cert_md5/src/lib.rs`. Validate against 5 ground-truth certs. If 5/5 → CAMPAIGN COMPLETE → set fact + escalate to user.

## Constraints & gotchas

- **No git commits.**
- **MD5 with padding** has 56 + 8 = 64-byte single-block messages (padded), or 64+ bytes spanning multiple blocks. The "MD5_LOOP_ENTRY" suggests the worker captured the per-block call. The H-N8 chal+secret+chal = 16+32+16 = 64 ASCII chars exactly fills ONE block (without padding). So the MD5 may be `MD5(block1) || MD5(block2)` chained, with block1 = chal+secret+chal and block2 = padding+length. Then cert = MD5 final state.
- **Each MD5 loop iteration processes 64 bytes** — for an input >55 bytes there are TWO iterations: the message itself plus the padding block. Capture BOTH; they're both runtime-derivable.
- **Cross-challenge invariance**: blocks across 5 challenges should differ ONLY in the challenge bytes (assuming secret + other fields are constant). Diff to localize where the challenge appears in each block.

## Relevant files / references

- md5-sha-fit deliverables: `/home/sdancer/nmss-emu-md5-sha-fit/analysis/md5_part_fit.json`, scripts under same dir.
- Existing unicorn replay (with MD5 hooks): `/home/sdancer/nmss-emu-trampoline/replay_trampoline_snapshot_unicorn.py` — verify this path; may be in a different worktree.
- H-N8 captures: `/home/sdancer/nmss-emu-cert-builder-hw-bp/analysis/chal_*.json`
- H-N4 ground truth: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- Patched binary: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/md5_input_capture_progress_2026-05-11.jsonl`. Stages: `entry_points_verified`, `dump_infra_ready`, `5x_captured`, `formula_recovered`, `5_of_5_match_CAMPAIGN_COMPLETE`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) 5/5 cert match → **CAMPAIGN COMPLETE**. Set facts: `cert_md5_input_formula_2026_05_11` + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.
- (b) ≥3/5 match → likely formula identified with residual edge cases.
- (c) Captures successful but no clean input formula derivable → write `analysis/md5_input_blocker.md` with the captured raw data so a future worker can apply more techniques (correlation, ML byte-pattern matching, etc.).
- (d) Captures unsuccessful (entry points wrong / unicorn replay unavailable) → write blocker, fall back to HW-BP capture via native-replay-rs.
