# stage-drv-body — extract stage_drv body (4300 insns) as `body(w1, x0_buf, prologue_state) → (ret, x0_buf_delta)`

**You ARE allowed and expected to write code.** Python (miasm), Rust if emitting an emulator.

## Role & workdir

Continue from `symbolic-exec-stage-drv` (path completed cycle 30 — see falsified.md entry). The prologue is fully lifted. Now characterize the **body** at `0xbea20..end_of_function` (~4300 insns) as a function from `(w1, x0_buf_state, prologue_stack_state) → (ret_value, x0_buf_writes)`. Workdir: `/home/sdancer/nmss-emu-stage-drv-body/`.

## Why this path

Cycle-30 symbolic-exec result confirmed:
- Prologue (447 insns) lifted cleanly via miasm — **no Themida**, pure MBA. 54 magic constants + 30 concrete regs reach body entry.
- **stage_drv return value is deterministic per w1** across all 86 captured calls (15 distinct w1 values observed; same w1 always returns same ret).
- Body's "input surface" beyond magic constants = 17 symbolic stack slots (callee-saves, TLS canary, X0 spill, 1 X0-conditional slot at SP-164).
- The transformation lives in: (a) the body's side-effects on `x0` buffer (carrier struct), and (b) how those side effects compose across 41 stage_drv invocations per cert.

Recovering the body unlocks the algorithmic cert transformation end-to-end.

## Goal / sub-goal

- **Goal:** `nmss_cert_transformation_recovered` (metric 0.82 → up to 0.95 if body table or executable model recovered).
- **Sub-goal:** Concrete representation of body — table-form OR Rust emulator — that, given `(w1, x0_buf_in)`, predicts `(ret, x0_buf_out)` matching observed itraces for ≥3/5 challenges.

## Success criteria

- **Minimum (path A — itrace-driven table)**: A table `body_table.jsonl` keyed by `(w1, x0_buf_in_hash)` → `(ret, x0_buf_out)`, fit from itraces. Cross-validate by predicting ret/x0_out for held-out itrace events with ≥80% accuracy. Set fact `stage_drv_body_table_2026_05_11` with absolute path.
- **Stretch (path B — symbolic Rust emulator)**: `fn stage_drv_body(w1: u32, x0_buf: &mut [u8], prologue_state: StageDrvFrame) → u32` that matches ≥3/5 ground-truth cert vectors when composed with the 41-call sequence + the existing format pipeline.
- **Hard gate**: if neither path produces a quantifiable model within 6h, write `analysis/body_extraction_blockers.md` describing exactly what's missing (e.g. "x0_buf state at call k+1 doesn't follow from observed call k — hidden global state present").

## Inputs you have

- **stage_drv disasm**: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/stage_drv_disasm.txt` (4630 lines, base VA `0x6cc2371324`, module-rel `0xbe324`). Prologue 0..446 (already lifted); body starts at insn 447 (module-rel `0xbea20`).
- **Round-1 lift artifacts**: `/home/sdancer/nmss-emu-symexec-stage-drv/analysis/`:
  - `stage_drv_prologue_table.jsonl` — 128-row per-w1 frame (54 stack cells + 30 concrete regs + 17 symbolic slots).
  - `stage_drv_prologue.rs` — Rust dispatch from the prologue lift (7559 lines, 5 tests pass).
  - `stage_drv_dispatch_table.json` / `stage_drv_per_w1_dispatch.json` — slot-by-slot classification.
- **5 per-challenge itraces** at `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/challenge_*_trace.jsonl`. Each event includes:
  - `regs` — all GPRs at the hook PC.
  - `x0_w64` / `x1_w64` / `x2_w64` / `x3_w64` / `x4_w64` / `x8_w64` — 64-byte windows at pointer regs (null when reg is small int).
  - `x19_w128`, `x22_w128`, `x23_w128`, `x24_w128` — 128-byte windows.
  - `x20_w256`, `sp_w256`, `fp_w128` — wider snapshots.
  - 86 stage_drv invocations total across the 5 traces.
- **Module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin` (5.1 MB, base `0x6cc22b3000`). Use for raw bytes the disasm doesn't cover.
- **15 distinct observed w1 values** in itraces: 0x07, 0x0d, 0x11, 0x12, 0x13, 0x16, 0x1c, 0x1d, 0x1f, 0x27, 0x2d, 0x2e, 0x38, 0x3a, 0x3b. (Prologue lift covers 0x00..0x7f.)
- **Observed deterministic ret-per-w1** map from cycle 30: `{0x07:0, 0x0d:1, 0x11:2, 0x12:9, 0x13:0xf, 0x16:0, 0x1c:1, 0x1d:0x23, 0x1f:0, 0x27:0, 0x2d:0, 0x2e:0, 0x38:2, 0x3a:2, 0x3b:1}`.

## Next 3 ordered tasks

1. **Inventory itrace stage_drv events**. Find all events where `target_pc` (for `bl_hit`) or `pc` (for `bl_entry` / `bl_leave`) maps to stage_drv's entry `0xbe324` in module-relative terms. Note: target_pc in itraces is absolute, and the base may differ per session (anti-detect rebases the module each spawn). Use the session-base from the trace's first event + offset `0xbe324` to identify stage_drv calls. Write `analysis/stage_drv_events.jsonl` with `{session, call_idx, w1, x0_buf_in, x0_buf_out, ret}` per event.

2. **Path-A first (cheap)**: build the table `body_table.jsonl` and cross-validate. If x0_buf_in collisions across challenges produce **identical** x0_buf_out → x0 is the full state and the body is a pure function. If colliding x0_buf_in produces DIFFERENT x0_buf_out → hidden state exists (TLS canary, global var, hidden side input). Either way, document the finding in `analysis/path_a_findings.md`.

3. **Path B if path-A is insufficient**: symbolic-lift the body per w1, seeded with the prologue's body-entry state. Reuse `/home/sdancer/nmss-emu-symexec-stage-drv/scripts/02_lift_prologue.py` patterns. Body is 4300 insns vs prologue's 447 — expect heavier paths; cap depth at 5000 with miasm's depth limiter; if state-explosion: cut the body at the next `bl` or `br` boundary and treat downstream as a callee.

## Constraints & gotchas

- **No git commits.**
- **Don't re-do the prologue lift** — it's done. Treat its output as ground truth.
- **Per-session module base differs** — the itrace's PCs are absolute, but module-rel = pc - session_base. The session_base is itrace-file-specific (look at the first event's PC and subtract the disasm's expected module-rel for that event's symbolic label, OR use the existing base captured in `/home/sdancer/nmss-emu-callback-itrace/analysis/` if present).
- **x0_w64 only captures 64 bytes at \*x0** — the carrier struct may be larger. If body reads/writes beyond +64 from x0, the itraces undercapture. Note this in path_a_findings.md if you detect it.
- **w1 is in X1 register**, NOT W1 (miasm gotcha — round-1 worker confirmed: must concretize X1).
- **15 observed w1 values may not be all values reachable** — the body might handle more under different challenges; if path-A's coverage is incomplete, note which w1 values are missing.

## Relevant files / references

- Prologue lift artifacts: `/home/sdancer/nmss-emu-symexec-stage-drv/analysis/`
- stage_drv disasm: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/stage_drv_disasm.txt`
- Itraces: `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/`
- Module dump: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`
- Prior path's falsified.md entry: `/home/sdancer/orchestrator/analysis/falsified.md` (symbolic-exec-stage-drv consumed-success)

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/stage_drv_body_progress_2026-05-11.jsonl`. Stages: `events_inventoried`, `path_a_table_built`, `path_a_validation_done`, `path_b_lift_started`, `path_b_validation_done`, `blockers_documented`.

## Operating mode

In-process Agent (background). 6h budget. STOP and report on:
- (a) path A succeeds: table built + validation ≥80% + fact set.
- (b) path B succeeds: Rust emulator + ≥3/5 cert match.
- (c) hard gate: write blockers doc, exit with structural diagnosis.
