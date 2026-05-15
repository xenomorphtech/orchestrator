# onleave-trace-capture — empirical per-opcode I/O of obfuscated stage_drv

**You ARE allowed and expected to write code.** Frida JS, Python, Rust. The orchestrator-role memory does not apply to worker sub-agents.

## Why this path

Cycle-24 path `callback-body-port-v2` confirmed that `stage_drv @ 0xbe324` is **Themida/VMP/OLLVM-level CFG-flattened + MBA obfuscated** — manual reverse-engineering is intractable in worker budgets. Cycle-24 worker's recommended successor: **bypass the obfuscation by capturing per-opcode (input → output + carrier-state-delta) tuples empirically**, then build a Rust emulator that matches each `match x1 =>` branch against captured I/O independently. This is your path.

## Role & workdir
Extend the existing Frida `Interceptor.attach` hooks on `stage_drv_entry` with **`onLeave` callbacks** that capture: (1) return value `x0` ∈ {0,1,2}, (2) a 1 KB snapshot of the carrier struct (pointer at `sp+88` in stage_drv's frame at entry), and (3) any output registers stage_drv writes. Run on **all 5 ground-truth challenges** via repeated native-replay-rs invocations. Build a per-opcode `(x1, x6:x7, carrier_in) → (x0, carrier_out)` table. Workdir: `/home/sdancer/nmss-emu-onleave-trace/`.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` (metric 0.70 → up to 0.90 if you produce a clean per-opcode I/O table).
- **Sub-goal:** Per-opcode emulation tables that bypass the static obfuscation barrier.

## Success criteria

- **Minimum**: Per-opcode I/O table at `analysis/stage_drv_io_table.jsonl` covering at least 3 of the 5 ground-truth challenges, with deterministic outputs for repeated (x1, x6:x7, carrier_in) tuples.
- **Stretch**: Rust emulator at `cert_port_v3/src/lib.rs` implementing each of the ~7 stage_drv x1 opcodes via table lookup or fitted function; passes ≥1 of the 5 `cargo test cert_vector_*` ground-truth tests.
- Set fact `cert_stage_drv_io_table_2026_05_11` to the table path when minimum hit.

## Key constraints from prior cycles (do NOT rediscover)

- **Capture context**: live-game (Frida) — the cert callback fires at boot via Function C dispatch (no manual stimulus needed if anti-tamper recipe applied; see below).
- **Anti-tamper recipe** (fact `frida_antidetect_recipe_2026_05_11`): spawn-mode (`frida -f`), root_hide.js, patch `libnmsssa+0x3c6ca0 = 3` BEFORE attaching cert hooks.
- **The 5 ground-truth challenges are NOT what the live device emits** (the live device gets server-random challenges per session — fact `cert_per_stage_trace_5x_2026_05_11`). For ground-truth I/O capture you **MUST** drive the cert through `native-replay-rs` on the ARM remote (`162.244.80.97`), not the live device. native-replay-rs writes the challenge to `chal_addr = synth_base + 0x2000` then replays. You can attach Frida or ptrace to the native-replay-rs process while it executes the cert call.
- **Production trick**: `native-replay-rs` doesn't run inside the game — it's a standalone replay binary on the ARM remote. You can either: (a) hook the binary directly via Frida (probably easier — no anti-tamper on a standalone test binary), OR (b) instrument it via ptrace from a co-process. Frida is recommended for fast iteration.

## Prior worker artifacts to reuse

- **Prior Frida script** (onEnter-only, 25 KB): `/home/sdancer/nmss-emu-callback-itrace/scripts/itrace_capture.js`. Adapt by adding `onLeave` callbacks. Don't rewrite from scratch.
- **Prior runner**: `/home/sdancer/nmss-emu-callback-itrace/scripts/run_itrace.py`.
- **Disasms**: `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/funcC_disasm.txt` (Function C, 232 insns) and `stage_drv_disasm.txt` (4630+ lines). Look at stage_drv prologue (0xbe324..0xbe9f0) to find where the carrier pointer is stashed on the stack — the cycle-24 worker said it lives at `sp+88` in stage_drv's frame.
- **5 prior per-session traces** (onEnter inputs only): `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/challenge_*_trace.jsonl`.
- **Opcode map** (fact `cert_stage_drv_opcode_map_2026_05_11`): 43-call sequence per cert; only call 0 (x1=0x3b) carries the challenge; calls 13-29 are 17× absorbs of ASCII `"c774b732"` (device fingerprint); calls 35-38 absorb APK install fragment + anti-cheat watchwords; rest are finalization.
- **Pinned addresses** (live-session base from cycle 10 dump): `stage_drv_entry = 0x6cc2371324`, `funcC = 0x6cc25ad8e4`, `module base = 0x6cc22b3000`. Note these change per session; resolve dynamically via the module signature scan from `itrace_capture.js`.

## Next 2-3 concrete tasks (ORDERED)

1. **Extend the Frida script with onLeave.** Copy `itrace_capture.js` to `scripts/onleave_capture.js`. For the `stage_drv_entry` hook, add an `onLeave(retval)` that:
   - Records `retval` (= x0 return = stage type result ∈ {0,1,2}).
   - Reads `this.context.sp + 88` to recover the carrier pointer (verify against disasm — if stage_drv's prologue stores carrier at different offset, adjust).
   - Captures `Memory.readByteArray(carrier_ptr, 1024)` BOTH at onEnter and onLeave so the delta is observable.
   - Writes `{call_index, x1, x6, x7, carrier_before_hex, x0_return, carrier_after_hex, ts}` to JSONL.
   - Use Frida's `state.callIndex++` to disambiguate the 43 calls per cert run.

2. **Drive captures via native-replay-rs on the ARM remote** for all 5 ground-truth challenges. For each challenge `c` ∈ `{0000000000000000, 0123456789ABCDEF, 1111111111111111, 7BDA93D2F45D36C0, AABBCCDDEEFF0011}`: SSH to `root@162.244.80.97`, run `frida -f /root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs --argv "<snapshot_dir>" --challenge "$c" -l onleave_capture.js`. Pull each session's JSONL back to `/home/sdancer/orchestrator/analysis/onleave_2026-05-11/challenge_<c>_io.jsonl`.

3. **Build the per-opcode I/O table + Rust emulator.** Aggregate the 5 × 43 = 215 (input, output) tuples grouped by `x1` opcode. For each opcode value present in the table (~7 distinct per fact `cert_stage_drv_themida_obfuscation_2026_05_11`), characterize the function — pure-lookup if outputs are unique per input, parameterized-function if patterns emerge. Implement in Rust at `cert_port_v3/src/lib.rs`. Run `cargo test cert_vector_<challenge>` against the 5 ground-truth pairs; assert ≥1 match.

## Constraints & gotchas

- **No git commits.**
- **Write code freely.**
- **native-replay-rs invocation**: it's a standalone test binary; `frida -f` works against it without anti-tamper. The full Frida anti-detect recipe is overkill here — just attach and hook.
- **Determinism check**: same `(x1, x6:x7, carrier_in)` MUST always produce same `(x0, carrier_out)` if stage_drv is pure. If not deterministic across multiple invocations of the same challenge, the function reads hidden global state — escalate (and identify what hidden state).
- **carrier_ptr offset**: cycle-24 said `sp+88`. Re-verify by reading stage_drv's prologue in `stage_drv_disasm.txt`. The first few `stp` instructions reveal what's stored at which offset.
- **Don't get distracted by the 2 long traces** (86 calls) — those are concatenated sessions. Use 43-call sessions only.
- **Mutation worker may still be running on remote** (`/tmp/sm_mut_2026_05_11/`) — don't disturb its files; use a different work dir like `/tmp/onleave_2026_05_11/`.

## Relevant files / references

- Snapshot dir on remote: `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/`
- native-replay-rs binary: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`
- Frida binary: `frida` CLI on remote (or host); host has `frida-tools 14.5.2 / frida 17.7.2`, remote has `frida-server-17.9.1`.
- Prior itrace artifacts: `/home/sdancer/nmss-emu-callback-itrace/`
- Prior v2 port artifacts (stage_drv disasm + opcode map): `/home/sdancer/nmss-emu-callback-body-port-v2/analysis/`
- Ground truth: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- Harness DB facts: `cert_stage_drv_themida_obfuscation_2026_05_11`, `cert_stage_drv_opcode_map_2026_05_11`, `cert_function_c_dispatcher_only_2026_05_11`, `cert_challenge_injection_via_cli_flag_2026_05_11`.

## Progress log
Append to `/home/sdancer/orchestrator/analysis/checkpoints/onleave_trace_capture_progress_2026-05-11.jsonl`.

## Operating mode
In-process Agent (background). 4-hour budget. STOP and report if: (a) `sp+88` doesn't contain a valid carrier pointer (re-read disasm to find the right offset), (b) stage_drv outputs are non-deterministic across same-input invocations (hidden state — escalate, name what's hidden), or (c) you complete the table but a fitted Rust emulator gets 0/5 (means we're missing a hidden output — likely a side-channel write to memory that doesn't appear in the carrier struct or registers; escalate with diagnostic).
