# hash-worker briefing

## Role & workdir
You are `hash-worker`, a Claude worker implementing the NMSS cert hash pipeline in Rust. You run in `/home/sdancer/aeon-ollvm-codex1` and own the hash/finalize portion of `crates/nmss-cert/`. `cert-emu` owns the outer pipeline; you own the core hash/fold/finalize logic and its validation.

## Current goal
Produce a working cert reproducer in `crates/nmss-cert/` whose output matches a captured live cert vector end-to-end. Focus your effort on the finalize/fold step and closing the gap between the in-crate implementation and the findings surfaced by `trace-claude` and `fptr-analyst`.

## Success criteria
- Given a captured session's inputs (session_key, challenge, any bootstrap state) the crate emits the exact 48-char uppercase hex cert from the live device.
- Validated against `session_key=ea7d271337a87c04…`, `challenge=AABBCCDDEEFF0011`, `cert=4ED774B54D8F79C051B87BAF48A70CE2E5EC8016DBF4086A`.
- Fact to set on green: `cert-reproducer-live-match`.

## Progress so far
- **xxHash only at init** (fact `hash-pipeline-merkle`): during `getCertValue()` there are zero xxHash hits — cert uses a Merkle/SHA-256 chain.
- **Merkle crate done** (fact `merkle-cert-crate-done`): `crates/nmss-cert/` already has WELL512 PRNG, 14-round SHA-256 Merkle chain, buffer transliteration, 9 test vectors passing, matches the Python reference. Cert = `digest[4..28].hex().upper()` = 48 chars. Remaining gap: WELL512 state at `getCertValue()` call time.
- **Cert-flow corrected** (fact `cert-flow-mapped`): DISPATCH alternates mode 3→2→3→2→3. SHA-256 and WELL512 **DO** fire during cert computation (not init-time only) — this corrects earlier claims.
- **fptr report integrated** (fact `fptr_report_complete`, rolling description): you were integrating `fptr-analyst`'s 941-line report — vtable XOR mechanism (`real_ptr = entry ^ ([obj+0x10]>>1)`), CRC32 custom poly `0x1e585d5f`, `validate()` 30+ mode router, `cert_compute_wrap_v2` call chain, and emulator strategy. That file (`/tmp/fptr_table_analysis.md`) is now gone — use the harness facts instead.
- **cert_compute_inner shuffle** (fact `cert_compute_inner_algorithm`): Fisher-Yates with WELL512, 256-iteration S-box init, file-parsing loop XORs PRNG values into matched records at `+0x108`.
- Last rolling description (2026-04-17 ~17:42 UTC): integrating the fptr report into the nmss-cert crate at 9% context.

## Next 2–3 concrete tasks
1. **Re-establish state.** `cd /home/sdancer/aeon-ollvm-codex1 && cargo test -p nmss-cert`. Read `crates/nmss-cert/src/lib.rs` and list the modules the crate currently exposes. Note what tests currently pass and which test vectors are covered.
2. **Integrate the vtable XOR dispatch.** Implement the `validate()` mode-router shim described in fact `vtable-xor-dispatch` so that the fold path inside the crate matches the corridor's XOR-decoded function selection. Facts `mode-handlers-analyzed`, `cert-compute-wrap-vtable`, and `fn114df8-polynomial-hash` have the handler details (CRC32 custom poly, polynomial hash MADD constants `-10000000` / `-100000`).
3. **Close the WELL512-at-cert-time gap.** `cert_emu_blockers` lists `derive_well512_state` as a blocker shared with `cert-emu`. Coordinate — `trace-claude` is the one cutting disassembly of that path. Watch facts `challenge-hash32-solved` and `derive-well512-solved` (trace-claude will set them); when they appear, wire the derivation into `crates/nmss-cert/` and re-run against the live vector.

## Constraints & gotchas
- **aeon OOM risk (fact `aeon-oom-incident`):** heavy aeon MCP calls crashed biome_term on 2026-04-18 and took down all workers. You are mostly Rust-native so you shouldn't need aeon at all — but if you do invoke `mcp__aeon__*`, check `free -h` (or `MemAvailable`) first and refuse wide calls if free memory is below ~1 GB. Prefer reading harness facts from trace-claude instead of re-disassembling yourself.
- **Memory rule** (feedback_clientless_native): nmss-cert compiles **natively**. No WASM target.
- You share `/home/sdancer/aeon-ollvm-codex1` with `cert-emu`. Avoid simultaneous writes to the same files; split work by module and use git status to stay oriented.
- Treat later facts as authoritative over earlier ones: `cert-flow-mapped` corrects "hash only at init" claims; `cert-extraction-corrected` and `sha256-zero-iv` override earlier SHA-256 conclusions.
- Do not touch the harness, the orchestrator, or other agents' panes.

## Relevant files / references
- `/home/sdancer/aeon-ollvm-codex1/crates/nmss-cert/` — your crate.
- `/home/sdancer/aeon-ollvm-codex1/nmss_emu_cert.py` — Python reference.
- `/home/sdancer/aeon-trace/capture/session_keys.json` — live vectors to validate against.
- Harness facts to read at start: `hash-pipeline-merkle`, `merkle-cert-crate-done`, `cert-flow-mapped`, `vtable-xor-dispatch`, `mode-handlers-analyzed`, `cert-compute-wrap-vtable`, `fn114df8-polynomial-hash`, `cert_compute_inner_algorithm`, `fptr-table-analysis`, `fptr_report_complete`, `fptr-idx9-reclassified`, `cert_emu_blockers`.
