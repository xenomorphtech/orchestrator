# oracle-service — wrap native-replay-rs as POST /cert HTTP service

**You ARE allowed and expected to write code.** Rust, Python, shell. Worker sub-agent rule.

## Role & workdir
Wrap the working `native-replay-rs` binary + the canonical snapshot dir as a small HTTP service: `POST /cert {challenge}` → JSON `{cert, elapsed_ms, status}`. Workdir: `/home/sdancer/nmss-emu-oracle-service/`.

## Why this is a hedge

Cycle-17 reframe established that native-replay-rs is genuinely algorithmic (5 challenges → 5 distinct correct certs, mutation-bisection cycle-23 confirmed via positive-control). Cycle-24 found stage_drv is Themida-grade obfuscated — pure-Rust port is hard. Cycle-27 found onleave-trace-capture (the bypass path) has a broken hook. **`oracle-service` ships a real-algorithm cert service NOW using the already-working binary, independent of whether onleave/symbolic-exec eventually recover pure Rust.** Predicted Δmetric is "deliverable not hypothesis" — it doesn't move `transformation_recovered` but unblocks any downstream `pure_rust` consumer waiting for an end-to-end interface.

## Current goal / sub-goal
- **Goal**: `nmss_cert_pure_rust` (orthogonal — ships the *service form* of the deliverable while pure-Rust paths continue).
- **Sub-goal**: `oracle-service` deliverable: a binary + systemd-style or shell-script process that listens on an HTTP port and returns certs for any of the 5 ground-truth challenges (or any future challenge if the snapshot supports it).

## Success criteria
- Service running at `http://127.0.0.1:9876/cert` (or pick a port, document it) accepting `POST {challenge: "AABBCCDDEEFF0011"}` and returning `{"cert": "8F868A5849505353C39BA200827F07EA635A3F71D2DE812C", "verified": true, "elapsed_ms": N}` for each of the 5 ground-truth pairs.
- Smoke test script that hits the service 5× (once per challenge) and asserts all 5 verified=true.
- Set fact `oracle_service_running_2026_05_11` with the host:port and the start command.

## Inputs you have

- **native-replay-rs binary**: on the ARM remote at `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`. Takes args `<snapshot_dir> --challenge <hex16> --out <json>`. Already verified 5/5 (fact `snapshot_algorithmic_confirmed_v2_2026_05_11`).
- **Snapshot dir on ARM**: `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/`
- **Ground truth**: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- **Working cargo test path** (for sanity-check baseline): on the ARM remote, `cd /root/nmss-emu-trampoline/native-replay-rs && cargo test cert_vector_ --release -- --test-threads=1` returns 5 passed.
- **Reuse-existing-Rust path** (preferred over a Python shim): the `native-replay-rs` crate has all the plumbing; just add a small HTTP wrapper crate in this worktree at `/home/sdancer/nmss-emu-oracle-service/oracle_service/` that calls into the binary via subprocess OR (better) factors `child_inner` from main.rs into a library function and links it directly.

## Next 1-2 concrete tasks (ORDERED)

1. **Decide implementation surface**: simplest viable is a Python `Flask`/`uvicorn` (or pure-Python `http.server`) wrapper around `subprocess.run(['native-replay-rs', snapshot_dir, '--challenge', c, '--out', tmpfile])`. Run it on the ARM remote (where native-replay-rs lives). Per-request elapsed_ms is ~17-22s based on observed mutation-bisection timings — that's the snapshot replay's real cost; document it. **Faster path**: factor `child_inner` into a `native-replay-rs` library function and call it directly without spawning a subprocess (avoids fork+mmap+page-fault setup per request) — but if that requires nontrivial refactor (>1h), stick with subprocess.
2. **Test against 5 ground-truth pairs**: write `scripts/smoke_5x.py` that POSTs each of the 5 challenges, asserts the returned cert matches `test_vectors_2026-04-24/summary.json`. Capture latency stats.
3. **Document**: a one-page `README.md` at `oracle_service/README.md` with: start command, port, request format, response format, error cases (unknown challenge → 200 with `verified=false`? or 400?), known limits (5 ground-truth challenges only, OR every challenge if the snapshot is properly indexed — depends on what the binary does with arbitrary challenges).

## Constraints & gotchas

- **No git commits.**
- **Stay in `/home/sdancer/nmss-emu-oracle-service/`** for source; remote service runs at `/root/nmss-emu-trampoline/`-style staging on `162.244.80.97`.
- **Subprocess-per-request is fine for v1**: ~17s/req is acceptable; the goal is correctness, not throughput.
- **Don't add network auth** — bind to 127.0.0.1 only (it's the ARM box, not internet-exposed; if user wants public, that's a separate concern).
- **Don't change native-replay-rs source** unless adding a library function is trivial. The binary is the verified artifact; preserve it.

## Relevant files / references

- native-replay-rs main.rs: `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/src/main.rs` (constants: `CHAL_OFFSET=0x2000`, `SYNTH_SIZE=0x40000`)
- Ground truth: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- 5/5 reproduction proof: `/home/sdancer/nmss-emu/analysis/token_matches_2026-04-24.json`
- Remote access: keyless SSH from this host to `root@162.244.80.97`; password fallback `ARM64_PASSWORD` in `/home/sdancer/orchestrator/.env`

## Progress log
Append to `/home/sdancer/orchestrator/analysis/checkpoints/oracle_service_progress_2026-05-11.jsonl`.

## Operating mode
In-process Agent (background). 1h budget. STOP when service is up + 5/5 smoke pass. Don't gold-plate — this is a hedge deliverable, not the deep R&D path.
