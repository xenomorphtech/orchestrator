# native-replay-orch-port (H-N2) — port the cert-critical native-replay-rs function to Rust

**You ARE allowed and expected to write code.** Python (miasm/disasm), Rust (the actual port).

## Role & workdir

Port the 2806-instruction cert-critical function at module-rel `0x17ded0..0x180aa8` (the **native-replay-rs cert orchestrator**) to a standalone Rust function. Verify by running native-replay-rs against captured I/O — the function bounds and substrate match the verification target, so this port is **directly verifiable** unlike all prior porting attempts. Workdir: `/home/sdancer/nmss-emu-native-replay-orch-port/`.

## Why this path

Cycle-47 `native-replay-callsite-classifier` (H-N1) delivered:
- **Cert-critical function bounds**: `0x17ded0..0x180aa8`, 2806 insns / 11,224 bytes.
- **Function entry 0x17ded0 EXACTLY matches cycle-41 cert-callback fptr** (`0x78cd413ed0 = 0x78cd296000 + 0x17ded0`).
- **Same function holds 3 of 6 stage_drv calls** — calls 0 (w1=0x15), 1 (w1=0x16), 5 (w1=0xe challenge-carrier).
- **Challenge ingestion location identified**: 2 `bl 0x601d0` (memcpy) calls at `0x17ea54..0x17ea74` immediately precede the w1=0xe call, copying `x6=challenge[0..8]` and `x7=challenge[8..16]` into `carrier+0x228` (16B) and stack+0x240.

This function IS native-replay-rs's cert callback — porting it to Rust + composing with helpers A and B should reproduce 5/5 ground-truth certs. Crucially, verification is **substrate-matched**: native-replay-rs's HW-BP capture (cycle 41) provides the per-call I/O against which to validate the port. No live-game / Frida dependency.

## Goal / sub-goal

- **Goal:** `nmss_cert_transformation_recovered` (metric 0.94 → 0.98+ if port matches ≥3/5 ground truth) AND `nmss_cert_pure_rust` (metric 0 → 3+ if port reproduces certs).
- **Sub-goal:** A Rust function `fn cert_orchestrator_at_17ded0(challenge: [u8; 8], prologue_state: StageDrvFrame) → [u8; 24]` that matches ≥3/5 ground truth certs via composition with the cycle-30 stage_drv prologue lift + cycle-33 body table + helpers A (`0x18fe00..0x196780`) and B (`0x202c18..0x20b8ec`).

## Success criteria

- **Minimum**: Lift the 2806-insn function via miasm symbolic execution per-w1-call path (reuse cycle-30 framework). The 6 stage_drv calls within (3 in main fn + 3 in helpers A/B) are abstract opaque calls with the cycle-30 prologue + cycle-33 body table providing their semantics. Output: `cert_port_v3/src/lib.rs` with the Rust port + 5 unit tests against ground-truth.
- **Stretch**: ≥3/5 ground-truth certs match. Set fact `cert_orchestrator_ported_2026_05_11 = <rust crate path>`.
- **Hard gate**: If the function contains MBA-encoded loops or non-summarizable bl callees beyond the 6 stage_drv + 2 memcpy + ~5 known helpers, write `analysis/port_blockers.md` listing the specific obstacles.

## Inputs you have

- **Cycle-30 stage_drv prologue lift**: `/home/sdancer/nmss-emu-symexec-stage-drv/analysis/stage_drv_prologue.rs` (7559-line Rust, 5 unit tests pass, 128 w1 values lifted; per-w1 deterministic ret).
- **Cycle-33 body table**: `/home/sdancer/nmss-emu-stage-drv-body/analysis/body_table.jsonl` + `body_ret_by_w1.json` (per-w1 deterministic ret across 86 paired calls).
- **Cycle-41 HW-BP I/O table**: `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl` (30 rows × 5 challenges × 6 stage_drv calls; carrier_before, carrier_after, ret per call — the ground-truth I/O for validating the port).
- **Cycle-47 H-N1 deliverables**: `/home/sdancer/nmss-emu-native-replay-callsite-classifier/analysis/`:
  - `cert_critical_caller.json` — function bounds + w1=0xe site
  - `lr_to_w1_classification.jsonl` — all 6 LRs classified
  - `lr_function_bounds_v2.json` — bounds for the 3 distinct functions (main 0x17ded0, helper A 0x18fe00, helper B 0x202c18)
  - `SUMMARY.md` — full writeup
- **Module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin` (5.1 MB, base `0x6cc22b3000`).
- **Stage_drv disasm**: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/stage_drv_disasm.txt` (the stage_drv function itself — already lifted; you treat it as opaque per the body table).
- **Patched native-replay-rs binary** on remote with `--trace-call-hw`: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`. Run with various challenges to get more I/O ground truth.
- **5 ground-truth challenge→cert pairs**: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`.

## Next 3 ordered tasks

1. **Extract + disasm the cert-orchestrator function bytes**. Extract module-rel `0x17ded0..0x180aa8` from the module dump (11,224 bytes) to `analysis/cert_orch_0x17ded0.bin`. Disasm with capstone aarch64 to `analysis/cert_orch_0x17ded0_disasm.txt`. Identify: stp prologue, callee-saved registers, frame size, the 3 bl-to-stage_drv sites, the 2 memcpy sites, all other bl targets (categorize: known helper / unknown).

2. **Lift via miasm symbolic execution**. Reuse cycle-30 `scripts/02_lift_prologue.py` pattern: concretize entry state (challenge as symbolic byte[16]), symbolic-execute the 2806 insns. At each `bl 0xbe324`: substitute with the cycle-30 prologue + cycle-33 body model (`body(w1, x0_buf) → ret + carrier-mutations-known-from-body-table`). At each `bl 0x601d0`: substitute with memcpy semantics. At unknown bls: stop and report. Emit Rust port as `cert_port_v3/src/lib.rs` with `fn cert_orchestrator(challenge: [u8; 16]) → [u8; 24]`.

3. **Validate against 5 ground truths**. Write `cert_port_v3/tests/ground_truth.rs` that runs the port for each of the 5 test challenges from `summary.json` and compares against expected 24-byte certs. Report passing rate. If <3/5, dump per-call intermediate state (carrier+0x228 at every stage_drv call) and diff against the patched native-replay-rs's HW-BP captures for the same challenge to localize the divergence.

## Constraints & gotchas

- **No git commits.**
- **2806 insns is over the briefing's 1500 gate** but tractable. Cycle-30 lifted 447 insns of prologue in <2 sec/w1. 2806 insns × ~25 ms/insn for full path = ~70 sec total. Heavy but doable.
- **3 functions to model** (main + helper A + helper B), but helpers A and B can ALSO be lifted via the same approach if their bodies are tractable. Or treat them as opaque-callable summaries from the HW-BP I/O.
- **Helpers A/B contain stage_drv calls at w1=0x1d, w1=0x1d, w1=0x1d (call 2/3/4)**. The cycle-33 body table for w1=0x1d says ret=0x23, 484 bytes mutated. Use this as the helper-call summary.
- **Challenge enters via x6/x7**, NOT via x0 register at function entry — x6/x7 are set by the call site's argument shuffling. The function entry takes (x0 = something else, x6 = chal[0..8], x7 = chal[8..16]). Confirm by reading the immediate caller in the parent function.
- **No live device, no Frida** — pure offline symbolic execution + Rust port. The 5 ground truths plus the patched binary on remote are the validation oracle.

## Relevant files / references

- H-N1 deliverables: `/home/sdancer/nmss-emu-native-replay-callsite-classifier/analysis/`
- Cycle-30 prologue lift framework: `/home/sdancer/nmss-emu-symexec-stage-drv/`
- Cycle-33 body table: `/home/sdancer/nmss-emu-stage-drv-body/analysis/`
- Cycle-41 HW-BP I/O table: `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl`
- Ground truths: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- Module dump: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`
- Patched native-replay-rs binary on remote (--trace-call-hw): `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/native_replay_orch_port_progress_2026-05-11.jsonl`. Stages: `disasm_extracted`, `lift_started`, `lift_complete`, `rust_port_drafted`, `port_passes_<n>_of_5`, `port_falsified_with_localized_divergence`.

## Operating mode

In-process Agent (background). 8h budget. STOP on:
- (a) ≥3/5 ground-truth match → set fact, escalate metrics 0.94 → 0.98 and pure_rust 0 → 3+.
- (b) Lift completes but 0/5 match → dump intermediate-state divergence localization, write `analysis/port_diagnosis.md`.
- (c) Hard gate: function contains insurmountable obstacles (e.g., unmodeled bl callees, MBA loops) → write `analysis/port_blockers.md` and propose H-N3 (HW-BP state snapshot) as fallback.
