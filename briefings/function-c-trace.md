# function-c-trace — instrument Function C's frame across the 41-call stage_drv iteration

**You ARE allowed and expected to write code.** Python (Frida driver), JS (Frida hook).

## Role & workdir

Frida-instrument **Function C at module-rel `0x17f8e4`** (absolute `0x6cc24328e4` per cycle-20 base) — the caller orchestrating the 41 stage_drv invocations — to capture its **frame state, locals, registers, and reachable heap pointers** at:
- Function C entry
- Each `bl` to stage_drv (callsite + return point)
- Function C exit (when it returns the cert)

Workdir: `/home/sdancer/nmss-emu-function-c-trace/`.

## Why this path

Cycle-38 `itrace-carrier30` (worker a560ab7f71dee8daf) **falsified** the carrier30 hypothesis and produced a MAJOR REFRAME:

- Body of stage_drv mutates **ZERO bytes** in any captured window across 185 paired calls (carrier+0x30 = 0 mutations, carrier+0x00 = 0, *(x0)[:128] = 0).
- `*(carrier+0x30)` is a **singleton pointer** `0xb400006e84c961d0` constant across all 6 game spawns despite ASLR — it's not per-alloc; it's a static address inside the deleted-module shard.
- w1 sequence is **nearly identical** across all 5 sessions; first 29 calls execute the IDENTICAL w1 prefix. Per-w1 ret is fully deterministic.
- **Body is effectively STATELESS w.r.t. all observable state reachable from x0**.

Conclusion: the cert accumulator **cannot live in stage_drv body**. It must live in **Function C's frame** (the caller). Function C calls stage_drv 41 times sequentially; the per-call ret value (and possibly other side channels — e.g. Function C's stack-locals written by stage_drv via the carrier+0x30 singleton) gets accumulated across iterations by code IN Function C, not by stage_drv body.

## Goal / sub-goal

- **Goal:** `nmss_cert_transformation_recovered` (metric 0.88 → up to 0.95 if Function C's accumulation logic is observable).
- **Sub-goal:** A per-iteration trace of Function C's stack, locals, and reachable heap that captures how it composes the 41 stage_drv calls into the final cert output.

## Success criteria

- **Minimum**: Per-challenge trace at `analysis/funcC_state_2026-05-11/challenge_<hex>_trace.jsonl` capturing Function C's regs (X19-X28 callee-saved + SP-relative stack), 256 bytes of stack memory at sp..sp+256, and 256 bytes at the suspected accumulator regions on EACH of the 41 stage_drv bl callsites within Function C. At least 5 challenges × ≥30 captures per challenge.
- **Stretch**: Identify which bytes in Function C's frame are mutated between consecutive stage_drv calls. If a single stack offset evolves monotonically/structurally, that's the accumulator. Report it.
- **Hard gate**: if Function C's frame ALSO shows zero mutations between calls (mirroring the body), then the cert state must live in TLS / global / another module — write `analysis/funcC_stateless_blocker.md` and recommend the next attack surface.

## Inputs you have

- **Function C disasm**: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/funcC_disasm.txt` (1075 lines, base VA `0x6cc24328e4` = module-rel `0x17f8e4`). 232 instructions per cycle-20 worker's analysis.
- **Cycle-14 itrace pipeline**: `/home/sdancer/nmss-emu-callback-itrace/scripts/` and its successor `/home/sdancer/nmss-emu-itrace-carrier30/scripts/itrace_carrier30.js` (Frida-17 API-correct, includes spawn-mode anti-detect, handles deref safety). USE itrace_carrier30.js AS YOUR STARTING POINT — it has the working hook framework.
- **Cycle-38 itrace-carrier30 deliverables**: `/home/sdancer/nmss-emu-itrace-carrier30/analysis/itrace_carrier30_2026-05-11/` — 5 GOOD traces with stage_drv-entry-side data. You can correlate Function C's frame state at its bl-to-stage_drv with the existing stage_drv entry traces.
- **Cycle-33 + cycle-38 findings combined**: stage_drv signature `stage_drv(carrier_ptr, length, ?, x5_aux, x6:x7_data) → u8`. Function C bl-sites to stage_drv: there are 41 such calls within Function C's 232 insns (per cycle-20 worker observation). Find these `bl` sites in funcC_disasm.txt — each one is a hook target.
- **Anti-Frida memory rule**: `NO Frida on libUnreal.so` — only libnmsssa. Function C is in libnmsssa.
- **Frida spawn-mode anti-detect recipe**: patch libnmsssa+0x3c6ca0 = 3, per cycle-14 progress jsonl.

## Next 3 ordered tasks

1. **Inventory Function C's bl-to-stage_drv sites**. Parse `funcC_disasm.txt` for every `bl` whose target resolves (via the disassembler's annotation OR module-relative resolution) to stage_drv at `0x17f8e4 → 0xbe324` (i.e. delta `0xbe324 - 0x17f8e4 = -0x6f5c0`... actually compute it from the absolute addresses in the disasm). Save the list of callsite offsets to `analysis/funcC_bl_stage_drv_sites.json`. Expect ~41 entries based on cycle-20's "41 iterations per cert" finding.

2. **Fork itrace_carrier30.js → funcC_state_trace.js**. Add `Interceptor.attach` hooks at:
   - Function C entry (`0x6cc24328e4`): dump regs, sp, 256B at sp.
   - Each bl-to-stage_drv callsite (from task 1): dump regs (especially X19-X28), sp, 256B at sp, 256B at `*(x0 + 0x30)` (the singleton). Note this is the SAME pointer across all calls, but its contents may be reading something we missed; this is your double-check on cycle-38's "zero mutations" claim.
   - Function C return (the last `ret` insn): dump x0 (return value), sp, 256B at sp.
   Capture into a trace JSONL with events `funcC_enter`, `funcC_pre_stage_drv_<n>`, `funcC_post_stage_drv_<n>`, `funcC_exit`.

3. **Run 5x against live game**. Use the same spawn-mode + anti-detect recipe from itrace_carrier30. Save 5 traces. Then run a `03_diff_funcC_frame.py` script: across consecutive `funcC_pre_stage_drv_<n>` and `funcC_pre_stage_drv_<n+1>` events within a single trace, byte-diff the 256B stack window. Any byte that flips is part of the per-iteration accumulator. Across challenges, the same offsets should flip but the VALUES should differ (challenge entropy). Report which offsets in `analysis/funcC_accumulator_offsets.json`.

## Constraints & gotchas

- **No git commits.**
- **NO Frida on libUnreal.so** — Function C is in libnmsssa, you're safe.
- **Function C is 232 insns** per cycle-20 worker. If you find ~41 bl-to-stage_drv sites, that confirms the structure. If you find fewer (e.g. 3-5 bl's in a loop), then the iteration is a LOOP, not unrolled — hook the same bl site 41 times.
- **Hook overhead**: 41 hook fires × 256B-stack-dump × 5 challenges = lots of data. JSONL files may be 5-10 MB each. That's fine.
- **DON'T** assume the accumulator is on stack — also check x19-x28 (callee-saved regs) and any heap pointer Function C holds.
- **Stage_drv body confirmed stateless w.r.t. carrier**: but stage_drv may write to a TLS slot or to Function C's stack via x0_carrier's other offsets. Capture broadly first, narrow after.

## Relevant files / references

- Function C disasm: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/funcC_disasm.txt`
- itrace_carrier30 working hook: `/home/sdancer/nmss-emu-itrace-carrier30/scripts/itrace_carrier30.js`
- run_itrace driver: `/home/sdancer/nmss-emu-itrace-carrier30/scripts/run_itrace_carrier30.py`
- cycle-38 reframe findings: `/home/sdancer/nmss-emu-itrace-carrier30/analysis/path_a_v2_findings.md` and `residual_blockers.md`
- Stage_drv = w1 → ret deterministic table: `/home/sdancer/nmss-emu-stage-drv-body/analysis/body_ret_by_w1.json`
- Cycle-20 base resolution: callsite_pc/target_pc in itrace JSONLs is absolute; subtract session base for module-rel.

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/funcC_trace_progress_2026-05-11.jsonl`. Stages: `bl_sites_inventoried`, `hook_written`, `spawn_attempt_<n>`, `trace_captured_<sessionhex>`, `5x_done`, `frame_diff_done`, `accumulator_localized`.

## Operating mode

In-process Agent (background). 4h budget. STOP and report on:
- (a) Function C frame shows clear per-iteration accumulator offset(s) → set fact `funcC_accumulator_localized_2026_05_11`, escalate to next path (port Function C's accumulator logic).
- (b) Function C frame is ALSO stateless across iterations → write `analysis/funcC_stateless_blocker.md` recommending TLS-trace, global-data-snapshot, or "the cert composes from `(challenge, w1_seq, ret_seq)` alone — test H4 stateless emulator".
- (c) anti-cheat blocks 5/5 captures → preserve hook, write anticheat_block.md, escalate.
