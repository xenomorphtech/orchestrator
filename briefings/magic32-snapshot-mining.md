# magic32-snapshot-mining — Offline mining of trampoline_proc_memdump_5558 for PGS playerId

## Role & workdir
Offline memory analyst. Workdir: `/home/sdancer/nmss-emu-magic32-snapshot-mining` (worktree of `nmss-emu`, branch `magic32-snapshot-mining`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: find the Google PGS player ID (companion to captured MAGIC32 `2FCF997702C244969BFEAF7F0D6AAA1C`) by mining the existing live-process memory snapshots. Pure offline; no device, no Frida.

## Success criteria
Closing fact: `nmss_magic32_numerically_reproduced`. Concrete deliverable:
- (playerId, MAGIC32) ground-truth pair extracted from the snapshot.
- Algorithm derivation `compute_magic32(pgs_player_id) -> [u8; 16]` that produces MAGIC32 from that playerId.
- Pure-Rust impl in `cert-rust-repro` + verification test.

## Progress so far (4 falsified sibling paths)

- `magic32-disasm` (retired-stalled): chain depth 4+; concrete sub-findings preserved but no closure.
- `magic32-apk-strings-sweep` (falsified): AES key NOT a baked static constant.
- `magic32-hw-bp` (falsified): 3 fresh-launch attempts, HW-BP no hit (cached login skips PGS producer).
- `magic32-java-blackbox` (falsified): NMSS blocks Java.use() introspection across all 4 Frida config permutations.

All artifacts under `/home/sdancer/nmss-emu-{magic32-disasm, magic32-apk-strings-sweep, magic32-hw-bp, magic32-java-blackbox}/analysis/`.

## Next 2–3 concrete tasks

1. **Inventory the memdump shards.** Path: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`. Examples from prior work: `78c678d000.bin`, `78c6896000.bin`, `79e50b7000.bin`, `7ac5043000.bin`, `78cd296000.bin`, `76781000.bin`, `12c00000.bin`. Each shard is a contiguous memory region. Identify which shard(s) likely hold the heap regions where Google PGS state lives (typically [anon:dalvik-main-space] or [anon:libc_malloc] — names usually embedded near shard boundaries or in companion .maps files if present).

2. **Search for Google PGS player ID candidates.** Format heuristic: PGS opaque player IDs are typically `g<18-20 digit decimal>` ASCII strings (e.g., `g12345678901234567890`). Across shards:
   - `rg -ao 'g[0-9]{17,22}'` over each shard (raw byte search — playerId is ASCII).
   - For each candidate, note its file offset + 64-byte context.
   - Filter against the captured MAGIC32 = `2FCF997702C244969BFEAF7F0D6AAA1C` — try AES-128-ECB encrypt of each candidate (padded with various standard padding schemes) under candidate keys derived from common KDFs.

3. **Algorithm search.** For each plausible playerId candidate, try these AES key derivations and check if the result matches MAGIC32:
   - `key = sha256(playerId)[:16]`
   - `key = sha256("PGSClientSecret_value" || playerId)[:16]` — substitute candidate PGSClientSecret strings found in libUnreal.so rodata (use the prior magic32-apk-strings-sweep artifact for candidate constants near PGS-related strings).
   - `key = HKDF-SHA256(salt=playerId, ikm=PGSClientSecret_candidates, info='MAGIC32')[:16]`
   - `key = device_id_hash` patterns.
   - Plaintext padding: try zero-pad-to-16, PKCS#7 to 16, MD5(playerId)[:16] as plaintext, raw ASCII truncate-to-16.
   - **Verification**: AES-128-ECB(key, plaintext) → hex_upper compared to MAGIC32.

4. **Write deliverable.** If a match is found: write `compute_magic32(player_id_bytes: &[u8]) -> [u8; 16]` in `cert-rust-repro`, test against the captured pair, set fact `nmss_magic32_numerically_reproduced`.

## Constraints & gotchas

- **Snapshot path verification**: First run `ls /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/` to confirm shards still present. If missing, check `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/` (mentioned in WIKI) — may need scp or rsync.
- **No device, no Frida**: pure file-based analysis. Avoids anticheat entirely.
- Some shards are very large (>100 MB). Stream-process; don't fully load into memory.
- The captured session was 2026-04-29 — playerId may not be in heap if it was already GC'd or in a different process state.
- This worker runs under systemd `harness-worker@magic32-snapshot-mining.service` (system.slice, MemoryMax=24G).

## Relevant files / references

- Memdump root: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`.
- Captured MAGIC32: `2FCF997702C244969BFEAF7F0D6AAA1C` (16 bytes = `0x2FCF9977 02C24496 9BFEAF7F 0D6AAA1C`).
- libUnreal.so rodata candidates from apk-sweep: see `/home/sdancer/nmss-emu-magic32-apk-strings-sweep/analysis/task2_broader_sweep_2026-05-14.md` for the 14 high-entropy 16-byte blocks.
- Prior PGS string inventory: `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md` (lists `PGSClientSecret`, `PGSClientId`, etc.).
- Tools: `rg`, `python3 (cryptography)`, `cargo (aes crate)`, `xxd`.

## Falsification

- 3 cycles: no plausible Google-PGS-formatted playerId in any shard, OR ≥10 candidates tested via ≥1000-key-permutation sweep and no match found.
- In that case: mark goal `nmss_magic32_numerical_repro` as `stalled-meta` — every path has been falsified; escalate to user for resource ask (e.g., manual playerId capture on a fresh device with anti-anticheat tooling beyond xerda).
