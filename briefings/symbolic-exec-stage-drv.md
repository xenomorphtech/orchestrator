# symbolic-exec-stage-drv — round 2: validation-ceiling root-cause

**You ARE allowed and expected to write code.** Python (miasm), Rust if you generate emulator code.

## Role & workdir
Continue the symbolic-exec lift of stage_drv's prologue. Workdir: `/home/sdancer/nmss-emu-symexec-stage-drv/`. Round 1 (cycles 28–29) is COMPLETE; round 2 attacks the **validation ceiling**.

## Round 1 result — what's already done

Round-1 worker produced (in `analysis/`):
- `stage_drv_prologue_table.jsonl` — 128 rows (one per w1, all 0x11..0x3b plus extended).
- `stage_drv_dispatch_table.json`, `stage_drv_per_w1_dispatch.json`, `stage_drv_invariant_constants.json` — per-w1 dispatch + invariant decomposition.
- `stage_drv_prologue.rs` (73 KB) — Rust emission of the lifted table.
- `itrace_validation.json` + `itrace_precise_validation.json` — validation against 86 itrace calls.

Verified findings:
- 54 stack cells, ALL w1-varying (0 invariants). Each cell carries 2 distinct values across 128 w1 — binary dispatch per cell.
- 100% w1 coverage (no out-of-model w1 in itraces).
- Reg-match rate **38.46%** across 86 itrace calls. 10 regs (X1, X2, X4–X7, X10, X12, X15, X18) match 100% across all 86 calls. 16 regs (X0, X3, X9, X11, X14, X16–X17, X19–X28) disagree 100% across all 86 calls.

The 16 disagreeing regs are all candidates for caller-state pass-throughs (callee-saved X19–X28, input-pointer-derived X0/X3, scratch X9/X11). Static lift cannot predict them without per-call caller-state.

## Round-2 goal

Determine whether the 38% validation ceiling is:
- **(A) Caller-state-only ceiling** — the lift is structurally correct, and the 16 disagreeing regs are by-design symbolic (pass-through caller state). Proof: re-validate with each itrace's actual caller-state fed in as the lifter's initial X0..X28 state. If reg-match rises to ≥80%, the lift is complete and this path's work is done — set fact `stage_drv_prologue_lift_complete_2026_05_11` and STOP.
- **(B) Lift bug** — even with caller-state fed in, reg-match stays below 80%. Then diagnose: which insn class is mis-lifted? Output diagnostic to `analysis/lift_bug_diagnosis.md`.

## Concrete tasks (ORDERED)

1. **Inventory itrace caller-state**. Each `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/challenge_*_trace.jsonl` records X0..X30 at every stage_drv entry. Build a per-call dict: `{call_idx: {X0:..., X1:..., ..., X30:...}}`. There are 86 stage_drv-entry events across the 5 challenge files.

2. **Round-2 re-lift**. Modify `scripts/02_lift_prologue.py` (or write `scripts/10_relift_callstate.py`) so the symex initial state is seeded from the per-call caller-state, not generic symbols. For each of the 86 calls: load X0..X28 from the trace, concretize w1=X1, run the 447-insn prologue, dump body-entry state.

3. **Re-validate**. Reuse `scripts/07_precise_validate.py` logic. Target: per-call match-rate ≥80% on regs X0..X28. Write `analysis/itrace_validation_round2.json`.

4. **Branch on the result**:
   - If ≥80%: write `analysis/lift_complete_evidence.md` summarizing the round-1 + round-2 evidence chain; set fact `stage_drv_prologue_lift_complete_2026_05_11` with the artifact paths; STOP.
   - If <80%: write `analysis/lift_bug_diagnosis.md` — per-register breakdown of round-2 disagreements, which insns immediately precede each disagreeing reg in the disasm, and a one-paragraph hypothesis for the bug.

## What's already on disk (DON'T REDO)

- `analysis/stage_drv_prologue_447insn.bin` (1788 B prologue slice).
- `analysis/stage_drv_prologue_table.jsonl` (128 w1 lifted with generic-symbolic caller state).
- `analysis/stage_drv_dispatch_table.json` (52 KB summary).
- `analysis/stage_drv_invariant_constants.json` — 0 invariants confirmed.
- `analysis/stage_drv_per_w1_dispatch.json` — 36 KB.
- `analysis/itrace_validation.json` + `analysis/itrace_precise_validation.json`.
- `analysis/stage_drv_prologue.rs` — Rust emission.
- `scripts/01_extract_slice.py` … `scripts/09_emit_rust.py` — reuse these.

## Constraints & gotchas

- **No git commits.**
- **Re-use the existing miasm IR machinery** in scripts/02. Don't rewrite from scratch.
- **Concrete caller-state matters** — when seeding from the itrace, also seed `@64[TPIDR_EL0+0x28]` if recorded (stack canary), otherwise leave symbolic.
- **The X19–X28 callee-saved regs are stored at SP-{16..96}** at prologue entry (see `stack_symbolic` in the table). The prologue *restores* them at exit, so if your lift stops at prologue end the regs reflect the saved values — which IS the caller state. If reg-match still fails after seeding, the lift may be running past the prologue boundary (insn 447 is wrong — try shorter).
- **Don't try to lift the body** of stage_drv. Body is for the next path.

## Success criteria

- **Path A (lift complete)**: round-2 reg-match ≥80% across 86 calls; fact set; `analysis/lift_complete_evidence.md` written.
- **Path B (lift buggy)**: `analysis/lift_bug_diagnosis.md` with per-reg disagreement breakdown + bug hypothesis.

Either outcome is a successful path completion — A unblocks the body-extraction path; B redirects effort. Stalling = inconclusive after 4h of work.

## Progress log
Append to `/home/sdancer/orchestrator/analysis/checkpoints/symexec_stage_drv_progress_2026-05-11.jsonl`. Use stage names `relift_start`, `relift_w1_done`, `relift_validate_done`, `evidence_or_diagnosis_written`.

## Operating mode
In-process Agent (background). 4h budget. STOP and report on path A or B completion, or escalate if you hit a third-category outcome (e.g. itraces don't actually record full X0..X28).
