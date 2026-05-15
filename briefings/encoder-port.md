# encoder-port (H-N5, CAPSTONE) — port the encoder at module-rel 0x20aad4 to pure Rust

**You ARE allowed and expected to write code.** Python (miasm symbolic lift), Rust (the actual port). Substantial code.

## Role & workdir

Port the encoder function at module-rel `0x20aad4` (entry point of helper-B's middle) to standalone Rust. The function signature is:
```
encoder(x0: *const StaticStruct,    // 0x78cd780e48 — constant 64-byte blob
        x5: *const StaticHeap,      // 0xb4000079d51864c1 — constant 64-byte blob
        x6: u64,                    // challenge bytes [0..8] (ASCII, LE-packed)
        x7: u64,                    // challenge bytes [8..16]
        ... static constants in other regs ...
       ) → std::string (24 bytes ASCII hex = cert)
```
Workdir: `/home/sdancer/nmss-emu-encoder-port/`.

## Why this path

H-N4 (cycle 58) delivered the campaign endgame:
- The encoder is at module-rel `0x20aad4` (corrected from H-N2's wrong hypothesis).
- Challenge enters via x6:x7 (ASCII LE-packed). All other inputs are CONSTANT.
- Static struct at x0 + static heap at x5 are captured (constant 64-byte blobs each).
- 5/5 ground-truth (challenge, cert) pairs verified — encoder_io_2026-05-11.jsonl contains them.
- The cert orchestrator at 0x17ded0..0x180aa8 just wraps this encoder + std::string ABI.

With clean inputs + outputs + small candidate function, **this is the last function to port to reach `nmss_cert_pure_rust = 5/5`**.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (metric 0 → 5 if ≥3 challenges reproduce). Also tops out `nmss_cert_transformation_recovered` (0.99 → 1.0).
- **Sub-goal:** A Rust function `fn cert_encoder(challenge: [u8; 16]) → [u8; 24]` linked against the cycle-30 stage_drv prologue + cycle-33 body table, matching ≥3/5 ground-truth certs.

## Success criteria

- **Minimum**: Function bounds of 0x20aad4 localized (scan forward for ret); disasm extracted; miasm lift attempted. Save `analysis/encoder_function_bounds.json` + `analysis/encoder_disasm.txt`.
- **Stretch**: Rust port `cert_encoder/src/lib.rs` reproduces ≥3/5 ground-truth certs from H-N4's encoder_io_2026-05-11.jsonl. Set fact `cert_encoder_ported_2026_05_11`. Goal `nmss_cert_pure_rust` met at ≥3/5.
- **Full**: 5/5 ground-truth match. Campaign closes.
- **Hard gate**: Function is also NEON-heavy (like H-N2 hit) / too large / has dispatches into unmodellable callees → write `analysis/encoder_port_blockers.md` and propose tactical alternative (e.g. capture the encoder's INTERNAL state transitions via more HW-BPs).

## Inputs you have

- **H-N4 encoder I/O**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl` — 5 rows with full input regs + output cert per challenge. `encoder_io_diff.txt` summarizes which regs are constant vs challenge-derived.
- **Module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin` (5.1 MB, base 0x6cc22b3000).
- **Static struct content** at x0=0x78cd780e48: hex bytes from encoder_io_diff.txt under `x0_static_struct_w64_hex`. 64-byte constant blob.
- **Static heap content** at x5=0xb4000079d51864c1: hex bytes from encoder_io_diff.txt under `x5_static_heap_w64_hex`. 64-byte constant blob.
- **5 ground-truth (challenge, cert) pairs** in encoder_io_2026-05-11.jsonl. `expected_cert` field is the real ground truth (validated by native-replay-rs's replayed_cert match).
- **Cycle-30 stage_drv prologue lift**: `/home/sdancer/nmss-emu-symexec-stage-drv/analysis/stage_drv_prologue.rs` (7559-line Rust, deterministic per-w1 ret) — if the encoder calls stage_drv internally, use this.
- **Cycle-33 body table**: `/home/sdancer/nmss-emu-stage-drv-body/analysis/body_table.jsonl` + `body_ret_by_w1.json`.
- **H-N2 callee triage**: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/callee_triage.json` may mention what 0x20aad4 calls.
- **Patched native-replay-rs binary** on remote with `--trace-call-hw` for cross-checking intermediate state if needed.

## Next 3 ordered tasks

1. **Localize function bounds**. From module dump at file-offset `0x20aad4`, scan forward for the first `ret` insn AND backward for any preceding `stp x29, x30` prologue (in case 0x20aad4 is NOT the function entry but a middle-bl-target). Save `analysis/encoder_function_bounds.json` with `{entry: 0x..., end: 0x..., size_insns: N, bl_count: M, bl_to_stage_drv_count: K}`. If size > 1000 insns OR the function calls into NEON-heavy regions, that's a gate-(c) risk — note it.

2. **Symbolic lift via miasm**. Reuse cycle-30 framework. Concretize x0 → static_struct (64-byte blob), x5 → static_heap (64-byte blob), x6:x7 as symbolic (the challenge). Symbolic-execute the function. At each `bl 0xbe324` (stage_drv) call: substitute with cycle-30 prologue + cycle-33 body model. At each `bl 0x601d0` (memcpy): symbolic memmove. At each unknown bl: stop and report. Emit Rust port as `cert_encoder/src/lib.rs`.

3. **Validate against 5 ground truths**. Test the Rust port against each of the 5 (challenge, cert) pairs from encoder_io_2026-05-11.jsonl. Output pass-rate. If <3/5, dump per-step intermediate symbolic state and diff against the HW-BP captures (which give x0 buffer evolution per call). Iterate.

## Constraints & gotchas

- **No git commits.**
- **Static blobs must be byte-perfect**: x0's 64 bytes and x5's 64 bytes are CONSTANT across all runs. Bake them into the Rust port as `[u8; 64]` literals. Even one wrong byte will diverge the output.
- **PER-RUN-VARYING regs (x1, x8, x19, x24, x29, sp) are ASLR addresses** — NOT challenge-derived. Treat them as opaque pointers; the encoder shouldn't dereference them in challenge-dependent ways. If it does, the lift will reveal that.
- **NEON limit** (H-N2 found): if the encoder uses `mov v9.16b, v0.16b` or similar NEON insns that miasm's aarch64 backend doesn't model, isolate the affected sub-region and handle manually (likely AES-NI or SIMD memcpy/memcmp).
- **The encoder may call stage_drv internally** — H-N4's data shows the encoder is at module-rel 0x20aad4, which per H-N2's callee_triage holds calls 2/3/4 (w1=0x1d). Use cycle-30 prologue + cycle-33 body model for those.
- **DON'T try to lift the whole orchestrator 0x17ded0..0x180aa8** — H-N2 hard-gated on that. Stay scoped to 0x20aad4's function only.

## Relevant files / references

- H-N4 deliverables: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/`
- H-N2 disasm of orchestrator: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/cert_orch_0x17ded0_disasm.txt` (may contain 0x20aad4's body too)
- H-N3 ground_truth_v2: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/ground_truth_v2_2026-05-11.json`
- Cycle-30 prologue lift: `/home/sdancer/nmss-emu-symexec-stage-drv/analysis/stage_drv_prologue.rs`
- Cycle-33 body table: `/home/sdancer/nmss-emu-stage-drv-body/analysis/`
- Module dump: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`
- Patched native-replay-rs binary: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` (use `--trace-call-hw` at intermediate addresses if needed for diagnosis)

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/encoder_port_progress_2026-05-11.jsonl`. Stages: `function_bounds_localized`, `disasm_extracted`, `lift_started`, `lift_complete`, `rust_port_drafted`, `port_passes_<N>_of_5`, `port_falsified_with_diagnosis`.

## Operating mode

In-process Agent (background). 8h budget. STOP on:
- (a) ≥3/5 ground-truth match → set fact, escalate metric `nmss_cert_pure_rust` to N/5, `nmss_cert_transformation_recovered` to 1.0 IF 5/5. **CAMPAIGN COMPLETE if 5/5.**
- (b) Lift completes but 0/5 match → dump intermediate-state divergence localization (which call/insn is the first to diverge from HW-BP captures).
- (c) Hard gate: function too large / NEON-blocked / unmodelled callees → write blockers, propose tactical alternative (more HW-BPs internal to encoder).
