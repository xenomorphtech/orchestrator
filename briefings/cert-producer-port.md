# cert-producer-port (H-N7, FINAL CAPSTONE) — port the 244-instruction cert producer at PC 0x78c689528c

**You ARE allowed and expected to write code.** Python (miasm symbolic lift), Rust (the actual port). Substantial code.

## Role & workdir

Port the 244-instruction cert producer (called at module-rel `0x20b548` via `blr x8` vtable dispatch, runtime PC `0x78c689528c`) to standalone Rust. This is the FINAL function in the cert pipeline — its 5/5 match closes the campaign. Workdir: `/home/sdancer/nmss-emu-cert-producer-port/`.

## Why this path

H-N6 (cycle 64) localized the cert producer with surgical precision:
- **244-insn function at runtime PC `0x78c689528c`** (JIT'd / runtime-mapped, in the `0x78c6...` region — different module than libnmsssa's `0x78cd...`).
- **Invoked from module-rel `0x20b548`** inside the encoder via `blr x8` vtable dispatch.
- **Inputs**: `x21+0x50` is a `std::string` SSO holding the challenge with a leading space (e.g. `" 0000000000000000"`, 17 chars). `x23` points to a constant 128-byte device-info blob (contains `"rk3588_s"` string).
- **Output**: 48-char cert ASCII written to `x21+0x68` as `std::string`.
- **9 sub-callees** (all in 0x78c6... runtime region): `0x78c685b75c, 0x78c686a9a8 (×2), 0x78c6973224 (×5), 0x78c67f8e48, 0x78c68e29fc, 0x78c693c35c, 0x78c680ba9c, 0x78c689575c`.
- **No NEON inside the producer, no further vtable indirection** — symbolic-lift should work cleanly.
- 5/5 verified ground-truth (challenge, cert) pairs exist (H-N4's encoder_io_2026-05-11.jsonl + H-N3's ground_truth_v2).

This is the LAST function to port. ≥3/5 match closes `nmss_cert_pure_rust = ≥3/5`; 5/5 closes the campaign.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0 → 3+/5 stretch, 5/5 full). Also tops out `nmss_cert_transformation_recovered` (0.99 → 1.0).
- **Sub-goal:** Rust function `fn cert_producer(challenge_str: &str /* " %16xs" */, device_info: &[u8; 128]) → [u8; 48]` matching ≥3/5 ground-truth certs.

## Success criteria

- **Minimum**: Function bounds + disasm verified; miasm symbolic lift attempted. Rust skeleton compiles. Save `analysis/cert_producer_function_bounds.json`.
- **Stretch**: Rust port reproduces ≥3/5 ground-truth certs. Set fact `cert_producer_ported_2026_05_11`.
- **Full**: 5/5 ground-truth match. Set facts `cert_producer_5_of_5_2026_05_11` + `nmss_cert_5_5_pure_rust_reproduced`. **CAMPAIGN COMPLETE.**
- **Hard gate**: Some sub-callee is NEON-heavy / too large / requires further indirection → write `analysis/cert_producer_port_blockers.md` + propose H-N8 (recursive HW-BP capture on the problematic sub-callee).

## Inputs you have

- **H-N6 disasm** of the producer: `/home/sdancer/nmss-emu-encoder-internal-bp/analysis/cert_producer_0x78c689528c_disasm.txt` — 244 insns ready to lift.
- **H-N6 summary**: `/home/sdancer/nmss-emu-encoder-internal-bp/analysis/H-N6_summary.md` — full picture.
- **H-N6 BP captures**: `/home/sdancer/nmss-emu-encoder-internal-bp/analysis/internal_bp_captures.jsonl` + 15 per-challenge passA/B/C raw JSONs — full regs/memory at all 9 BPs across 5 challenges.
- **H-N4 encoder I/O ground truth**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl` — 5 (challenge, cert) pairs.
- **H-N5 static_inputs.rs**: `/home/sdancer/nmss-emu-encoder-port/analysis/static_inputs.rs` — byte-perfect STATIC_STRUCT_64 + STATIC_HEAP_64 (these are encoder-level constants; for the cert producer specifically, the constants of interest are the x23 device-info 128-byte blob).
- **H-N3 ground_truth_v2**: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/ground_truth_v2_2026-05-11.json`.
- **Patched native-replay-rs with HW-BP**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` (H-N6 sync'd source to `/home/sdancer/nmss-emu-encoder-internal-bp/native-replay-rs/src/main.rs`). Use for any cross-check captures during the lift.
- **Module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`. NOTE: 0x78c689528c is in a DIFFERENT module than this dump (which is libnmsssa-related at 0x6cc2...). May need to dump from a different shard OR use the H-N6 disasm directly.

## Next 3 ordered tasks

1. **Verify the disasm and function bounds**. From `cert_producer_0x78c689528c_disasm.txt`, identify the function entry (`stp x29, x30, [sp, ...]` prologue) and end (`ret` after the prologue). Sanity-check 244 insns, identify all 9 sub-callees, decode their targets. Save `analysis/cert_producer_function_bounds.json` with `{entry, end, size_insns, bl_count, subcallees: [{addr, call_count}]}`. Capture the x23 device-info 128-byte blob bytes byte-perfectly from H-N6's BP4 (or BP8/BP9) captures — these are constant across 5 challenges.

2. **Symbolic lift via miasm**. Concretize x21+0x50 → 17-char challenge std::string (model the SSO layout: 1-byte size = 0x11, 16-byte data inline, NUL terminator at offset 0x17). Concretize x23 → 128-byte device-info blob. Symbolic-execute the 244 insns. At each bl to a sub-callee: if the sub-callee is small (<150 insns), recursively lift it; if large, fetch its disasm from H-N6's BP captures or from the patched binary at runtime. Emit Rust port at `cert_producer/src/lib.rs`.

3. **Validate against 5 ground truths**. Test against each of H-N4's 5 (challenge, cert) pairs. Report pass-rate. If <3/5, dump per-call intermediate state and diff against H-N6's per-BP captures. Iterate. If 5/5: set facts, escalate to user: **CAMPAIGN COMPLETE**.

## Constraints & gotchas

- **No git commits.**
- **x21+0x50 challenge format**: 17 chars with leading SPACE (e.g. `" 0000000000000000"`). Don't forget the leading space.
- **SSO layout**: 17 ≤ 22 means inline (SSO). std::string layout is implementation-dependent — H-N6 probably documented the exact byte layout in its summary; check there.
- **Device-info x23 blob is 128 bytes constant** across all 5 challenges. Capture once, hardcode as `[u8; 128]` literal.
- **Sub-callees are also runtime addresses** (0x78c6... range). If miasm can't see them, use H-N6's BP captures (we have regs/memory at 4+5+4=13 BPs across 5 challenges = 65 snapshots) to either (a) lift them from their own disasm OR (b) treat as input-output oracles.
- **JIT'd / different module**: the producer is at 0x78c689528c which is NOT in the libnmsssa dump. Its bytes were captured by H-N6 in cert_producer_0x78c689528c_disasm.txt — that's the source of truth for the disasm.
- **DON'T try to lift the encoder** (0x20aad4..0x20b8e8) — H-N5 hard-gated on that. Stay scoped to the 244-insn producer.

## Relevant files / references

- H-N6 deliverables: `/home/sdancer/nmss-emu-encoder-internal-bp/analysis/` (especially H-N6_summary.md + cert_producer_*_disasm.txt + 15 captures)
- H-N4 encoder I/O: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- H-N3 ground_truth_v2: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/ground_truth_v2_2026-05-11.json`
- H-N5 static_inputs.rs: `/home/sdancer/nmss-emu-encoder-port/analysis/static_inputs.rs`
- Cycle-30 prologue lift framework (for miasm patterns): `/home/sdancer/nmss-emu-symexec-stage-drv/scripts/02_lift_prologue.py`
- Patched binary on remote: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_producer_port_progress_2026-05-11.jsonl`. Stages: `function_bounds_verified`, `device_info_extracted`, `lift_started`, `lift_complete`, `rust_port_drafted`, `port_passes_<N>_of_5`, `port_5_of_5_CAMPAIGN_COMPLETE`, `port_falsified_with_diagnosis`.

## Operating mode

In-process Agent (background). 8h budget. STOP on:
- (a) **5/5 match → CAMPAIGN COMPLETE.** Set facts, escalate to user.
- (b) **≥3/5 match** → set fact `cert_producer_ported_2026_05_11`, escalate with the remaining diagnosis.
- (c) **0/5 match** → dump intermediate-state divergence, propose H-N8 (more HW-BPs at the sub-callees' boundaries).
- (d) **Hard gate**: a sub-callee is NEON-heavy / unmodellable → write blockers, propose recursive HW-BP capture path.
