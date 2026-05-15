# nmssCore-disasm — static port of `nmssCoreGetCertValue` to Rust

## Role & workdir
You statically disassemble and port the **identified cert producer** `nmssCoreGetCertValue` from the snapshot shard `78cc403000.bin` to pure Rust. Workdir: `/home/sdancer/nmss-emu-nmsscore-disasm/` (your own worktree on `c654108`). Read-only siblings: `/home/sdancer/nmss-emu/` (main) and the snapshot dir.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` — recover the challenge→cert transformation and its data dependencies.
- **Metric:** `fraction_of_algorithm_slice_synthesized_to_rust` (0..1). Pre-planner estimate ~0.5; your contribution targets +0.4 toward 1.0.
- **Sub-goal:** Port the producer at module_offset `0x17ded0` and the inner producer at `0x492ad4` to a Rust function consistent with both the 17/64-hex static plateau partial signal AND the 48-char live cert capture for 7BDA.

## Success criteria
- Rust function (in `cert-rust-repro/src/native_oracle/nmsscore.rs` or similar new module) that, given the documented inputs of `nmssCoreGetCertValue` (challenge + side_x0 + snapshot-resident state), produces the 48-char ASCII cert.
- Verification: at least 17 of 64 hex characters match the static plateau for 7BDA (matching the prior `cert_offline_static_plateau` cycle 963 partial signal). Stretch: full match on at least one of the 5 ground-truth pairs once `side_x0` is supplied.
- Set fact `nmsscore_disasm_rust_port_2026_05_11` to the path of the new Rust file when done.

## Why this is the right path (control-system context)
- The fold path at `0xdd9f0..0xe1768` in libnmsssa.so is now confirmed to be a **witness/internal route**, NOT the live cert path (fact `cert_producer_nmssCoreGetCertValue_2026_05_03`: `fold_events=[]`).
- The actual live cert producer is `nmssCoreGetCertValue` at `module_base + 0x17ded0` in a deleted-mmap shard. Random filename per session (current sessions: `F7B3B00F8A5`, `86C9DF48BB32`, `6273AE88C490C78D7B`, `CFF3FAD10`) but contents constant.
- Inside it, the cert STRING is built by an inner subcall at `0x492ad4` with `0x485bb8` supplying a singleton/service context (fact `cert_nmssCoreGetCertValue_internal_split` cycle 999).
- Trace-diff is running in parallel attempting a different attack vector; its findings will cross-check your port. They are NOT a dependency.

## Progress so far (8 days dormant, May-3 facts)

- **0x17ded0 disasm partially done** (fact `cert_nmssCoreGetCertValue_internal_split`): "does NOT hex-encode the 48-char output inline. It copies the final return std::string from sp+0x220, where the string was seeded by an internal subcall at 0x492ad4 (with 0x485bb8 supplying a singleton/service context)."
- **Callback chain documented** (fact `cert_callback_chain_resolved_2026_05_03`): cert producer is runtime-resolved by `0xe0ab4` for selector `0x45`, stored at `0x12a59c` into global slot +2896, invoked by `0x12c47c`. Sister slots for selectors 0x44/0x3c-0x3f/0x46-0x49 at offsets 2832-2928.
- **Module base** is 0x78cc288000 (NOT 0x78cc403000 — that's the shard filename). Module-relative offset of the fptr is `0x17ded0`. The shard `78cc403000.bin` contains one rwxp page with this code (fact `cert_F7B3B00F8A5_module_base_correct`).
- **Offline reproducer scaffold exists**: `/home/sdancer/nmss-emu/cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs` (uncommitted, in main checkout). It compiles and runs the HKDF-Expand-Label recipe; structurally complete. Missing input: `side_x0` (44 bytes at `[self+0x510]`).
- **17/64 hex partial match** for 7BDA was reached by cert-rust-reimpl cycle 963 (fact `cert_offline_static_plateau`): `5E73344B9EAF6000801BAB803A8BBF2FDC4948ABDFB26D08C08AA66F73ED55B5` vs expected `BECA86489D2D6F7E305BEF0116BAEF2FDC4FE34B035E8D1F1D880D6D8C6F42D5`. **Note**: the 64-hex `BECA86…` is the *internal/witness* value per fact `cert_jni_return_is_48char_ascii_2026_05_03`; the *real* on-wire cert is the 48-char ASCII string. Your port should target both: the 17/64-hex intermediate (proves you're computing the right intermediate state) AND the 48-char ASCII output (proves you have the full pipeline).
- **48-char live cert for 7BDA**: `B64C183D793AD722DE26F9301D7D66C20A448F1119C90596` (fact `cert_live_elf_hookable_pcs_corrected_2026_05_03`).

## Next 2–3 concrete tasks (ORDERED)

1. **Disasm `0x492ad4` and `0x485bb8` from the snapshot shard.** Use `objdump -D -m aarch64 -b binary --adjust-vma=0x78cc403000` (or the equivalent `aarch64-linux-gnu-objdump`) on `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/memdump/78cc403000.bin` to disassemble the page. The function at `0x492ad4` is the inner producer; `0x485bb8` provides the context. Module base is `0x78cc288000`, shard base is `0x78cc403000`, so file-offset of `0x492ad4` = `(module_base + 0x492ad4) - 0x78cc403000` if 0x492ad4 is a module-relative offset, OR direct file-offset if it's a raw VA. Verify by checking the first few instructions look like a function prologue (`stp`/`mov sp` etc.).
   - Save the disasm of the relevant ~200 instructions to `/home/sdancer/nmss-emu-nmsscore-disasm/analysis/nmsscore_disasm_2026-05-11.md`.

2. **Port `0x492ad4` to Rust** in `cert-rust-repro/src/native_oracle/nmsscore.rs` (or a similar new file). Lift the data dependencies: what does `0x485bb8` provide (the singleton bytes — likely from snapshot)? What other constants are inlined? Express the cert string construction as a pure function `fn build_cert_string(challenge: &[u8;8], side_x0: &SideX0, ctx: &CallbackContext) -> String`. Use the existing offline reproducer's HKDF-Expand-Label recipe as the model for how to thread snapshot-resident state.

3. **Verify against the static plateau and the live 48-char target.** Add a test that runs the Rust port on placeholder `side_x0 = [0u8; 44]` (with field36=1, field40=4 per `cert_side_object_shape`) for 7BDA and asserts: (a) at least 17 of the first 64 hex characters of the *intermediate* match the prior plateau, and/or (b) the 48-char output is exactly `B64C18…` if you can supply real side_x0 from somewhere. If 17/64 doesn't reach, dump the intermediate state at each stage so the next cycle can compare to known-good fixtures.

## Constraints & gotchas

- **No git commits, no git state changes.** Edit only files in `/home/sdancer/nmss-emu-nmsscore-disasm/` (your worktree).
- **Module base / offset arithmetic is treacherous.** The fact stream documents BOTH module-base addresses (e.g. `0x78cc288000`) AND shard-filename-based addresses (e.g. `0x78cc403000`). Confirm what you're disassembling by sanity-checking against known function bytes — e.g. the descriptor selector at `0xcffc4(0x45)` should be inside the same module. If your file-offset math returns garbage instructions, switch the base.
- **objdump emits get_sreg_qualifier_from_value warnings.** Pipe to `grep -v` or `sed`.
- **Do not target the libnmsssa.so wrapper tree.** That's the witness path. Target the deleted-module shard.
- **Do not invent inputs.** If you need `side_x0` and it's not in the snapshot, document the gap and proceed with placeholder; the `side-x0-capture` path is queued for live device recovery.
- **`90237F0E…` for 7BDA is the snapshot-state cert, not the live cert** (fact `cert_jni_return_is_48char_ascii_2026_05_03`). The 5/5 ground truth in `analysis/test_vectors_2026-04-24/summary.json` is correct *for the snapshot state*. Live device produces different ASCII because session-resident state differs.

## Relevant files / references

- Snapshot shard: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/memdump/78cc403000.bin`
- Maps file (gives layout): `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/maps.txt`
- Offline reproducer scaffold (read-only reference): `/home/sdancer/nmss-emu/cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs`
- Per-callback-chain checkpoint: `/home/sdancer/nmss-emu/analysis/checkpoints/cert_callback_producer_chain_2026-05-03.json`
- Disasm of deleted module: `/home/sdancer/nmss-emu/analysis/checkpoints/cert_F7B3B00F8A5_disasm_2026-05-03.json`
- 5 ground-truth pairs: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- Live cert for 7BDA: 48-char ASCII `B64C183D793AD722DE26F9301D7D66C20A448F1119C90596`.
- Plateau snapshot cert for 7BDA: 64-hex `BECA86489D2D6F7E305BEF0116BAEF2FDC4FE34B035E8D1F1D880D6D8C6F42D5` (intermediate witness — not the goal).
- Harness DB at `http://127.0.0.1:3000` (HARNESS_SERVER in `.env`). 1729 facts; grep with `harness facts | grep cert_` for the cert-specific subset.

## Operating mode
Codex via in-process Agent (the harness `agent-add --kind codex_app_server` path requires a workflow refresh — skip for now). Run tasks 1-3, post a single-paragraph summary so the next orchestrator tick can update the path status. If disasm reveals that `0x492ad4` is structurally different from what the call-chain checkpoint implies (e.g. it's a stub that calls another function), STOP and report — the cert pipeline may have shifted again.
