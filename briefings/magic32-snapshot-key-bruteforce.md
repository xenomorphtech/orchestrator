# magic32-snapshot-key-bruteforce — 16-byte sliding-window AES brute-force across snapshot heap

## Role & workdir
Offline cryptanalyst. Workdir: `/home/sdancer/nmss-emu-magic32-snapshot-key-bruteforce` (worktree of `/home/sdancer/nmss-emu`, branch `magic32-snapshot-key-bruteforce`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: the AES KEY for MAGIC32 may still be resident in heap even though the plaintext playerId was zeroed. **For each 16-byte window in heap-class shards, treat as candidate AES key**; AES-encrypt small set of plausible plaintexts; if any output equals MAGIC32, the key + plaintext are both recovered.

## Why this is distinct from the falsified snapshot-mining + snapshot-regrep paths

- `magic32-snapshot-mining`: searched for `g<digits>` regex. FALSIFIED.
- `magic32-snapshot-regrep`: searched for `a_<digits>` regex. FALSIFIED (per `/home/sdancer/nmss-emu-magic32-snapshot-regrep/analysis/snapshot_regrep_2026-05-14.md`).
- Both prior paths searched for the PLAINTEXT input (playerId) which was zeroed after AES. **This path searches for the KEY**, which may persist in heap because it's typically held in a key-schedule struct that's NOT zeroed until program exit.

## Cross-pollination facts
- `magic32_simple_derivation_falsified_2026_05_14`: 22272 simple AES derivations against ktion23's playerId tested with 0 matches → derivation is complex, key is likely NOT a hash of public inputs alone.
- `nmss_magic32_numerically_reproduced` (target).
- MAGIC32 ciphertext: `2FCF997702C244969BFEAF7F0D6AAA1C`.

## Success criteria
Concrete deliverable:
- A 16-byte key that, when used with AES-128-ECB or AES-128-CBC zero-IV against ONE of ~50 plausible plaintext forms, produces `2FCF997702C244969BFEAF7F0D6AAA1C`.
- The KEY's hex value + the (shard, byte_offset) where it was found + the surrounding 64 bytes of context (typically reveals the key-schedule struct or PGSClientSecret derived buffer).
- Pure-Rust `compute_magic32(player_id_str)` implementation in `cert-rust-repro/src/magic32.rs`.
- Fact `nmss_magic32_numerically_reproduced` set.

## Next concrete tasks (single long turn, 3h budget)

1. **Set up plausible plaintext space.** From sibling artifacts + gms-state-read, the plaintexts to try are derived from the playerId. Since the snapshot's playerId is unknown but is some `a_<digits>` form (per modern PGS), try a parameterized family:
   - utf8 of various `a_<19-22-digit>` patterns (we can NOT enumerate all possible playerIds; instead, use these GENERIC forms that don't depend on which playerId):
     - 16 zero-pad bytes
     - All-zero, all-0xFF, and other constants (in case it's a per-app secret encrypted with a static plaintext)
     - The literal bytes `a_` + 14 ASCII bytes (covers any short variant)
   - Also: try AES applied INVERSELY — for each candidate key, decrypt MAGIC32 and check if result looks like a playerId (starts with `a_`, `g`, contains decimal digits, ASCII printable).
   - The INVERSE direction is more useful: instead of trying all playerIds, try all KEYS — each candidate key decrypts MAGIC32 to a unique 16-byte plaintext. If the plaintext is interpretable (ASCII digits / `a_<...>`), we have a hit. Decryption is FAST and self-validating.

2. **Implement the brute force.**
   - Top 3 heap-class shards from prior task 1 inventory: `78ed96e000.bin` (521 MB), `76781000.bin` (512 MB Dalvik LOS), `12c00000.bin` (384 MB Dalvik main).
   - Total: 1.4 GB. With 16-byte sliding window stride=1: 1.4 × 10^9 candidate keys.
   - For each candidate window: AES-128-ECB-decrypt MAGIC32, check if plaintext is "interesting" (defined below). If interesting, log.
   - "Interesting" plaintext criteria:
     - All bytes ASCII printable (0x20–0x7E)
     - First byte is `a` (0x61) AND second is `_` (0x5F)
     - First byte is `g` (0x67) AND next 15 are ASCII digits
     - Last 8+ bytes are ASCII digits
     - Plaintext entropy < 6 bits/byte AND length ≥ 8 printable
   - Tools: Rust `aes` crate at the speed of x86-aesni (modern Ryzen ≈ 500M AES-decrypts/sec). 1.4B candidates → ~3 seconds raw AES, but 1-byte stride means RAM-bound at ~30 MB/s read → ~50 seconds per pass over 1.4 GB. Manageable.

3. **Rust implementation (recommended) for the sweep:**
   ```rust
   use aes::cipher::{KeyInit, BlockDecrypt, generic_array::GenericArray};
   use aes::Aes128;
   use memmap2::Mmap;
   use std::fs::File;

   fn is_interesting(pt: &[u8; 16]) -> bool {
       let printable = pt.iter().filter(|&&b| (0x20..=0x7E).contains(&b)).count();
       if printable >= 14 {
           // Strong: mostly printable
           if pt[0] == b'a' && pt[1] == b'_' { return true; }
           if pt[0] == b'g' && pt[1..].iter().all(|&b| b.is_ascii_digit()) { return true; }
           if pt.iter().filter(|&&b| b.is_ascii_digit()).count() >= 10 { return true; }
       }
       false
   }

   fn sweep_shard(path: &str, magic32: &[u8; 16]) -> Vec<(usize, [u8; 16], [u8; 16])> {
       let f = File::open(path).unwrap();
       let mmap = unsafe { Mmap::map(&f).unwrap() };
       let mut hits = Vec::new();
       let ct = GenericArray::from_slice(magic32);
       for off in 0..mmap.len().saturating_sub(16) {
           let key = &mmap[off..off+16];
           let cipher = Aes128::new(GenericArray::from_slice(key));
           let mut block = *ct;
           cipher.decrypt_block(&mut block);
           let pt: [u8; 16] = block.into();
           if is_interesting(&pt) {
               let mut k = [0u8; 16];
               k.copy_from_slice(key);
               hits.push((off, k, pt));
           }
       }
       hits
   }
   ```

4. **Validate hits.**
   - For each hit, verify: AES-128-ECB-encrypt(key, plaintext) == MAGIC32. (Round-trip check.)
   - Cross-check: is the key located near a string like "PGSClientSecret" / "AES" / "key" / "magic32" in the surrounding 256 bytes? If so, it's the genuine key-schedule struct.
   - Cross-check: does the plaintext look like a PGS playerId format we expect? If yes, set fact.

5. **Also try AES-128-CBC zero-IV** in parallel — Hive idiom suggests Korean SDKs love zero-IV CBC. Same window approach.

6. **Also try AES-128-ECB with KEY interpreted as the AES key-schedule (round keys)** — sometimes the heap has the expanded key schedule (176 bytes for AES-128) rather than the raw 16-byte key. In that case, the first 16 bytes of the schedule are the raw key. Same approach works.

7. **If a hit is found**: implement `compute_magic32(player_id_str: &str) -> [u8; 16]` in `cert-rust-repro/src/magic32.rs`. The function needs:
   - The derivation function from playerId to key (which we now have evidence for: the recovered key in heap will let us reverse-engineer it).
   - The plaintext encoding.
   - Test against the snapshot's MAGIC32 + any (key, plaintext) pair we recover.
   - `cargo test`.
   - Set fact `nmss_magic32_numerically_reproduced`.

8. **Write artifact** `analysis/key_bruteforce_2026-05-14.md`:
   - Number of candidate windows swept.
   - Number of "interesting" hits.
   - For each hit: (shard, offset, key_hex, decrypted_plaintext_hex+ASCII, surrounding-context-hex).
   - Rust impl + test result.
   - Verdict.

## Constraints & gotchas

- 1.4 GB heap × 1-byte stride = ~1.4 billion candidate keys. With Rust + AES-NI: ~30-50 sec per shard. Use rayon for parallelism across shards.
- "Interesting" filter must NOT be too loose (we'd get millions of false-positive Latin-1 noise) NOR too tight (we'd miss valid hits if the playerId format is unusual).
- The original snapshot was captured on a device with Frida-agent loaded (per `/memfd:frida-agent-64.so` in maps) → may be from a developer device, may have unusual state.
- This worker runs under systemd `harness-worker@magic32-snapshot-key-bruteforce.service` (system.slice, MemoryMax=24G).

## Relevant files / references
- Snapshot memdump: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`.
- Top 3 heap-class shards (from snapshot-regrep task 1):
  - `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78ed96e000.bin` (521 MB)
  - `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/76781000.bin` (512 MB)
  - `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/12c00000.bin` (384 MB)
- Captured MAGIC32: `2FCF997702C244969BFEAF7F0D6AAA1C`.
- cert-rust-repro at `/home/sdancer/nmss-emu/cert-rust-repro/` — add module `src/magic32.rs`.
- Tools: Rust `aes` crate, `memmap2` crate, `rayon` for parallelism, `cargo`.

## Falsification

- All 3 top heap shards swept, no candidate window decrypts MAGIC32 to interesting plaintext.
- Expand to all 6 heap shards (+1 GiB ish). If still no hits → the key was zeroed too, in which case the producer's allocator zeros on free aggressively (libc malloc + glibc tcache eager zero), AND the key was on stack only.
- In that case: escalate to `magic32-kernel-hide-frida-on-nmss` (fix PGS Status=10 via kernel module on NMSS app to enable live capture).
