# cert-body1-hashfn (H-N16) — recover body_1 seed + identify prefix-select hash function

**You ARE allowed and expected to write code.** Rust (cert-rust-repro), Python (capture + hash-fit), shell (remote BPs).

## Role & workdir

The cert algorithm is now **structurally 95% recovered**. H-N15 verified `cluster1_inner_32 = SHA256(prefix_5 || SHA256(prefix_4 || ... || SHA256(prefix_1 || body_1)))` end-to-end for chal=0; pure-Rust impl `cluster1_inner_32_from_chain` lives in `cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` with a passing unit test. Your sole mission: recover the **2 remaining unknowns** to make the cert producer fully algorithmic.

**Workdir**: `/home/sdancer/nmss-emu-cert-body1-hashfn/` (create with `git worktree add`).

This is the FINAL path. Two narrow targets, each fully specified.

## Why this path

H-N15 stop case (b) (see `/home/sdancer/nmss-emu-cert-cluster1-32B-derivation/analysis/cluster1_32B_structural_findings.md`):

- The 5-SHA chain structure is **algebraically verified** for chal=0 with prefix sequence `[94E134E2, C6D521A7, 94E134E2, D3B03299, D3B03299]` + body_1 = `74b784fd...e3b841` → cluster1_inner_32 = `9dc8f2f888e350f6...575ca17e`.
- Prefixes come from a 1024-byte static ASCII table at heap `x21 = 0xb400007ab51773c0`, via a 13-entry byte-offset table at sp+0x72c, selected by `hash(prev) mod 13`.
- **MD5(challenge_ASCII) = x0 buffer at first hit** (matches all 5 chals — challenge entry point).
- **Body_1 seed** is the output of upstream SHA-256 chain at PCs `0x78c68b2388` / `0x78c68b6410`.
- **Prefix-select hash function** is at PC `0x78c67e2030` — file offset `0x55030` in `78c678d000.bin`. Failed: MurmurHash2-64A, CityHash64/32, FNV-1/-1a-64, djb2, sdbm, SHA-256[:8], MD5[:8], CRC32. Likely a custom `std::hash<std::string>` libc++ specialization with a mixing constant.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5).
- **Sub-goal A:** Identify `fn body_1_from_challenge(c: &[u8; 16]) -> [u8; 32]`.
- **Sub-goal B:** Identify `fn prefix_select_hash(input: &[u8]) -> u64` (such that `hash mod 13` selects the table13 byte-offset).

## Success criteria

- **Minimum**: Captures at PCs `0x78c68b2388` + `0x78c68b6410` across all 5 challenges, showing the upstream SHA chain that builds body_1. Disasm of file-offset 0x55030 function (with the mixing constants extracted).
- **Stretch**: Both pieces implemented in Rust; passing test reproducing `cluster1_inner_32` for all 5 challenges without the fixture.
- **Campaign close**: Both pieces fully wired → `cluster1_inner_32_fixture_for_challenge` removed → `cargo test phase_d_e2e_5vec_checkpoint` passes 5/5 → set facts `cert_body1_derivation_recovered_2026_05_12` + `cert_prefix_select_hash_identified_2026_05_12` + `nmss_cert_5_5_pure_rust_reproduced` + escalate to user as **CAMPAIGN COMPLETE**.

## Concrete tasks (ordered)

1. **Disasm 0x55030 function** in `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/memdump/78c678d000.bin`. Use:
   ```
   ssh root@162.244.80.97 aarch64-linux-gnu-objdump -D -b binary -m aarch64 --start-address=0x55030 --stop-address=0x55400 /root/nmss-emu-trampoline/trampoline_proc_memdump_5558/memdump/78c678d000.bin
   ```
   Look for: load of magic constants (eor, mul with primes), shift+mix pattern, finalizer. Common libc++ std::hash<std::string>: MurmurHash2 64-bit with seed `0xc70f6907UL`. Common alternatives: a custom polynomial hash. **Save disasm to `analysis/hashfn_0x55030_disasm.txt`** and write up your interpretation in `analysis/hashfn_decomp.md`.

2. **HW-BP capture** at PCs `0x78c68b2388` and `0x78c68b6410` for all 5 challenges to capture body_1 input/output:
   - Use patched `native-replay-rs --trace-call-hw` on remote
   - Dump x0/x1/x2/x29 registers + sp+0x968 (32B SHA state) + sp+0x7B0 (cluster1_inner_32 buffer where body_1 lands)
   - Save to `analysis/body1_captures_5x.jsonl`
   - Cross-challenge diff: identify how challenge bytes flow into body_1

3. **Port both pieces to Rust + remove fixture**. Modify `cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs`:
   - Implement `fn prefix_select_hash(input: &[u8]) -> u64` per the disasm at 0x55030
   - Implement `fn body_1_from_challenge(c: &[u8; 16]) -> [u8; 32]` per the captured upstream SHA chain
   - Wire them: `cluster1_inner_32(c) = chain_with_prefixes_selected_by(body_1_from_challenge(c), prefix_select_hash)`
   - DELETE `cluster1_inner_32_fixture_for_challenge`
   - Run `cargo test phase_d_e2e_5vec_checkpoint` — must pass 5/5 via pure algorithm
   - **Save before/after of the source** so the diff is auditable

4. **If 5/5 pure-Rust → CAMPAIGN COMPLETE.** Set facts + escalate.

## Constraints & gotchas

- **No git commits.**
- **Module 78c678d000.bin contains the libc++/runtime helpers** for the cipher blob 9781e236. The hash function is likely `std::hash<std::string>` libc++ specialization. Look for the libc++ signature: load `0xc70f6907` (32-bit seed) or `0xc6a4a7935bd1e995` (64-bit Murmur prime).
- **The 13-entry byte-offset table at sp+0x72c**: this is a stack-local copy of a small lookup that maps `hash mod 13` → 1-byte offset into the 1024-byte ASCII table. H-N15 saw values like 0x080, 0x0af, 0x145, 0x1a6, 0x21a, 0x253, 0x2ab, 0x2ac at various x19 indices. Could be a permutation of 13 specific byte offsets pre-computed at runtime — capture this table at sp+0x72c entry.
- **Body_1 = output of 2 earlier SHA-256s** (per H-N15: PC 0x78c68b2388 + 0x78c68b6410). May itself be a SHA-chain of MD5(chal) + magic. Capture both PCs' input + output.
- **Run all replays on remote `root@162.244.80.97`**; rsync results back.
- **Use existing 5/5 fixture as ground-truth** for hypothesis verification.

## Relevant files / references

- **H-N15 final findings (read first)**: `/home/sdancer/nmss-emu-cert-cluster1-32B-derivation/analysis/cluster1_32B_structural_findings.md`
- **H-N15 hypothesis tests**: `/home/sdancer/nmss-emu-cert-cluster1-32B-derivation/analysis/hypothesis_tests.py`
- **H-N15 captures (remote)**: `root@162.244.80.97:/tmp/sp940_caps/`, `/tmp/sp940_final_caps/`, `/tmp/cluster1_load_caps/`, `/tmp/sp7b0_writes/`
- **Pure-Rust chain impl (already-done; extend, don't rewrite)**: `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs`
- **Module shard with hash fn**: `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/memdump/78c678d000.bin` (file offset 0x55030)
- **Ground truth (5 chal→cert pairs)**: e.g. `0000000000000000 → 4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47`
- **Patched native-replay-rs**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_body1_hashfn_progress_2026-05-12.jsonl`. Stages: `hashfn_disasm_done`, `body1_captures_5x`, `hashfn_identified`, `body1_derivation_decoded`, `pure_rust_5_of_5_CAMPAIGN_COMPLETE`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) 5/5 pure-Rust match (no fixture) → **CAMPAIGN COMPLETE**. Set facts `cert_body1_derivation_recovered_2026_05_12` + `cert_prefix_select_hash_identified_2026_05_12` + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.
- (b) Both pieces identified algorithmically but Rust port at <5/5 → set partial facts, document residual.
- (c) Hash function disasm yields a custom non-standard mixer that's NOT trivially portable → write blocker with the full disasm + algebraic interpretation.
- (d) Body_1 captures show a deeper recursion that resists same-cycle decode → write blocker with the captured upstream-chain structure for follow-up.
