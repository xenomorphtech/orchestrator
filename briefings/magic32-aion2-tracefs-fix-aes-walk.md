# magic32-aion2-tracefs-fix-aes-walk — Analyze captured AES probe data + write closing artifact

## Role & workdir
Offline analyst of already-captured probe data. Workdir: `/home/sdancer/nmss-emu-magic32-aion2-tracefs-fix`. NO MORE CAPTURES — all needed data already exists in `analysis/raw/`.

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: analyze the existing ncgp_aes capture (which already fired with a real `ncgp_setkey` event during aion2 sign-in) and determine whether AES(captured_key, captured_plaintext) matches the captured login-identifier ciphertext.

## Critical context — DO NOT RE-CAPTURE
Prior turns burned ~24 GB of memory via aeon-mcp + accumulating turn state, causing codex thread freeze. Restart was forced. **You must not call aeon MCP, must not run new perf captures, must not run pm clear / am start.** Only analyze the existing artifacts in `analysis/raw/`.

## Success criteria
Concrete deliverable:
- Closing artifact `analysis/aes_walk_2026-05-14.md` documenting:
  - The 3 NCGP-lib probe sets installed (libncgm.so / libncgpa.so / libncgp.so).
  - Per-lib capture file paths + event counts.
  - The fired ncgp_setkey event details: function `IFELVckI9JlDYaHOaQJaAv` @ libncgp.so file offset `0x66f0c`, sample fields (x0, bits, out, lr).
  - Algorithm verification: did AES-128(captured_key_at_x0_ptr, ...) produce a value matching login-identifier hex?
  - Set fact `nmss_magic32_per_session_algorithm_recovered` (success) OR `nmss_magic32_aion2_aes_no_match` (clean falsification).
- Pure Rust impl `compute_magic32(...)` in `/home/sdancer/nmss-emu/cert-rust-repro/src/magic32.rs` if algorithm form recovered.

## Existing artifacts (read these first)
- `analysis/raw/ncgp_aes.dump.txt` (9.2 MB) — libncgp.so probe dump. Look for `record sample` blocks containing `ncgp_setkey` events. The fired event has x0=<key ptr>, bits=<key bits>, out=<schedule ptr>, lr=<caller addr>.
- `analysis/raw/ncgp_aes.report_sample.txt` (453 bytes) — summary, already shows sample_count=1 for ncgp_setkey.
- `analysis/raw/ncgpa_aes.dump.txt` (8.7 MB) — libncgpa.so probe dump.
- `analysis/raw/ncgpa_aes.report_sample.txt` (187 bytes).
- `analysis/raw/open_aion2_filter3.*` — earlier task 1 baseline.
- Working library binaries on host: `libUnreal_aion2_v2.so`, `libncgm_aion2_v2.so`, `disasm_*.txt`.
- Sibling closing artifact: `/home/sdancer/nmss-emu-magic32-uprobes-via-aion2/analysis/task4_writeside_chain_2026-05-14.md` — has the 7-session login-identifier history (CONCLUSIVE per-session non-determinism) and Google auth callback chain.

## Concrete tasks

### TASK 1 — Extract key bytes from captured event
The fired `ncgp_setkey` event has `x0 = pointer to the 16-byte AES-128 key`. The captured field in tracefs/perf is the POINTER value (u64), NOT the actual key bytes. To read the key, we need to dereference x0 in the process memory **at the time of capture**.

Approach A — if the perf record includes user-space stack dump or surrounding memory: extract from there.
Approach B — re-derive from the lr (caller) site by static analysis: `IFELVckI9JlDYaHOaQJaAv@libncgp.so:0x66f0c` is the AES_set_encrypt_key wrapper. The caller's x0 setup tells us where the key bytes come from (a stack buffer, a static const, etc.).

Steps:
1. Parse `raw/ncgp_aes.dump.txt` to extract the fired ncgp_setkey event's full raw payload.
2. Examine: x0 (key ptr), bits, out (schedule ptr), lr (caller pc).
3. Run `aarch64-linux-gnu-objdump -d --start-address=<lr-128> --stop-address=<lr> libncgp.so` to disassemble the caller setup. Look for instructions that load x0 (e.g., `mov x0, x19` where x19 was set earlier from a structure offset or stack buffer).
4. Document the key sourcing path: is it a 16-byte buffer derived from a network response? A SHA-truncate? A constant?

### TASK 2 — Read login-identifier current value
- Pull `adb shell cat /data/user/0/com.nctaiwan.aion2/files/login-identifier.txt` to get the most recent 16-byte ciphertext. (This is a single shell command, no app interaction.)

### TASK 3 — Cross-pollinate to thered
- Compare libncgp.so `IFELVckI9JlDYaHOaQJaAv` function signature with thered's libUnreal.so AES PCs (`0x195b9f8`, `0x195be04` from prior magic32-disasm). NOTE: aion2 is NCSoft, thered is Netmarble — DIFFERENT publishers, so direct algorithm transfer is unlikely. The structural finding from sibling is what carries: per-session non-determinism applies broadly.
- Don't waste time on transfer if structurally implausible — just note it in the artifact.

### TASK 4 — Write closing artifact
`analysis/aes_walk_2026-05-14.md` with: probe inventory, fired event details, algorithm form (if recovered) OR concrete falsification, transfer feasibility verdict, Rust impl + test (if applicable).

### TASK 5 — Set fact + exit
- `harness fact-set nmss_magic32_per_session_algorithm_recovered "..."` on success
- OR `harness fact-set nmss_magic32_aion2_aes_no_match "..."` on clean falsification
- DO NOT call aeon MCP at any point. DO NOT call openviking MCP.

## Constraints & gotchas
- Memory budget: previous turn hit 23.85 GB / 24 GB cgroup. Codex thread froze. RESTART context — be lean. NO aeon MCP. NO huge file reads (use head/tail/sed/awk to extract relevant slices from 9.2 MB dumps).
- Per-session non-determinism: each fresh sign-in gives a NEW 16-byte token. Captured key/plaintext from the perf event corresponds to THAT specific session's sign-in. To verify AES match, must compare against login-identifier.txt from the SAME session (already on device after the last pm clear).
- adb root + SELinux permissive confirmed.
- This is a SHORT turn (analyze + write). Single pass, exit cleanly.

## Falsification
- ncgp_setkey fires but key/plaintext bytes are stack-allocated and cleared by the time we dereference → cannot read; mark partial-falsified, write descriptive artifact.
- AES(captured_key, captured_plaintext) ≠ login-identifier hex → clean falsification, libncgp.so AES is NOT the producer (could be a different cipher upstream/downstream).
- Algorithm form recovered for aion2 but NCSoft-only → not transferable to thered (Netmarble) → set fact `nmss_magic32_aion2_algorithm_recovered_ncsoft_only`.

## Relevant files / references
- workdir: `/home/sdancer/nmss-emu-magic32-aion2-tracefs-fix/`
- cert-rust-repro: `/home/sdancer/nmss-emu/cert-rust-repro/`
- Sibling: `/home/sdancer/nmss-emu-magic32-uprobes-via-aion2/analysis/task4_writeside_chain_2026-05-14.md`
- 7-session login-identifier inventory (per-session-variant proof): DB5233D4, FD111638, 13173FC4, AE1A565A, 3265092D, 88368244, 60A5DF7A, 5DBC2240.
- Tools: `aarch64-linux-gnu-objdump`, `python3 (cryptography)`, `cargo`. No aeon. No openviking.
