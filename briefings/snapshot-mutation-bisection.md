# snapshot-mutation-bisection — locate per-challenge entropy via mutation testing

**You ARE allowed and expected to write code.** The orchestrator-role memory does NOT apply to worker sub-agents.

## Role & workdir
Mutate carrier-state bytes in the trampoline memdump snapshot one at a time, run `native-replay-rs` against each mutation, and observe which mutations change the produced cert. If ANY mutations produce different certs, that localizes the per-challenge entropy region in the snapshot. If ALL mutations produce identical certs, that hard-confirms the cycle-13 bypass-finding (`FALSIFICATION_offline_algorithm_from_static_2026_05_11`): the snapshot's "5/5 reproduction" is informationally a baked-in lookup, not algorithmic. Workdir: `/home/sdancer/nmss-emu-snapshot-mutation/`.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` (metric 0.20 → up to +0.4 with this path).
- **Diagnostic value:** the binary result (mutations matter / don't matter) constrains all other paths' interpretation:
  - "mutations matter" → snapshot has more entropy than thought; reopens snapshot-side investigation; the `--ctx-seed-240-text` bypass note in `WIKI.md:13` may be specific to one tooling lane.
  - "mutations don't matter" → snapshot-side investigation is doomed; live capture (`callback-instrumented-trace`, sibling running) is the only viable path.

## Success criteria
- Reproducible mutation-bisection script at `scripts/mutate_and_replay.py`.
- Per-mutation result table at `analysis/snapshot_mutation_2026-05-11/results.jsonl`: `{region_path, byte_offset, original_value, mutated_value, cert_for_<challenge>, changed: bool}`.
- Summary at `analysis/snapshot_mutation_2026-05-11/summary.md` answering: did any mutation change the cert? Which regions/offsets? What's the smallest mutation set that produces the largest cert delta?
- Set fact `snapshot_mutation_bisection_2026_05_11` with the summary path.

## Approach (recommended)

1. **Baseline.** Run `native-replay-rs` against the snapshot at `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/` (the working copy where the `token_matches` 5/5 was demonstrated) — but operate on the snapshot at `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/` for mutation purposes. Confirm baseline produces the expected `90237F0E…` for 7BDA. **However** — per `WIKI.md:13` the prior "5/5" used `--ctx-seed-240-text`. Test BOTH variants: with and without that flag. Document which one matches the documented ground truth.

2. **Smart mutation set.** Don't blindly mutate 4.4 GB of memdump pages — too slow. Smart picks:
   - **`raw32_after_13` source bytes** (per fact `cert_offline_static_plateau_2026_05_03`): the 32-byte intermediate has documented inputs. Mutate the 13 record ASCII strings that feed the SHA chain (find them via grep through the memdump for patterns matching `cert-rust-repro/src/native_oracle/stages/` references). 1 byte at a time × few hundred bytes.
   - **Challenge area in snapshot**: the snapshot was captured during execution of 7BDA's cert. Find any 8-byte sequence matching `0x7BDA93D2F45D36C0` (little-endian or big-endian, ASCII or hex bytes) in the memdump — those are candidate "challenge slots". Mutate each and see if the cert moves.
   - **Carrier-struct fields** (per fact `cert_callback_structure_mapped_2026_05_11`): offsets +52, +56, +528, +544, +788 of the carrier struct (returned by `bl 0x6cc2433870`). Locate these in the snapshot, mutate, replay.
   - **The snapshot's challenge field** (per fact `arm_manual7_challenge_field_diagnosis_2026-05-01`): there's a documented per-challenge field at snapshot offset 0x2150. Different snapshot; but worth checking analogous location in trampoline memdump.

3. **Run + record.** For each mutation, copy the snapshot dir, apply the byte patch, run `cargo test cert_vector_7bda93d2f45d36c0 --release` (or equivalent), record the produced cert + whether it changed. Use `dd if=/dev/zero of=<patched>.bin bs=1 count=1 seek=<offset> conv=notrunc` or write a small Python helper. **Don't mutate the original** — work on copies.

4. **Synthesize.** Group mutations by cert-impact. If ALL produce identical cert → log this as confirmation of the bypass-hypothesis and stop. If SOME produce changes → identify the region(s); cross-reference with the cert-callback's documented input ranges; report.

## Constraints & gotchas

- **Original snapshot is read-only** in `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/`. Mutate copies in your worktree.
- **Cargo test for cert_vector_* requires aarch64 host.** The remote ARM box `162.244.80.97` was used for the 5/5 demo (`token_matches_2026-04-24.json`). You may need to ship a mutated snapshot to it and run there. Or — check if your local x86 host can replay the snapshot under qemu-aarch64 (probably too slow). The remote box has a copy of the snapshot at `/root/nmss-emu-trampoline/`; you can mutate there directly via SSH.
- **5/5 vs bypass**: critically verify whether the original 5/5 reproduction used `--ctx-seed-240-text VALUE`. Read the test command in `cert-rust-repro/tests/`. If yes, your "baseline" actually injects the answer — you must use the variant WITHOUT that flag for the mutation to test the real algorithm. The bypass-flag mode will trivially produce the same cert no matter what you mutate (since the answer is in the flag).
- **Time budget ~3h.** Don't get stuck rebuilding native-replay-rs. The remote box has a built binary; mutations are just byte-patches + reruns.
- **No git commits.**

## Relevant files / references

- Remote host: `root@162.244.80.97`, keyless SSH set up. Snapshot mirror: `/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/`. native-replay-rs build there too.
- Original snapshot (read-only): `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/`
- Working native-replay-rs at the token-matches commit: `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/` (this is the canonical 5/5 source)
- 5 ground-truth pairs: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- WIKI.md line 13 (the bypass note): `/home/sdancer/nmss-emu/WIKI.md`
- Key facts: `FALSIFICATION_offline_algorithm_from_static_2026_05_11`, `cert_callback_structure_mapped_2026_05_11`, `cert_offline_static_plateau_2026_05_03`.

## Progress log
Append to `/home/sdancer/orchestrator/analysis/checkpoints/snapshot_mutation_progress_2026-05-11.jsonl`.

## Operating mode
In-process Agent (background). Iterate baseline → smart mutation set → results → summary. **STOP early and report** if (a) baseline can't reproduce documented 5/5 even without `--ctx-seed-240-text`, (b) you observe the FIRST mutation changing the cert (continue but flag — that's the answer), or (c) the entire mutation set produces identical certs (also the answer — bypass confirmed).
