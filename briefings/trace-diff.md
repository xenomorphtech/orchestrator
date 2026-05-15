# trace-diff — capture cert-emission tail, then produce a complete algorithm slice

## Role & workdir
You own per-instruction trace capture of the cert path in `native-replay-rs` and the cross-challenge differential that isolates the **algorithm slice** for downstream synthesis. Workdir: `/home/sdancer/nmss-emu-trace-diff/` (already instrumented by the previous worker — do NOT recreate).

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` — recover the challenge→cert transformation and its data dependencies.
- **Metric:** `fraction_of_algorithm_slice_synthesized_to_rust (0..1)`. Current 0.0 — moves up only when downstream synthesis lands. This path's contribution is to make the slice **complete** so synthesis becomes possible.

## Success criteria
- Capture full per-instruction traces for all 5 challenges that include the cert-emission tail (the final hash/output stages). Previous run captured 4.4–9.1M instructions per challenge and stopped mid-cert inside `call_getcert`.
- Re-run the diff on the complete traces and update `/home/sdancer/orchestrator/analysis/trace_diff_2026-05-11/algorithm_slice_algo_only.jsonl` so it covers cert emission.
- Set fact `nmss_cert_traces_complete_5x_2026-05-11` to the algorithm-slice path when done.

## Progress so far (previous worker — 2026-05-11)
- **Phase 1 — anti-obfuscation neutralized.** 7 syscalls stubbed behind `NMSS_REPLAY_STUB_TIME=1`: `nanosleep(101)`, `clock_nanosleep(115)`, `clock_gettime(113)`, `clock_getres(100)`, `gettimeofday(169)`, `times(153)`, `getrusage(165)`. 5/5 cert_vector tests still pass with stubs enabled (no trace).
- **Phase 2 — LL/SC emulation.** Added `is_exclusive_loadstore` + `emulate_exclusive_ldst` for LDXR/LDAXR/STXR/STLXR/LDXP/LDAXP/STXP/STLXP — PTRACE_SINGLESTEP clears the local exclusive monitor and the cert path hits LL/SC via libc pthread_mutex. Always-succeed semantics; safe for cert path.
- **Phase 3 — per-instruction trace writer.** `--step-trace-every-instruction`, `--step-trace-out`, `--full-trace-max-steps` flags added. Env `NMSS_REPLAY_TRACE_OUT=<dir>` makes `assert_replay_vector` open per-challenge JSONL. PC-region filter implemented via `load_snapshot_executable_ranges` (lines ~477–581 of main.rs).
- **Run #1 captured partial traces only.** 4 challenges ran in parallel (~10 min), 0000 ran ~16 min (started earlier). Per-trace JSONL size: 0000=4.78GB, others 2.32–2.42GB. After `gzip --rsyncable`: 0000=83MB, others ~39MB. All 5 visit the SAME 12,087 unique PCs (challenge drives compute, not control flow). The traces stop mid-cert; final hash/output not represented.
- **Algorithm slice computed.** 27,997 algo-divergent rows (after dropping SP/x18/x29/x30 env noise). Top divergent registers: x19 (21,464), x20, x21, x22, x15, x2, x10, x3. Hottest divergent PCs `0x78cd3a561c–60` map to `/data/data/com.netmarble.thered/files/CFF3FAD10` — JIT-loaded code region, the cert algorithm lives there.
- **CNTVCT_EL0 scan.** 7 direct reads found (mask `(instr & 0xFFFFFFE0) == 0xD53BE040`); not patched because syscall stubs were sufficient. Report: `/home/sdancer/orchestrator/analysis/trace_diff_2026-05-11/cntvct_reads.json`.
- **NEW anti-obfuscation flagged.** Trace contains ASCII fragments `/proc/%d/`, `%d/spam/` in registers — cert path reads `/proc/self/maps`. The synthesized `maps.txt` in the snapshot may not be bit-exact to the original device's `/proc/self/maps`; this could be why historical Rust ports fail. Separate `proc-maps-probe` path will investigate; you don't need to.

## Next 2–3 concrete tasks (ORDERED — do task 1 first)

1. **Use the existing PC-region filter to constrain single-stepping to the CFF3FAD10 region** (and any other divergent regions from the previous slice). The strlen loop at `0x78cd4294c0–cc` (2.13M visits per test) and `0x78cd428fb0–bc` (102k visits) ate most of the previous budget — they live in libc and contain no algo-divergent register state. Configure the trace writer so that within these region-PCs we run normal `PTRACE_CONT` (block-level, breakpointed on region exits) and only SINGLESTEP inside CFF3FAD10. Existing `load_snapshot_executable_ranges` should give you the region table.

2. **Run all 5 cert_vector tests serially (not parallel) with the region-filtered trace** on `root@162.244.80.97` (keyless SSH; password fallback in `/home/sdancer/orchestrator/.env`). Budget: up to 90 min per test, ~8 h total wall. Snapshot is at `/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/` (shared with sibling paths — read-only OK). Confirm 5/5 still pass under tracing AND that the trace runs all the way through `turn-completed` (i.e. cert is actually emitted). gzip results, pull back to `/home/sdancer/orchestrator/analysis/trace_diff_2026-05-11/traces_v2/`.

3. **Re-run the diff against the complete traces.** Update `algorithm_slice_algo_only.jsonl` and `summary.json` in place (back up the old ones first as `*_v1.jsonl`). Verify the slice now contains the cert-emission tail by checking that the last few rows show register state consistent with the 48-hex cert ASCII building up.

## Constraints & gotchas

- **No git commits, no git state changes.** Edit `main.rs` in place in this worktree.
- **The minimization path (`/home/sdancer/nmss-emu-fulltrace/`) is a SIBLING path** with a separate uncommitted modification to its own copy of main.rs (added a PROT_NONE / SIGSEGV handler). Do NOT touch that worktree.
- **The /proc/self/maps anti-obfuscation lead is for the `proc-maps-probe` path**, not you. If you observe the cert reading maps during your runs, just note it in the report.
- **Async-signal-safety** still matters in any signal handler edits — pre-allocated buffers, no `println!` / `Vec::push` inside handlers.
- **5/5 oracle pass is non-negotiable.** Any instrumentation change must still produce 5/5 — if the region-filter breaks it (e.g. by skipping a needed page), back it out and report the failure mode.
- **LL/SC emulator force-succeeds** the store-exclusive status. Safe for cert path; if you observe code reading the status register expecting a real failure, report and consider tightening.
- **adb is not in the cert path** — this is pure native aarch64 Linux on `root@162.244.80.97`. No device interaction.

## Relevant files / references

- Local worktree: `/home/sdancer/nmss-emu-trace-diff/native-replay-rs/`
- Remote staging: `root@162.244.80.97:/root/nmss-emu-trace-diff/`
- Previous run artifacts: `/home/sdancer/orchestrator/analysis/trace_diff_2026-05-11/`
- Algorithm slice (v1): `algorithm_slice_algo_only.jsonl` (86 MB, 27,997 rows)
- Cross-challenge summary (v1): `summary.json`
- CNTVCT_EL0 reads: `cntvct_reads.json`
- Ground truth: 5 (challenge → 48-hex cert) pairs at `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json` AND `/home/sdancer/nmss-emu/analysis/checkpoints/native_cert_all5_clean_session_2026-05-02.json`.
- Portfolio doc: `/home/sdancer/orchestrator/analysis/paths.json` — your path is `nmss_cert_transformation_recovered.paths[0]`.
- Hypothesis row: `/home/sdancer/orchestrator/analysis/hypotheses.md` — status `stalled`, ready to move back to `active` when you produce a complete-traces artifact.

## Operating mode
Codex (codex_app_server kind). When you finish task 1+2, post a single-paragraph summary so the next orchestrator tick can update the path status. If task 2 reveals that the cert path branches differently under region-filtering (e.g. produces a different cert because we skipped a libc page that mutates state), STOP and report — that's a falsification of the region-filter shortcut and the path needs to back off to full single-step with a longer budget.
