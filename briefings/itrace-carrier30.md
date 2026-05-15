# itrace-carrier30 — augmented Frida capture to dereference `*(x0+0x30)` at every stage_drv entry/leave

**You ARE allowed and expected to write code.** Python (Frida driver), JS (Frida hook).

## Role & workdir

Re-run the cycle-20 callback-instrumented-trace Frida pipeline, but augmented to **dereference the heap pointer at `*(x0 + 0x30)`** and dump a 1024-byte window there at every stage_drv `diag_hit` and `diag_leave`. Workdir: `/home/sdancer/nmss-emu-itrace-carrier30/`.

## Why this path

Cycle-33 `stage-drv-body` Path A (worker a1007f04900ae4ff5) succeeded at 100% within its capture surface AND surfaced a major structural finding:

- `x0_buf[:64]` (the carrier struct header) is **INVARIANT across the body** for all 210 observed call-pairs.
- The body's cert-contributing state mutations flow through **`*(carrier + 0x30)`** — a heap pointer (observed value `0xb400006e84c961d0` in challenge 2B6419320D30CDAE) that no register at stage_drv entry/leave holds.
- Without that capture, a Rust body emulator can correctly predict ret + visible window but CANNOT reproduce the cert.

This path expands the capture window to include `*(x0+0x30)+offset` so the body's actual state evolution becomes observable.

## Goal / sub-goal

- **Goal:** `nmss_cert_transformation_recovered` (metric 0.86 → up to 0.95 if augmented capture enables full body table).
- **Sub-goal:** 5 fresh per-challenge traces with `*(x0+0x30)` dumps at stage_drv entry+leave, fed into the existing body_table pipeline to enable a full `body(w1, carrier_inner_state_in) → (ret, carrier_inner_state_out)`.

## Success criteria

- **Minimum**: 5 successful traces saved as `analysis/itrace_carrier30_2026-05-11/challenge_*_trace.jsonl`. Each trace has `carrier30_w1024_in` and `carrier30_w1024_out` fields populated for ≥30/43 stage_drv calls.
- **Stretch**: Re-run the body_table pipeline (scripts/02 from stage-drv-body worktree) on the augmented traces. Show that for the SAME (w1, x0_buf_in[:64]) key, the carrier30 buffer DOES vary between in/out — proving the body's effect is now observable. Then report whether the table predicts (carrier30_out | w1, x0_buf_in, carrier30_in) deterministically.
- Set fact `cert_carrier30_captured_2026_05_11` with the trace dir path when minimum met.

## Inputs you have

- **Cycle-14 callback-instrumented-trace Frida pipeline**: `/home/sdancer/nmss-emu-callback-itrace/scripts/`. The harness ran successfully against the live game (PID 6237) producing the 5 cycle-20 itraces.
- **Cycle-20 itrace files**: `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/challenge_*_trace.jsonl` — the SHAPE these augmented traces should match (same event types, same regs structure), but with `carrier30_w1024_in` and `carrier30_w1024_out` added per stage_drv event.
- **Cycle-33 body_table pipeline scripts**: `/home/sdancer/nmss-emu-stage-drv-body/scripts/01..04_*.py` — the validation harness you'll re-run on augmented data.
- **Frida server on device**: cycle-14 confirms frida-server-17.9.1 + frida-tools 17.7.2 work with spawn-mode anti-detect (patch `libnmsssa+0x3c6ca0 = 3`). The cycle-14 progress jsonl at `analysis/checkpoints/callback_itrace_progress_2026-05-11.jsonl` documents the exact spawn recipe.
- **stage_drv address resolution**: stage_drv module-rel offset = `0xbe324`. Per-session base differs (anti-detect rebases on each spawn). Use the cycle-14 base-resolution path: `Module.findBaseAddress("libnmsssa.so")` returns the spawn's actual base, then add `0xbe324`.
- **Anti-Frida memory rule**: `NO Frida on libUnreal.so` — only libnmsssa is safe to hook. The stage_drv entry is in libnmsssa.

## Next 3 ordered tasks

1. **Copy + augment the hook script**. Fork `/home/sdancer/nmss-emu-callback-itrace/scripts/<hook>.js` to `scripts/itrace_carrier30.js`. At every `Interceptor.attach` for stage_drv (`onEnter` aka `diag_hit` and `onLeave` aka `diag_leave`):
   ```js
   var x0 = ctx.x0;
   var inner_ptr = Memory.readPointer(x0.add(0x30));
   var inner_buf = Memory.readByteArray(inner_ptr, 1024);
   send({event: 'carrier30', when: 'enter|leave', buf_hex: hexlify(inner_buf), ...existing_fields});
   ```
   Guard against null/unreadable inner_ptr (set buf_hex to null and record error).

2. **Run the spawn-mode harness 5×**. Re-launch the game with frida spawn + cycle-14 anti-detect patch; let the cert path execute; collect the JSONL. Save to `analysis/itrace_carrier30_2026-05-11/challenge_<sessionhex>_trace.jsonl`. The session hex is server-randomized — capture whatever 5 the server gives you. Document the actual challenges in `analysis/captured_challenges.txt`.

3. **Re-run body_table pipeline**. Copy `/home/sdancer/nmss-emu-stage-drv-body/scripts/02_build_body_table.py` to local `scripts/`, modify the key from `(w1, x0_buf_in)` to `(w1, x0_buf_in, carrier30_in_hash)` and the value from `(ret, x0_buf_out)` to `(ret, x0_buf_out, carrier30_out)`. Run validation. Report variance: does carrier30_out differ from carrier30_in for non-zero-effect w1 calls?

## Constraints & gotchas

- **No git commits.**
- **NO Frida on libUnreal.so** — anti-cheat will trip. Only hook libnmsssa offsets. stage_drv lives in libnmsssa.
- **Inner_ptr may be NULL or unmapped** on early-stage calls — handle gracefully so the hook doesn't crash the whole trace. Log a warning event per failed dereference.
- **1024-byte window is a guess** — if the buffer access pattern (from disasm of the body) shows reads/writes beyond +1024, widen to 4096. Don't go past 64 KiB without checking what kind of allocation `0xb400006e84c961d0` represents (the `b400` prefix is Android tagged-pointer / MTE).
- **DON'T re-attempt onLeave hooks against native-replay-rs** — that's the cycle-25/28 falsified path. This path runs against the LIVE GAME, which cycle-14 confirmed works.
- **5 successful traces is target**; if 3 work and 2 crash from anti-cheat retaliation, that's still enough for path-A v2 validation.

## Relevant files / references

- Cycle-14 hook: `/home/sdancer/nmss-emu-callback-itrace/scripts/`
- Cycle-20 itrace shape: `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/challenge_*_trace.jsonl`
- Body table pipeline: `/home/sdancer/nmss-emu-stage-drv-body/scripts/01..04_*.py`
- Cycle-33 findings: `/home/sdancer/nmss-emu-stage-drv-body/analysis/path_a_findings.md`
- Frida anti-detect recipe: cycle-14 progress jsonl `/home/sdancer/orchestrator/analysis/checkpoints/callback_itrace_progress_2026-05-11.jsonl`
- libUnreal Frida rule: feedback memory `feedback_no_frida.md` (no Frida on libUnreal.so)

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/itrace_carrier30_progress_2026-05-11.jsonl`. Stages: `hook_augmented`, `spawn_attempt_<n>`, `trace_captured_<sessionhex>`, `5x_done`, `table_rerun_done`.

## Operating mode

In-process Agent (background). 4h budget. STOP and report on:
- (a) 5/5 traces with carrier30 dumps + table rerun shows body effect now visible → path SUCCESS, set fact, escalate to next path (carrier30-table-emulator).
- (b) ≥3/5 traces but table shows residual non-determinism even with carrier30 captured → write `analysis/residual_blockers.md` listing what's still missing (e.g., reads from other addresses, hidden TLS).
- (c) anti-cheat detects Frida + blocks all 5 captures → write `analysis/anticheat_block.md`, escalate so the orchestrator decides whether to pivot to Path B (symbolic lift) or substrate alternatives.
