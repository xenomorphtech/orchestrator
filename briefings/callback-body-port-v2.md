# callback-body-port-v2 — Rust port of the REAL cert callback (Function C + stage_drv)

**You ARE allowed and expected to write code.** The orchestrator-role memory ("never write code directly") applies to the orchestrator instance only, NOT to worker sub-agents. Write Rust freely. Run `cargo test`. Verify against ground truth.

## Why v2

The cycle-13 `callback-body-port` worker ported **Function A at module-rel `0x17ded0`** (= `nmssCoreGetCertValue` per cycle-6 static disasm) and got **0/5** ground-truth matches. Cycle-19's live Frida instrumentation revealed that **Function A NEVER FIRES** in the live cert path — libnmsssa dispatches to **Function C at module-rel `0x17f8e4`** instead. Cycle-20 nailed down the per-challenge entropy entry point as **`stage_drv_entry @ module-rel 0xbe324`**, registers **x6:x7** carrying the 16-char ASCII-hex challenge **little-endian-packed**. Cycle-21 found the challenge injection mechanism in native-replay-rs source: `chal_addr = synth_base + 0x2000`, where `write_std_string_short(chal_addr, &challenge)` writes the challenge as a libc++ short-string.

This is v2 of callback-body-port with the corrected target.

## Role & workdir
Port the actual cert callback (Function C + stage_drv) to Rust, with correct challenge ingestion from `chal_addr`, verify against the 5 ground-truth pairs via `cargo test`. Workdir: `/home/sdancer/nmss-emu-callback-body-port-v2/`.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` (metric 0.70 → 1.0 if 5/5 matches) AND `nmss_cert_pure_rust` (0/5 → 5/5).
- **Sub-goal:** Rust function that, given a 16-char hex challenge, returns the matching 48-hex ground-truth cert algorithmically.

## Success criteria

- Rust function reproducing **at least 1 of 5** ground-truth pairs (success-minimum). Set fact `callback_body_port_v2_partial_<challenge>_2026_05_11`.
- **Stretch**: all 5/5 match. Set fact `nmss_cert_5_5_pure_rust_reproduced` (the campaign goal-success fact!).

## Load-bearing intelligence (synthesize, don't re-derive)

### From cycle-19 (callback-instrumented-trace, fact `cert_actual_producer_function_c_2026_05_11`)
- **Real callback**: Function C at module-rel **`0x17f8e4`** (corresponds to live VA = `module_base + 0x17f8e4`; in the cycle-10 capture's base, that's `0x6cc242e000 + 0x17f8e4 = 0x6cc25ad8e4`).
- **Function A** at `0x17ded0` is the function the cycle-6 static disasm called `nmssCoreGetCertValue`, but **it never fires in the live path** — ignore Function A's disasm at `/home/sdancer/nmss-emu-callback-body-port/analysis/callback_body_full_disasm_2026-05-11.txt` (that's the wrong function).
- Function C contains only **4 `bl`s**: 2× `crypto5 @ 0x24b0bb8` + 2× `crypto5_post @ 0x24bd210`. Plus the `bl` to `stage_drv_entry @ 0xbe324`.

### From cycle-20 (fact `cert_entropy_entry_stage_drv_2026_05_11`)
- **stage_drv_entry @ module-rel `0xbe324`** is the per-challenge entropy ingestion site.
- Signature: `stage_drv(carrier_ptr, length, ?, x5_aux, x6:x7_data_block) -> u8 ∈ {0,1,2}`.
- Called **41 times per cert call** — multi-call stateful absorb primitive that builds cert state in the carrier struct.
- **Challenge encoding**: x6:x7 carry the 16-char ASCII-hex challenge **LE-byte-packed**. Example: `x6 = 0x4344373846383341` decodes byte-reversed as ASCII `"A38F87DC"` (the first 8 chars of the live challenge). x7 carries the next 8 chars.

### From cycle-20 (fact `cert_stage_drv_inputs_detail_2026_05_11`)
- stage_drv inputs across 41 calls per cert:
  - **1 call** with x6:x7 = the challenge bytes (per-challenge variance)
  - **12+ calls** with constant `0xc774b732` in some register slot — **LCG multiplier / S-box index candidate**. Investigate.
  - Calls with base64 fragment `mpf3XJtmS-BlQ==/` (part of game's APK install path)
  - Calls with anti-cheat watchwords: `gamehack`, `gamespee`, `libaegis`
- **Per-challenge variance enters ONLY via x6:x7 in 1 of 41 calls.** The other 40 absorb fingerprint/anti-cheat state, which is constant across challenges.

### From cycle-21 mutation-bisection + native-replay-rs source peek (fact `cert_challenge_injection_via_cli_flag_2026_05_11`)
- **All 38 mutations of ASCII-stale-challenge-copies in the snapshot produced ZERO cert change** — those copies are log/debug artifacts, NOT read by the cert path.
- **The actual challenge write site** (from `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/src/main.rs:994-997`):
  ```rust
  const SYNTH_SIZE: usize = 0x40000;
  const CHAL_OFFSET: usize = 0x2000;
  let chal_addr = synth_base + CHAL_OFFSET;    // synth_base is per-replay
  write_std_string_short(chal_addr, &challenge)?;
  ```
- `write_std_string_short` is at `main.rs:2817`. It writes the challenge as a libc++ short-form `std::string` to `chal_addr`. Read its impl to know the exact layout.
- **The cert algorithm reads from `chal_addr`** (via some pointer chain) and ends up with the bytes in x6:x7 of stage_drv_entry.
- **mutation-bisection v2 is still running** (libnmsssa code corruption with NONZERO bytes); v1 had a flawed positive control. v2 result may land while you're working — check `remote:/tmp/sm_mut_2026_05_11/results.jsonl` if you need confirmation that mutations CAN change certs (positive control).

### Inputs you have on disk

- **Full 5.1 MB live deleted-module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`. Covers VAs `0x6cc22b3000..0x6cc279c000`. Function C is at `0x6cc25ad8e4`. stage_drv is at `0x6cc2371324` (= `0x6cc22b3000 + 0xbe324`).
- **Per-session traces** (5 captured live, with stage_drv inputs at each of the 41 calls): `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/challenge_<HEXVALUE>_trace.jsonl` and `divergent_stages_5x.md` summary. These show you exactly what every stage_drv call's inputs look like — use them to validate your Rust port stage-by-stage.
- **native-replay-rs source** (the working reproducer; read-only reference): `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/src/main.rs`. Key items: `CHAL_OFFSET=0x2000` (line 24), `chal_addr` setup (line 994), `write_std_string_short` impl (line 2817).
- **Wrapper port from cycle-6** (the ProducerCallback trait seam, still useful — it's the wrapper around Function C/A indirection): `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port/src/lib.rs`.
- **Previous (wrong-target) Rust port**: `/home/sdancer/nmss-emu-callback-body-port/nmsscore_port_callback/src/lib.rs`. Has the right shape (48-hex output) and reproduces `raw32_after_13` correctly — but the algorithm is wrong because it modeled Function A. May still be useful as a code-structure starting point.

## Next 2-3 concrete tasks (ORDERED)

1. **Disassemble Function C and stage_drv from the 5.1 MB live-module dump.** Function C at file-offset `0x17f8e4 - 0x0` (since module-rel offset == file offset for the dump's start at 0x6cc22b3000) is technically at `(0x6cc242e000 - 0x6cc22b3000) + 0x17f8e4 = 0x17b000 + 0x17f8e4 = 0x2fa8e4` in the dump. Actually wait — read `CAPTURE_METADATA.json` for the dump-to-VA mapping to be sure. Then `objdump -D -b binary -m aarch64 --adjust-vma=<VA> dump_slice.bin > analysis/funcC_disasm.txt`. Do the same for stage_drv at file-offset corresponding to module-rel `0xbe324`. Save to `analysis/stage_drv_disasm.txt`.

2. **Reverse + port stage_drv to Rust.** This is the cert primitive: `fn stage_drv(carrier: &mut Carrier, length: u32, q: u32, x5_aux: u64, x6_x7_data: u128) -> u8`. The carrier struct size is large (cycle-19's traces show field accesses at +52, +56, +528, +544, +788, etc.). Use the trace files to validate: for any of the 5 captured live sessions, compute stage_drv's output for each of the 41 inputs and check it matches the captured x0 (return value) in the trace. Use `0xc774b732` as an investigative anchor — it appears 12+ times in stage_drv inputs and is very likely a fixed round constant (LCG multiplier, S-box index, or similar).

3. **Wire up the full cert pipeline + verify.** Build a Rust function `fn compute_cert(challenge_hex: &str) -> String` that:
   - Initializes the carrier struct to its starting state (read from snapshot — find the initial state via Function C's prologue or via the first stage_drv call's input).
   - Walks the 41 stage_drv calls. For the 1 call that's per-challenge: split the 16-char ASCII hex into 2× 8-char halves, LE-byte-pack each into a u64, pass as x6:x7. The other 40 calls use constant inputs (challenge-independent — can be hardcoded from the trace).
   - Reads the final cert from the carrier struct (Function C's SRet path — see the trace for which carrier field holds the result).
   - Returns the 48-hex uppercase string.
   - **Test against `cargo test`** — assert 1+ of the 5 ground-truth pairs matches. Put your code in `cert_port_v2/` (a new crate in this worktree).

## Constraints & gotchas

- **No git commits.**
- **Worker MAY write code** — emphatically. Edit/Write/Bash freely.
- **`0xc774b732` is the highest-value investigation anchor** — if you discover it's an LCG multiplier or S-box index or a known crypto constant, that probably unlocks the entire algorithm.
- **Live device challenges are server-random**; the 5 ground-truth challenges (7BDA…, AABB…, 0000…, 1111…, 0123…) appear ONLY in snapshot test fixtures. Your validation must use native-replay-rs's `cargo test cert_vector_<challenge>` flow (which injects the challenge via `--challenge` to `chal_addr`), NOT live game inspection.
- **stage_drv has internal state** in the carrier struct that evolves across 41 calls. Don't model it as 41 independent calls — the call N+1's input depends on call N's output via the carrier.
- **Per-challenge variance enters via x6:x7 in 1 specific call**. The other 40 calls' inputs are challenge-INDEPENDENT (constant in trace) — you can hardcode them from the trace once, then vary only x6:x7 for the variable call.

## Relevant files / references

- Per-session traces (the most valuable artifact): `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/challenge_*_trace.jsonl`
- Divergent-stage summary: `/home/sdancer/orchestrator/analysis/itrace_2026-05-11/divergent_stages_5x.md`
- 5.1 MB live module dump: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`
- Capture metadata: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/CAPTURE_METADATA.json`
- native-replay-rs source (challenge injection): `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/src/main.rs` lines 23-28 (constants), 994-997 (chal_addr), 2817 (write_std_string_short)
- Previous-attempt Rust port: `/home/sdancer/nmss-emu-callback-body-port/nmsscore_port_callback/src/lib.rs`
- Ground truth: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- Wrapper port: `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port/src/lib.rs`
- Harness DB facts: `cert_actual_producer_function_c_2026_05_11`, `cert_entropy_entry_stage_drv_2026_05_11`, `cert_stage_drv_inputs_detail_2026_05_11`, `cert_challenge_injection_via_cli_flag_2026_05_11`, `snapshot_mutation_negative_result_2026_05_11`.

## Progress log
Append to `/home/sdancer/orchestrator/analysis/checkpoints/callback_body_port_v2_progress_2026-05-11.jsonl`.

## Operating mode
In-process Agent (background). Iterate tasks 1→2→3. **You may stop and report partial progress.** A clean disasm + reverse of stage_drv alone is valuable. STOP and report if: (a) the 5.1 MB dump doesn't contain Function C or stage_drv at the expected offsets (sanity-check first bytes match a function prologue), (b) the captured stage_drv outputs in the trace are non-deterministic given the inputs (means there's hidden global state we missed), or (c) Rust port produces well-formed 48-hex output but 0/5 match across multiple attempts to fit (means the algorithm we're modeling has unknown additional inputs — escalate with the diagnosis).
