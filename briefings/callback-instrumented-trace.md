# callback-instrumented-trace — capture per-bl register state per challenge

**You ARE allowed and expected to write code.** The orchestrator-role memory ("never write code directly") applies to the orchestrator instance only, NOT to worker sub-agents.

## Role & workdir
Frida-instrument every `bl` boundary inside the live `nmssCoreGetCertValue` callback body during real cert calls, log register state (x0–x30, sp, pstate, plus pointed-to data for x0/x1/x20) per stage **per challenge**. Diff across challenges to localize where per-challenge entropy enters and propagates. Workdir: `/home/sdancer/nmss-emu-callback-itrace/`.

## Why this is the path
Cycle 13 falsified the entire static-analysis line:
- `callback-body-port` produced a structurally-correct Rust port (48-hex shape, deterministic `raw32_after_13`) but **0/5** ground-truth matches across 11 variants × 1599 attempts. Model produces a *constant* `2A1103…` for every challenge — no per-challenge entropy is consumed in the model.
- 5 ground-truth byte sequences are **not** in any captured dump (snapshot or live).
- WIKI.md line 13: every prior "successful" replay used `--ctx-seed-240-text` to inject the answer as a bypass; **the algorithm has never been algorithmically derived from static artifacts**.

The only remaining productive substrate is **live observation during cert execution**. callback-frida-capture (cycle 10) proved the technique works (it captured the callback fn-ptr + 5.1 MB module dump + anti-detect-bypass recipe). Your job extends it: instead of one hook at the dispatch site, place hooks at every `bl` boundary inside the callback body and dump register state.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` (metric 0.20 → up to ~0.80 if you produce a clean per-stage trace).
- **Sub-goal:** for each of the 5 ground-truth challenges (or as many as can be elicited in-game), produce a JSONL trace of `(stage_pc, x0–x30, sp, x0_window, x1_window, x20_window)` at every `bl` entry inside `nmssCoreGetCertValue`. Then diff across the 5 traces — the stages where x-register values DIVERGE are the per-challenge entropy ingestion sites.

## Success criteria
- Per-challenge trace files at `analysis/itrace_2026-05-11/<challenge>_trace.jsonl`.
- A diff summary `analysis/itrace_2026-05-11/divergent_stages.md` naming the `bl` PCs where register state diverges across challenges (and which registers).
- Set fact `cert_per_stage_trace_5x_2026_05_11` to the analysis dir.
- **Stretch**: if a stage's input is clearly the challenge bytes (e.g. an x0 window matches the challenge ASCII), name that as the algorithm's entropy entry point.

## Existing reusable assets

- **Anti-detect recipe** (fact `frida_antidetect_recipe_2026_05_11`): spawn mode (`frida -f`), on-device `root_hide.js`, pre-attach patch of `libnmsssa+0x3c6ca0 = 3`. Reuse — the callback-frida-capture worker's scripts are at `/home/sdancer/nmss-emu-callback-frida/scripts/callback_capture_spawn.js` and `run_capture_spawn.py`. Adapt them.
- **Callback body bounds**: entry at module-rel `0x17ded0`, first ret at module-rel `0x17ef1c` (1044 insns / 4176 B main + landing pad). Full disasm at `/home/sdancer/nmss-emu-callback-body-port/analysis/callback_body_full_disasm_2026-05-11.txt`.
- **The bl targets to hook** (from callback-body-port cycle 11 task1): in main body, 27 distinct `bl` PCs. **You don't need to hook all 27 — focus on the work-delegating ones**: priority-5 list from prior worker `0x6cc24bdad4, 0x6cc2492850, 0x6cc24abee0, 0x6cc23ce104, 0x6cc24b0bb8`. Also hook `0x6cc2371324` (stage-driver per fact `cert_callback_identity_confirmed_2026_05_11`) and `0x6cc2374080` (logger — useful for stage-tag selectors). All addresses are live-session VAs from PID 6237; convert to module-rel offsets by subtracting the live module base, then re-add the live base of YOUR session's deleted-module shard at runtime.
- **Module locator**: the deleted-module shard at runtime is the rwxp mmap with a 5.1 MB span, 3 r-xp segments, and the typename literal `24NMSS_KIT_CERT_VALUE_INFO` at offset `0x4392c0`. Random filename per session — find it by signature (size + segment count), not by name. callback-frida-capture's script has this discovery logic.
- **Wrapper port (downstream target)**: `/home/sdancer/nmss-emu-callback-body-port/nmsscore_port_callback/src/lib.rs`. Has a `ProducerCallback` trait seam. Once you have the divergence map, the next path can implement the actual algorithm there.

## Next 2-3 concrete tasks (ORDERED)

1. **Set up the multi-hook Frida script.** Start from `/home/sdancer/nmss-emu-callback-frida/scripts/callback_capture_spawn.js`. For each priority `bl` target, install an `Interceptor.attach` with `onEnter(args)` that dumps `(timestamp, pid, stage_label, pc, x0..x30, sp, lr, NativePointer(x0).readByteArray(64), NativePointer(x1).readByteArray(64), NativePointer(x20).readByteArray(128))` to JSONL. Resolve PCs at runtime: `module_base + (priority_pc - live_session_base_from_cycle_10)`. Save script at `scripts/itrace_capture.js`.

2. **Run against 1 challenge first** to validate. The game runs cert calls during NMSS init naturally (callback-frida-capture proved this — no manual stimulus needed; install hooks BEFORE the cert path executes, i.e. use spawn mode). Capture a trace, sanity-check it's structured correctly, then add per-challenge variation. Note: you may need to drive the game to issue MULTIPLE cert calls (some idle game state may cache the cert) — investigate. If only one cert per game session is observable, plan to relaunch the game 5 times.

3. **Capture all 5 challenges + diff.** For each of the 5 ground-truth challenges (`0000000000000000`, `0123456789ABCDEF`, `1111111111111111`, `7BDA93D2F45D36C0`, `AABBCCDDEEFF0011`), drive a cert call and capture the trace. Write a diff script `scripts/diff_traces.py` that, for each stage_label, lists which register values diverge across the 5 challenges. Emit `analysis/itrace_2026-05-11/divergent_stages.md` with the findings.

## Constraints & gotchas

- **DO NOT hook libUnreal.so.** Anticheat blacklist; documented. libnmsssa.so + the deleted-module callback are safe.
- **Frida 17 API**: `Module.findBaseAddress` removed → use `Process.findModuleByName().base`. `Memory.readByteArray` removed → use `NativePointer.readByteArray`. No global `setTimeout`. (Documented in fact `frida_antidetect_recipe_2026_05_11`.)
- **The challenge bytes**: each challenge is a 16-character hex-encoded ASCII string (e.g. `7BDA93D2F45D36C0`). You should see these bytes (or their 8-byte hex-decoded form) appearing in one of the x-register windows of at least one early-stage `bl`. If you NEVER see them, that's a major finding (means even the live algorithm reads the challenge from somewhere indirect).
- **Per-challenge invocation**: you may need to vary an in-game action to trigger different cert challenges. The challenges may come from the server — investigate whether the game cycles through them or fixes one per session.
- **Async-signal-safety doesn't apply** (Frida hooks run in user code, not signal handlers).
- **No git commits.**

## Relevant files / references

- Anti-detect Frida scripts (adapt): `/home/sdancer/nmss-emu-callback-frida/scripts/{callback_capture_spawn.js, run_capture_spawn.py}`
- Callback body disasm: `/home/sdancer/nmss-emu-callback-body-port/analysis/callback_body_full_disasm_2026-05-11.txt`
- Sub-call disasms: `/home/sdancer/nmss-emu-callback-body-port/analysis/subcalls/`
- Full 5.1 MB module dump (read-only ref): `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`
- Ground truth: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- Wrapper port (downstream): `/home/sdancer/nmss-emu-callback-body-port/nmsscore_port_callback/src/lib.rs`
- Harness DB facts: `cert_callback_identity_confirmed_2026_05_11`, `frida_antidetect_recipe_2026_05_11`, `cert_callback_structure_mapped_2026_05_11`, `FALSIFICATION_offline_algorithm_from_static_2026_05_11`.

## Progress log
Append checkpoints to `/home/sdancer/orchestrator/analysis/checkpoints/callback_itrace_progress_2026-05-11.jsonl`.

## Operating mode
In-process Agent (background). Iterate tasks 1→2→3. **You may stop and report partial results** at any point — a clean trace of even ONE challenge is valuable (proves the technique). Don't stop because of perceived orchestrator constraints; the worker MAY write code. STOP only if: (a) Frida attach is rejected by anticheat even with the recipe, (b) per-bl hooks crash the game (too many breakpoints), or (c) all 5 challenges produce IDENTICAL register state at every bl (means challenge bytes never enter the cert path through this function — major finding, escalate).
