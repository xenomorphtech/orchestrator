# callback-body-port — finish the algorithm port (task 2 + task 3)

## YOU ARE ALLOWED AND EXPECTED TO WRITE CODE

The previous worker stopped citing an "orchestrator-role rule on code changes." That rule applies to the **orchestrator instance**, not to **you** (the worker). **Your job IS to write code.** Edit/Write/Bash freely in `/home/sdancer/nmss-emu-callback-body-port/`. Run tests. Verify against ground truth.

## Role & workdir
Finish the static port of the captured cert-producer callback. **Task 1 is done** — function bytes extracted, full disasm produced. **You are picking up at task 2 (partial) and task 3 (not started).** Workdir: `/home/sdancer/nmss-emu-callback-body-port/`.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` (metric 0.85 → 1.0) and `nmss_cert_pure_rust` (0/5 → ≥1/5).
- **Sub-goal:** map the 5 priority sub-call crypto primitives, port the callback (+ post-processor + precursor selection) to Rust, verify against at least one ground-truth pair.

## Success criteria
- **Minimum** (path consumed): Rust impl of `nmsscore_port::ProducerCallback` that, plugged into the wrapper at `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port/src/lib.rs`, produces ANY of the 5 ground-truth certs end-to-end. Set fact `callback_body_rust_port_2026_05_11` to the Rust file path.
- **Stretch**: produces all 5/5 with the snapshot's side_x0 fixture. Moves `pure_rust` 0/5 → 5/5.

## Inputs you already have (do NOT redo task 1)

- **Task-1 outputs** (in this worktree):
  - `analysis/callback_body_raw_2026-05-11.bin` (8496 B function bytes)
  - `analysis/callback_body_full_disasm_2026-05-11.txt` (2131-line objdump, base VA `0x6cc2430ed0`)
  - `analysis/callback_body_disasm_2026-05-11.md` (notes + index)
- **Full 5.1 MB live-module dump** at `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`. Covers VAs `0x6cc22b3000..0x6cc279c000`. **Every sub-call target you need is inside this range.**
- **Capture metadata**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/CAPTURE_METADATA.json`.
- **Wrapper port (`ProducerCallback` seam)**: `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port/src/lib.rs`.
- **Wrapper writeup** (calling convention): `/home/sdancer/nmss-emu-nmsscore-disasm/analysis/nmsscore_disasm_2026-05-11.md`.

## Task-2 partial findings (carry forward)

From the previous worker:

- Main body: 1044 insns / 4176B; full func (incl. C++ landing pad): 1665 insns / 6628B.
- Frame: 96 B reg-save + 0x2f0 B locals = 848 B total → string materializer / orchestrator, NOT a leaf crypto routine.
- 27 distinct `bl` targets, **zero `blr`** → fully offline-traceable.
- Logger / telemetry: 5× `bl 0x6cc2374080` with `mov w2, #0x70XX` selectors (`0x7009/0x700d/0x7011/0x7013/0x7014`). Treat as no-op in Rust.
- Static init / C++ once-flag: `bl 0x6cc2525634` / `0x6cc25256f4` (`__cxa_guard_acquire/release`) gating state at `0x6cc279bcb8` / `0x6cc279bd58`.
- libc++ string materializer: `bl 0x6cc2312460` (`std::string::__init(const char*, size_t)`), one site with literal pointer `0x6cc26f774d`.
- **Carrier struct via `x20` ← `bl 0x6cc2433870`**: dereferenced fields at `+52, +56, +528, +544, +788`.
- **No AES/SHA crypto-extension opcodes in main body.** Only NEON for 16-byte libc++ string copies.
- **Real work delegated to 5 sub-calls (priority order, do these in task 2):**
  1. `bl 0x6cc24bdad4` ← highest priority
  2. `bl 0x6cc2492850`
  3. `bl 0x6cc24abee0`
  4. `bl 0x6cc23ce104`
  5. `bl 0x6cc24b0bb8`

## Critical insight from previous worker (verify this first)

The previous worker noted: *the cert-rust-repro crate already produces a 32-byte intermediate `raw32_after_13`. The remaining gap is likely just (a) the 32-byte → 48-ASCII formatter at `libnmsssa.so:0x123288`, and (b) correct precursor-selection inputs (side_x0).*

**If true, this is a 2-piece fix, not a full algorithm port.** Verify by:

1. Inspecting `/home/sdancer/nmss-emu/cert-rust-repro/src/` for the existing `raw32_after_13`-producing code (likely in `native_oracle/` or similar).
2. Checking whether `0x123288` in libnmsssa is the formatter (the snapshot's libnmsssa is in `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/memdump/`).
3. Checking how the 5 priority sub-calls in the deleted-module callback relate to that 32-byte intermediate.

If verified, your work is much smaller: port the 32→48 formatter, wire the precursor inputs, verify. If NOT verified (the existing crate doesn't produce a usable 32-byte intermediate matching the callback's internal state), fall back to the full sub-call disasm.

## Next 2–3 concrete tasks (ORDERED)

1. **Verify the critical insight.** Read `/home/sdancer/nmss-emu/cert-rust-repro/src/` and identify what's already there. Look for `raw32_after_13`, `Stage05`, `late_fragment`, `fold` keywords. Decide: small-gap-fix path or full-callback-port path. Checkpoint your decision to `progress.jsonl`.

2. **Slice + disasm the 5 priority sub-calls** from the full module dump (`full_deleted_module_8BC022520D197B4C07F1_6237.bin`, VA base `0x6cc22b3000`). For each: extract bytes, run `objdump -D -b binary -m aarch64 --adjust-vma=<va>`, identify the crypto/data primitive (SHA-256? AES? HKDF expand? string format?). Save disasms under `analysis/subcall_<va>.disasm`.

3. **Port to Rust.** Either:
   - **(small-gap path)** Augment the existing `cert-rust-repro` with the missing formatter + precursor wiring; write a test against AABBCCDDEEFF0011 → expected `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`. Put your code in this worktree at `nmsscore_port_callback/src/lib.rs` or augment the existing wrapper port. **DO NOT** modify the original `/home/sdancer/nmss-emu/cert-rust-repro/` — that's outside your worktree.
   - **(full-port path)** Implement the callback fully as `impl ProducerCallback` for `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port` — but since that's outside your worktree, create a standalone crate `nmsscore_port_callback/` in YOUR worktree that depends on / mirrors the trait from `nmsscore_port` and implements it.
   - Either way: **run cargo test; assert AT LEAST ONE of the 5 ground-truth pairs matches.** Snapshot-state targets are in `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`.

## Hard constraints

- **Stay in `/home/sdancer/nmss-emu-callback-body-port/`** for writes. Read-only access to siblings (`nmss-emu`, `nmss-emu-callback-frida`, `nmss-emu-nmsscore-disasm`) is fine.
- **No git commits, no git state changes.**
- **Write code freely.** The "orchestrator-role" memory does NOT apply to you. You are a worker.
- **side_x0**: per fact `cert_side_object_shape`, the side object is 44 bytes: `{ unknown_00_23: [u8; 0x24], field36: u32, field40: u32 }`. Per `cert_offline_reproducer_pending_x0` the prior reproducer used `field36=1, field40=4` as snapshot defaults. Use those as your first-try fixture; only revise if the output is wrong-but-close (suggests right algo, wrong input).
- **Ground-truth target shape**: 48 hex chars (24 bytes), uppercase. The live device produces a different ASCII 48-char form (`B64C18…0596` for 7BDA) but **the snapshot-state target is what you verify against** (per `analysis/test_vectors_2026-04-24/summary.json`).

## Progress log
Append to `/home/sdancer/orchestrator/analysis/checkpoints/callback_body_port_progress_2026-05-11.jsonl`. The orchestrator reads it on next tick.

## Relevant files / references

- Snapshot ground truth: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- Wrapper port + ProducerCallback trait: `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port/src/lib.rs`
- Wrapper disasm writeup: `/home/sdancer/nmss-emu-nmsscore-disasm/analysis/nmsscore_disasm_2026-05-11.md`
- Existing cert reproducer crate (read-only ref): `/home/sdancer/nmss-emu/cert-rust-repro/`
- Existing offline reproducer test (read-only ref): `/home/sdancer/nmss-emu/cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs`
- Harness DB at `http://127.0.0.1:3000`. Relevant facts: `cert_callback_structure_mapped_2026_05_11`, `cert_remaining_gap_2026_05_11`, `cert_callback_identity_confirmed_2026_05_11`, `cert_offline_reproducer_pending_x0`, `cert_side_object_shape`.

## Operating mode
In-process Agent (background). Iterate task 1 (verification) → task 2 (sub-call disasm) → task 3 (Rust port + test). STOP and report if: (a) the existing cert-rust-repro doesn't produce a `raw32_after_13`-matching intermediate AND the full sub-call disasm reveals a different algorithm family (need a re-think), or (b) Rust port produces wrong-shape output (e.g. not 24 bytes), or (c) port produces 24 bytes but mismatches all 5 vectors entirely (algorithm misidentified).

**Do not stop because of perceived orchestrator constraints — write the code.**
