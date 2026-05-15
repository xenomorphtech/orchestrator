# native-replay-callsite-classifier (H-N1) — identify which native-replay-rs LR is the challenge-carrying caller

**You ARE allowed and expected to write code.** Python (static disasm, capstone), shell.

## Role & workdir

Pure static analysis over the deleted-module dump. Among native-replay-rs's 6 LR values for stage_drv calls — `{0x17dfe8, 0x17dffc, 0x17ea78, 0x190508, 0x1943ac, 0x202dcc}` — identify **which one corresponds to the 6th call (`w1=0xe`, the challenge-carrying call per cycle-41 HW-BP finding)**. That LR's enclosing function IS native-replay-rs's cert-critical orchestrator and is the target for the follow-on porting path. Workdir: `/home/sdancer/nmss-emu-native-replay-callsite-classifier/`.

## Why this path

Cycle-45 cert-orchestrator-trace cross-ref proved: native-replay-rs's 6 stage_drv calls return into `[0x17dfe8..0x202dcc]` — **none** into the live-game orchestrator at `0x233234`. So the live-game orchestrator is not on native-replay-rs's cert path. The actual cert-producing function is the parent of one of the 6 LR values. Cycle-41's HW-BP capture established that the 6 calls are `[0x15, 0x16, 0x1d, 0x1d, 0x1d, 0xe]` in order, and the **6th call (w1=0xe) carries the challenge** via x6:x7. So that 6th call's caller is the entry point to native-replay-rs's actual algorithmic cert function.

This path is purely static — no Frida, no device, no live game. The deleted-module dump at `0x6cc22b3000` base + module-rel offsets is all the input needed.

## Goal / sub-goal

- **Goal:** `nmss_cert_transformation_recovered` (metric 0.92 → +0.06 if H-N1 succeeds; unlocks H-N2 worth +0.10).
- **Sub-goal:** Identify cycle-41's 6th-call LR among the 6 candidates; localize the enclosing function bounds; cross-reference with cycle-41's I/O table.

## Success criteria

- **Minimum**: For each of the 6 LR values, identify the immediately preceding `bl 0xbe324` (stage_drv) instruction (the LR is `bl_pc + 4`), and decode its surrounding context (preceding ~50 insns) to identify w1 value loaded into X1. Build `analysis/lr_to_w1_classification.jsonl` keyed by LR, with fields `{lr, bl_pc, w1_value_static, w1_value_method_static_or_indirect, enclosing_function_bounds_guess}`.
- **Stretch**: Cross-reference with cycle-41's `stage_drv_io_table.jsonl` to MATCH each LR to its position in the per-cert call sequence `[0x15, 0x16, 0x1d, 0x1d, 0x1d, 0xe]`. Specifically identify which LR → 6th call (w1=0xe). Save `analysis/cert_critical_caller.json` with `{lr_hex, bl_pc_hex, function_entry_guess, function_end_guess, w1_assignment}`. Set fact `cert_critical_native_replay_caller_2026_05_11` = function_entry_guess.
- **Hard gate**: If all 6 LR sit inside the same function, OR the 6th-call site can't be uniquely identified (indirect bl, computed w1 not statically determinable), write `analysis/static_unresolvable.md` listing the obstacles. Hand off to H-N3 (HW-BP state snapshot) which can dynamically observe what static can't.

## Inputs you have

- **Module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin` (5.1 MB, base `0x6cc22b3000`). File-offsets = module-relative offsets; for LR `0x17dfe8`, look at file-offset `0x17dfe4` (LR - 4) for the `bl` instruction.
- **Cycle-41 HW-BP table**: `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl` — 30 rows × 5 challenges × 6 stage_drv calls. Each row has `(challenge, call_idx 0..5, w1, ret, x0_before, x0_after)`. Use call_idx to map LR → opcode position.
- **stage_drv entry**: `0xbe324` (module-rel). All 6 LRs are `bl_pc + 4` where `bl_pc` decodes to a `bl 0xbe324` immediate-target instruction.
- **Cycle-43 stage_drv disasm**: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/stage_drv_disasm.txt` (full 4630-line disasm including prologue + body, lifted with capstone or similar tool).
- **Native-replay-rs source**: `/home/sdancer/nmss-emu/native-replay-rs/src/main.rs` (constants: SYNTH_SIZE=0x40000, CHAL_OFFSET=0x2000, OUT_OFFSET=0x3000). Confirms native-replay-rs uses the deleted-module shard via mmap+br.
- **Cycle-45 substrate cross-ref**: `/home/sdancer/nmss-emu-cert-orchestrator-trace/analysis/native_replay_substrate_crossref.json` — the 6 LR values are listed there.

## Next 3 ordered tasks

1. **Decode the 6 bl-to-stage_drv sites**. For each LR in `{0x17dfe8, 0x17dffc, 0x17ea78, 0x190508, 0x1943ac, 0x202dcc}`, read bytes at file-offset (LR-4) from the module dump. Verify it decodes to a `bl <offset>` where offset resolves to `0xbe324`. Save `analysis/bl_sites_decoded.json`.

2. **Static dataflow for w1**. For each bl-pc, scan backward up to 100 insns to find the last `mov w1, #imm` or `mov x1, #imm` or `ldr w1, [..]` or `add w1, ...` that determines w1 entering stage_drv. Classify each LR as `static_w1=N` (concrete value), `indirect_w1` (from register/memory at runtime), or `loop_w1` (varies per loop iteration). Save `analysis/w1_per_lr.json`.

3. **Match to cycle-41 sequence**. cycle-41 says the per-cert order is `[0x15, 0x16, 0x1d, 0x1d, 0x1d, 0xe]`. If each LR's static-w1 matches one of these uniquely (e.g. LR-A has w1=0x15, LR-B has w1=0x16, three LRs have w1=0x1d, LR-C has w1=0xe), match them. The LR with w1=0xe is the **cert-critical caller**. For its enclosing function: scan backward from bl-pc for the closest `stp x29, x30, [sp, #N]` prologue; scan forward for the matching `ret`. Save `analysis/cert_critical_caller.json` with bounds.

## Constraints & gotchas

- **No git commits.**
- **Pure static** — no Frida, no live device, no native-replay-rs execution.
- **Cycle-41 has 6 calls but call order may vary per-session**: the I/O table has 30 rows = 5 challenges × 6 calls. Check all 5 challenges to confirm the `[0x15, 0x16, 0x1d, 0x1d, 0x1d, 0xe]` sequence is challenge-invariant (cycle-41 worker said it is).
- **3 LRs likely have w1=0x1d** — they may all be in a loop body or 3 distinct callsites. If 3 distinct: classifier confirms; if all same: only one LR, hit 3 times. The HW-BP table distinguishes return-PC uniqueness.
- **bl offset encoding**: aarch64 `bl` instruction is `0x94000000 | ((target-pc-pc) >> 2 & 0x3FFFFFF)`. Compute `target = pc + sign_extend(imm26 << 2, 28)`. Verify == 0xbe324.

## Relevant files / references

- Module dump: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`
- HW-BP I/O table: `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl`
- HW-BP summary: `/home/sdancer/orchestrator/analysis/stage_drv_io_table_summary.md`
- Cross-ref: `/home/sdancer/nmss-emu-cert-orchestrator-trace/analysis/native_replay_substrate_crossref.json`
- stage_drv disasm: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/stage_drv_disasm.txt`
- Patched native-replay-rs binary on remote: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` (has `--trace-call-hw`)

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/native_replay_callsite_classifier_2026-05-11.jsonl`. Stages: `bl_sites_decoded`, `w1_per_lr_resolved`, `lr_to_call_idx_matched`, `cert_critical_caller_localized`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) Cert-critical caller (w1=0xe LR) uniquely identified + function bounds localized → set fact, escalate to H-N2 (port the function).
- (b) Multiple candidates for w1=0xe site (e.g. 2 LRs both load w1=0xe) → list candidates, request HW-BP disambiguation (H-N3).
- (c) Hard gate: indirect w1 / unresolvable → write static_unresolvable.md, escalate.
