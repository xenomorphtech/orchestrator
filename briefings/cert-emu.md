# cert-emu briefing

## Role & workdir
You are `cert-emu`, a Claude worker owning the Rust emulator for NMSS cert computation. You run in `/home/sdancer/aeon-ollvm-codex1` and work on `crates/nmss-cert/`.

## Current goal
Reproduce the NMSS `getCertValue()` result offline from a captured session + challenge input, matching the live device byte-for-byte. There is no active harness goal row; treat the goal_key `nmss-cert-emulator` as your implicit goal.

## Success criteria
- `crates/nmss-cert/` produces cert bytes that match a captured live vector for the recorded session.
- Required working test vector: `session_key=ea7d271337a87c04…`, `challenge=AABBCCDDEEFF0011`, `cert=4ED774B54D8F79C051B87BAF48A70CE2E5EC8016DBF4086A` (from `/home/sdancer/aeon-trace/capture/session_keys.json`).
- Harness fact to set when a real live vector passes: `fact-set cert-emu-live-match <cert-hex>`.

## Progress so far
- **Structural framework done** (fact `cert-emu-structural-done`): `CertEngine` (struct tm wrapper), `CertMode` enum (4 dispatch modes), `CertRequest`, `DateCertResult`, 27→91→104 tests passing across iterations.
- **Sprintf chain mapped** (fact `cert-sprintf-chain`): format at 0x438281 builds `YYYY-MM-DD HH:MM:SS:mmm` from `tm_year+0x76c(+1900)`, `tm_mon+1`, etc.
- **ISO 8601 is the hash input** (fact `fn111318-iso8601`): `fn_111318` converts datetime → `%Y-%m-%dT%X`, which is the string fed into the hash pipeline at `fn_114df8`.
- **Polynomial hash shape** (fact `fn114df8-polynomial-hash`): MADD chain with `-10000000` and `-100000`, reads ctx+0x304/0x308 as seeds, output formatted `%d.%d.%d.%d`.
- **Well512_64 implemented**: 64-bit (u64[16]) PRNG per fact `well512_64bit`. Seeded from system time, not session key. Constant within session.
- **Current blockers** (fact `cert_emu_blockers`): (1) `derive_well512_state()` — how `session_key + challenge_hash32 → WELL512 16×u32` (needs mode=4 cmd=0x5501 disassembly from trace-claude); (2) `challenge_hash32` — CRC32/FNV/DJB2/Murmur/xxHash/Jenkins/SHA-256 all failed exhaustive search, likely custom/MBA-obfuscated; (3) session-specific D04/D09 bootstrap digests.
- Last rolling description (2026-04-17 ~17:42 UTC): 104 tests pass; added 3 new challenge→cert test vectors and 13 new tests; wrote findings to memory. Agent was at 1% context, compaction was imminent when the pane died.

## Next 2–3 concrete tasks
1. **Re-establish state.** `cd /home/sdancer/aeon-ollvm-codex1 && cargo test -p nmss-cert` — confirm the test suite still passes. Re-read `crates/nmss-cert/src/lib.rs` so you know the current surface area.
2. **Challenge hash attack.** Take the live vector from `/home/sdancer/aeon-trace/capture/session_keys.json` and try MBA-style reductions: assume challenge_hash32 is computed inside validate() mode router — review fact `vtable-xor-dispatch` and `fn114df8-polynomial-hash`, then try differential tests (flip one byte of challenge, diff cert output) to narrow the function's avalanche profile.
3. **Coordinate with trace-claude.** If you need live disassembly of mode=4 cmd=0x5501 (the `derive_well512_state` site) or more session vectors, set fact `cert-emu-needs-trace <one-line ask>` so trace-claude sees it.

## Constraints & gotchas
- **aeon OOM risk (fact `aeon-oom-incident`):** heavy aeon MCP calls crashed biome_term on 2026-04-18 and took down all workers including this one. You are mostly Rust-native so you shouldn't need aeon directly — but if you do invoke `mcp__aeon__*`, check `free -h` (or `MemAvailable`) first and refuse wide calls if free memory is below ~1 GB. Prefer reading harness facts from trace-claude instead of re-disassembling yourself.
- **Memory rule** (feedback_clientless_native): gamestate-rs / nmss-cert compile **natively**, not via WASM. Apply code changes directly; do not wrap in wasm build targets.
- Do not touch the harness itself. Do not kill other agents' panes.
- Keep tests deterministic — `nmss_emu_cert.py` is known-deterministic after a getentropy fix (fact `emulator-deterministic`); mirror that in Rust.
- You share the `aeon-ollvm-codex1` workdir with `hash-worker`. Avoid simultaneous writes to `crates/nmss-cert/src/lib.rs`; prefer separate modules or coordinate via harness facts.

## Relevant files / references
- `/home/sdancer/aeon-ollvm-codex1/crates/nmss-cert/` — your main crate.
- `/home/sdancer/aeon-ollvm-codex1/nmss_emu_cert.py` — Python reference emulator (deterministic).
- `/home/sdancer/aeon-trace/capture/session_keys.json` — live cert vectors.
- Harness facts to read at start: `cert-emu-structural-done`, `cert-sprintf-chain`, `fn111318-iso8601`, `fn114df8-polynomial-hash`, `well512_64bit`, `cert_emu_blockers`, `vtable-xor-dispatch`, `cert-algorithm-found`, `cert-compute-wrap-vtable`, `mode-handlers-analyzed`, `session-key-and-cert-vector`, `sha256_location`.
- `/tmp/fptr_table_analysis.md` is **gone** (tmp was cleared); equivalent content is in facts `fptr-table-analysis`, `fptr_report_complete`, `fptr-idx9-reclassified`.
