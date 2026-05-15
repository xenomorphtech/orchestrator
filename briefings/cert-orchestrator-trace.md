# cert-orchestrator-trace — find the real cert orchestrator function, hook it, lift the accumulator logic

**You ARE allowed and expected to write code.** Python (Frida driver), JS (Frida hook), Rust if porting accumulator.

## Role & workdir

Cycle-43 `function-c-trace` localized the per-iteration cert accumulator to specific stack offsets in an orchestrator function that is **NOT** Function C. The dominant bl-to-stage_drv callsite is at module-rel `0x233328`. Your job: find the orchestrator function's entry point (scan backwards from 0x233328 for the closest preceding `stp x29, x30, [sp, #N]` prologue), hook entry/exit/all-bl-to-stage_drv, dump frame at each hook, then identify and port the accumulator logic. Workdir: `/home/sdancer/nmss-emu-cert-orchestrator-trace/`.

## Why this path

Cycle-43 found:
- Function C (`0x17f8e4`) is NOT on the call stack when stage_drv fires — only 2/5 runs, never holding stage_drv calls.
- Dominant orchestrator callsite is at `0x233328` (19/43 stage_drv calls per cert), with secondary callers at `0xc1ba4` (5/43), `0x1aa2b8` (3/43), `0x173000`/`0x1713ac` (2/43 each), plus ~10 one-shots.
- The **orchestrator's stack frame (sp_w512)** at every bl-to-stage_drv contains an observable accumulator. Hot byte-flip offsets: `+0x08..0x0b, +0x50..0x5b, +0x68..0x6b, +0x80..0x84, +0x98..0x99, +0xa0..0xa8, +0xb0..0xb3`. Bytes vary BOTH per-iteration AND per-challenge — consistent with cert state carrying challenge entropy.

This path closes the loop: find the orchestrator function bounds, instrument it, then port its accumulator logic to native-replay-rs.

## Goal / sub-goal

- **Goal**: `nmss_cert_transformation_recovered` (metric 0.92 → up to 0.98 if accumulator logic ported and matches ≥3/5 ground-truth certs).
- **Sub-goal**: A Rust function `fn cert_orchestrator(challenge: [u8; 8], prologue_state: StageDrvFrame) → [u8; 24]` that reproduces ≥3/5 ground-truth certs by composing the 41 stage_drv ret values with the localized accumulator stored at orchestrator-frame hot offsets.

## Success criteria

- **Minimum**: Identify the orchestrator function entry (`stp` prologue) and bounds. Hook entry/exit/all-bl-to-stage_drv. Capture 5 traces. Set fact `cert_orchestrator_function_localized_2026_05_11` with module-rel entry/exit addrs.
- **Stretch**: Per-offset write-site analysis — for each hot accumulator offset (+0x08, +0x50, etc.), identify the insn(s) in the orchestrator that write to that offset (use Frida Stalker OR static disasm of the function bounds). Build a Rust port of the accumulator logic. Set fact `cert_orchestrator_accumulator_ported_2026_05_11`.
- **Hard gate**: If the orchestrator function isn't found at any preceding `stp` (e.g., 0x233328 is inside a function with non-standard prologue, or the function is HUGE >2000 insns), write `analysis/orchestrator_localization_blocker.md` and recommend ptrace-style memory tracking as fallback (H1 from cycle-33 backlog).

## Inputs you have

- **Cycle-43 deliverables**: `/home/sdancer/nmss-emu-function-c-trace/analysis/`:
  - `funcC_bl_stage_drv_sites.json` — inventory of bl-to-stage_drv callsites (use this as your address list).
  - `funcC_accumulator_offsets.json` — per-trace + cross-challenge byte-flip frequency matrix at orchestrator's sp+offsets.
  - `funcC_stateless_blocker.md` — full writeup including next-step recommendation.
  - 5 traces at `analysis/funcC_state_2026-05-11/challenge_*_trace.jsonl` — already include sp_w512 dumps at every stage_drv bl.
- **Cycle-43 hook framework**: `/home/sdancer/nmss-emu-function-c-trace/scripts/funcC_state_trace.js` — reuse this as the base; just retarget Function C → the orchestrator function found in task 1.
- **Stage_drv disasm + symexec lift** (cycle 30): `/home/sdancer/nmss-emu-symexec-stage-drv/analysis/stage_drv_prologue.rs` (7559 lines, deterministic per-w1 ret).
- **Stage_drv body analysis** (cycle 33): `/home/sdancer/nmss-emu-stage-drv-body/analysis/body_table.jsonl` + body_ret_by_w1.json (deterministic ret map).
- **HW-BP capture infra** (cycle 41, post-retire): `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl` + patched native-replay-rs binary on remote with `--trace-call-hw` flag. Native-replay-rs uses **6 stage_drv calls** (not 43). Substrate mismatch: native-replay-rs's path is the minimal cert path; live-game's 43-call path includes other crypto invocations.
- **Module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin` (5.1 MB, base `0x6cc22b3000`).
- **No Frida on libUnreal.so** (anti-cheat); orchestrator function is in libnmsssa — safe.

## Next 3 ordered tasks

1. **Find the orchestrator function**. Scan the module dump backwards from file offset `0x233328` for the closest preceding `stp x29, x30, [sp, #-N]!` (prologue pattern, instruction bytes start with `0xa9bf|0xa9b{eight-bit-spec}` and end with the imm9 stack-frame-size). Identify both the function entry (the `stp`) and end (first `ret` after it). Save bounds to `analysis/orchestrator_function_bounds.json` with `{entry: 0x..., end: 0x..., size_insns: N, bl_count: M, bl_to_stage_drv_count: K}`. If size_insns > 2000 or bl-to-stage_drv-count != ~19, escalate via the hard gate.

2. **Fork funcC_state_trace.js → orchestrator_trace.js**. Retarget Function C's entry hook to the orchestrator function's entry. Hook every bl-to-stage_drv site (use cycle-43's site list as the seed but add any new ones discovered in task 1). Add hooks at task-1's "end" (the `ret`) to capture exit state. Dump sp_w512 + x29_w128 + x0_w128 (the carrier ptr) + x19-x28 (callee-saved) at each hook. Run 5x against the live game using the cycle-43 spawn-mode recipe. Save to `analysis/orchestrator_state_2026-05-11/challenge_*_trace.jsonl`.

3. **Per-offset write-site analysis**. For each hot accumulator offset from cycle-43 (`+0x08, +0x50, +0x68, +0x80, +0x98, +0xa0, +0xb0`), disassemble the orchestrator function and find every `str` / `stp` / `stur` insn whose target is `sp+offset`. Build a write-site → offset table. If only 1-2 insns write each hot offset, that's the accumulator core; port them to Rust. Save `analysis/accumulator_write_sites.json` + `analysis/accumulator_port.rs` (the Rust port). Run against the 5 captured traces' input states to verify the port reproduces the observed accumulator evolution.

## Constraints & gotchas

- **No git commits.**
- **NO Frida on libUnreal.so** — orchestrator is in libnmsssa, you're safe.
- **The orchestrator likely has 100-500 insns** based on Function C being 232 insns and the orchestrator handling 19 bl-to-stage_drv + bookkeeping. If it's >2000, you may be in the wrong function — re-scan from the bl callsite itself, treating it as inside a hot loop rather than a flat sequence.
- **Multiple orchestrator candidates**: cycle-43 found 5 distinct caller addresses for stage_drv (`0x233328, 0xc1ba4, 0x1aa2b8, 0x173000, 0x1713ac`) — some may be the SAME orchestrator function called from different basic blocks (e.g., a loop), or DIFFERENT helper functions all called by some upper orchestrator. The function bounds from task 1 will reveal which.
- **Substrate mismatch**: native-replay-rs uses 6 stage_drv calls; live game uses 43. If the orchestrator at `0x233328` doesn't appear in native-replay-rs's call trace, then **the live-game's orchestrator function may not be what native-replay-rs is calling**. Cross-reference with native-replay-rs's HW-BP traces (cycle-41 deliverable) to determine if `0x233328` is reached during native-replay-rs's 6-call execution. Use the patched binary with `--trace-call-hw` to confirm.

## Relevant files / references

- Cycle-43 deliverables: `/home/sdancer/nmss-emu-function-c-trace/`
- Cycle-43 hook: `/home/sdancer/nmss-emu-function-c-trace/scripts/funcC_state_trace.js`
- Cycle-41 HW-BP table: `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl` + `stage_drv_io_table_summary.md`
- Patched native-replay-rs (--trace-call-hw): `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`
- Cycle-30 prologue lift: `/home/sdancer/nmss-emu-symexec-stage-drv/analysis/stage_drv_prologue.rs`
- Cycle-33 body table: `/home/sdancer/nmss-emu-stage-drv-body/analysis/body_table.jsonl`
- Module dump: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_orchestrator_trace_progress_2026-05-11.jsonl`. Stages: `function_bounds_localized`, `hook_written`, `5x_captured`, `write_sites_identified`, `rust_port_drafted`, `rust_port_verified_<n>_of_5`.

## Operating mode

In-process Agent (background). 6h budget. STOP on:
- (a) Rust port matches ≥3/5 ground-truth certs → set fact `cert_orchestrator_accumulator_ported_2026_05_11`, escalate as goal-mostly-met (metric 0.92 → 0.98).
- (b) Orchestrator function found and instrumented but write-sites are too many / dispersed to port in 6h → write partial port + structural diagnosis.
- (c) Orchestrator function not findable / function is gigantic → hard gate, recommend ptrace-style memory tracking (H1 backlog).
