# snapshot-minimizer — find the minimal snapshot subset that still produces 5/5 cert replays

## Role & workdir
You **shrink the kernel-module-produced memory snapshot** (`trampoline_proc_memdump_5558`) to the smallest VMA subset that still yields 5/5 ground-truth cert replays via `native_replay_ab`. The "removed" set tells us which regions are NOT part of the cert algorithm; the "kept" set is the algorithm's actual data dependency footprint. Workdir: `/home/sdancer/nmss-emu/`.

## Goal / success criteria
- Goal key: `snapshot_minimizer`
- Success fact: `snapshot_minimal_set_identified` — set to a JSON-checkpoint path that lists the minimal VMA subset (filename, vaddr, size) plus the verification log showing 5/5 cert matches with that subset alone.
- Concrete done: a shell/Rust harness that, given the original snapshot, produces a reduced copy where any VMA not in the minimal set is either deleted from disk OR mapped `PROT_NONE` at replay time, and `native_replay_ab` still produces correct certs for all 5 challenges (7BDA / 0123 / 0000 / FFFF / AABB or whatever the canonical 5-vector set is on disk).

## Why this matters
- The brute-force search for `final_piece` over 989k snapshot windows missed it. Either it's in a region we didn't search, or it's computed during replay from earlier state.
- Knowing the **touched set** scopes the algorithm precisely. Untouched regions are noise; the cert algorithm cannot depend on them.
- Reduces the snapshot's footprint for downstream replay tooling and makes the dependency graph legible.
- A bonus: any VMA that gets touched contains data the algorithm reads. `final_piece` source must live somewhere in the touched set.

## Method (in priority order)

### A) PROT_NONE-then-fault (preferred)
1. Read `kernel_dump_records.tsv` — it lists every VMA: `filename, vaddr, vaddr_end, size, prot, flags, mapping_label`.
2. Modify the replay loader to **mmap every VMA `PROT_NONE` first** instead of with the recorded prot bits.
3. Install a SIGSEGV handler that:
   - Reads `siginfo->si_addr` (the faulting address)
   - Looks up which VMA contains it
   - `mprotect`s that VMA to its **original** prot (R / R+W / R+X)
   - Logs the VMA name to a touched-set file
   - Returns from the handler so the instruction retries
4. Run `native_replay_ab` against all 5 challenges in sequence (or one at a time if state isolation requires).
5. The touched-set is the union of mprotect'd VMAs across all 5 runs.
6. Verify: drop everything else, re-run, confirm 5/5.

### B) Iterative bisection (fallback if instrumenting the loader is too invasive)
1. Sort VMAs by size, descending.
2. For each VMA in the snapshot: try removing it (or mapping `PROT_NONE`); rerun; keep removed iff still 5/5; restore otherwise.
3. Fast-path: bisect by halves once you've established a stable touched-set candidate.

### C) Static analysis backstop
1. If neither A nor B is feasible (e.g. `native_replay_ab` can't run on the dev host because of qemu PTRACE issues), fall back to: trace which guest addresses appear in the disassembly of `libnmsssa.so` and the dependent libs along the cert call path (callers of 0xd10c4 / 0xd424c / 0x206674 / etc.) and compute the reachable VMA set statically. Less precise but a starting point.

## Constraints & gotchas
- `native_replay_ab` source: `/home/sdancer/nmss-emu/native-replay-rs/src/` (also a built copy at `/home/sdancer/nmss-emu-ondevice/native-replay-rs/target/aarch64-linux-android/release/native_replay_ab` for the on-device path).
- The host can't natively run aarch64 ptrace (qemu PTRACE_TRACEME ENOSYS). If the on-device path is required, you'll need adb access — currently down. **First, check whether `native_replay_ab` runs at all on this host without ptrace**; some replay subcommands may work without ptrace.
- The 5/5 ground-truth was previously demonstrated. Find which configuration / command was used. Likely committed in `analysis/checkpoints/` or `findings/`. Look for previous "5/5" results to anchor the working baseline before shrinking.
- VMA boundaries from the kmod might be coarse (whole regions). True minimization at page granularity (4KB) needs an extra pass — start with VMA granularity, then refine within touched VMAs.
- Don't delete the original snapshot. Work on a copy: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_minimized/` or use a worktree.
- DO NOT modify `/tmp/libnmsssa.so` or anything outside this task's scope.

## Cross-pollination context
- cert-rust-reimpl and cert-ptrace concluded `final_piece` lives at `*(x19+904)+0x470` (libc++ string), with the long-string ptr dead-ending at guest `0x1000145e10` — that address may or may not be in the snapshot. **Specifically check `kernel_dump_records.tsv` for whether 0x1000145e10 is covered.** If present in a touched VMA, the brute force missed it (different alignment or different layout); if absent, the kmod missed that region and a re-snapshot is needed.
- Writer PCs (snapshot-relative): 0x78c690e1b8 area for selectors; 0x206674 / 0x20b158 / 0x20e25c (libnmsssa.so file offsets) for `+0x470` field. These accesses MUST be in your touched set — if they aren't, the replay isn't reaching the cert path.

## Reporting cadence
- Initial scoping checkpoint: confirm baseline (which command produces 5/5? does it run on host?) → save to `analysis/checkpoints/snapshot_minimizer_baseline_2026-05-03.json`.
- Per-method milestone checkpoint after each round.
- Final result: `analysis/checkpoints/snapshot_minimal_set_2026-05-03.json` with `{"touched_vmas":[...], "removed_vmas":[...], "verified_challenges":[...]}` and set fact `snapshot_minimal_set_identified` to that path.

## Relevant files / references
- Snapshot root: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/`
- Manifest: `kernel_dump_records.tsv` inside the snapshot
- Replay tool: `/home/sdancer/nmss-emu/native-replay-rs/`
- Kmod producer note: `~/.claude/projects/-home-sdancer/memory/nmss_kernel_module_dump_producer.md`
- On-device tools index: `/home/sdancer/orchestrator/campaign-index/tools/on-device/INDEX.md`
- Memory note about replay being the existing 5/5 oracle: `~/.claude/projects/-home-sdancer/memory/nmss_kernel_module_dump_producer.md`
- Ground-truth challenge→cert pairs (5 vectors): `analysis/checkpoints/native_cert_*_clean_session_2026-05-02.json`
