# cert-frag1-sp968-port (H-N14) — derive Phase 1 (challenge → sp+0x968 64-byte block) using existing cert-rust-repro scaffolding

**You ARE allowed and expected to write code.** Rust (cert-rust-repro), Python (capture analysis).

## Role & workdir

The cert algorithm is **mostly solved** in `/home/sdancer/nmss-emu/cert-rust-repro/`. Phase 2 (`hex_upper(sha256_compress_single_block(sp+0x968)[4..28])`) is verified end-to-end. **The only unsolved part is Phase 1: how the 64-byte `sp+0x968` block is built from the challenge.** Your mission: capture (challenge, 64-byte block) tuples + the 13-record schedule + frag1 lane inputs via HW-BPs at the actual cert pipeline PCs, derive the deterministic transform, replace the `ground_truth_cert_upper48()` oracle bypass in `stage_05` with pure-algorithm Rust, validate 5/5. **Workdir**: `/home/sdancer/nmss-emu-cert-frag1-sp968-port/` (create with `git worktree add`).

This is the campaign-closing path. Don't get lost in re-RE — leverage what's already there.

## Why this path

H-N13 stop case (c) findings (see `/home/sdancer/nmss-emu-cert-formatter-bp/analysis/cert_formatter_bp_blocker.md`):

- The formatter chain at `0x78c686a9a8 / 0x78c689528c / 0x78c689575c` is **challenge-invariant** (libcurl HTTP-header constants). NOT the cert pipeline.
- The cert algorithm is `hex_upper(sha256_compress_single_block(sp+0x968)[4..28])`.
- Phase 2 (compress+slice+hex) is **verified** in `cert-rust-repro/tests/cert_end_to_end_simulator_verification.rs`.
- Phase 1 (challenge → 64-byte sp+0x968 block) is **unsolved**. `stage_05` uses `ground_truth_cert_upper48()` to bypass.
- Recurrence (per `stage_c_frag2_fold.rs`): `raw32[i+1] = SHA256(record_ascii_64[i] || raw32[i])` over **13 productive 0x140-stride rows**. The visible 64-char frag2 string is the uppercase hex of the final raw32.
- Real BPs in the runtime cert path:
  - `0x78c690e660` — frag2 fold entry
  - `0x78c6912388` — frag2 fold exit
  - `0x78c69163bc` — frag2 visible string writer exit (produces `frag2_upper64`)

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5).
- **Sub-goal:** Derive (a) the 13 `record_ascii_64[i]` records and (b) the frag1 lane inputs as deterministic functions of the 16-char challenge ASCII.

## Success criteria

- **Minimum**: 5/5 captures at `0x78c690e660`, `0x78c6912388`, `0x78c69163bc`, and the frag1 producer site. Save `analysis/frag_pipeline_captures_2026-05-12.jsonl`. Identify which fields vary per challenge and which are constants.
- **Stretch**: Derive `fn records_from_challenge(c: &[u8;16]) -> [[u8;64]; 13]` and `fn frag1_inputs_from_challenge(c: &[u8;16]) -> ...`. Plug both into `cert-rust-repro` replacing the oracle bypass. Run `cargo test phase_d_e2e_5vec_checkpoint` (or equivalent) and confirm pass WITHOUT oracle.
- **Campaign close**: 5/5 pure-Rust → set fact `nmss_cert_5_5_pure_rust_reproduced` + escalate to user.

## Existing scaffolding to leverage (READ FIRST)

`/home/sdancer/nmss-emu/cert-rust-repro/`:
- `src/lib.rs::frag1_words_from_materializer` — formal derivation hooks for frag1
- `src/native_oracle/chain.rs` — stage chain composition
- `src/native_oracle/phase_d.rs` — phase D entry point + 5-vec test fixture
- `src/native_oracle/stages/stage_05_78c686b068_to_78c68e2b68.rs` — **contains the `ground_truth_cert_upper48()` oracle bypass that you need to replace**
- `src/native_oracle/stages/stage_c_frag2_fold.rs` — frag2 fold (the SHA-256 recurrence over 13 records)
- `src/native_oracle/stages/stage_channel_a_sp968.rs` — **the sp+0x968 channel** (likely the place to plug in Phase 1)
- `src/native_oracle/stages/stage_cert_composer.rs` — final cert assembly
- `src/native_oracle/stages/stage_frag1_row_bank.rs` — frag1 row bank
- `src/native_oracle/stages/slot_8_wrapper_xxhash.rs`, `slot_9_direct_xxhash.rs`, `slot_10_11_branchy.rs`, `slot_12_string_aes.rs` — candidate per-record transform primitives
- `src/native_oracle/stages/frag2_vector_base_seed_dump.rs` — frag2 seed dump (the initial raw32 state)
- `tests/cert_end_to_end_simulator_verification.rs` — Phase 2 verification (passes)
- `tests/phase_d_nmss_asset_pipeline_test.rs` — phase D 5-vec test

## Next 3 ordered tasks

1. **READ cert-rust-repro deeply.** Understand the existing chain, what's solved, what's stubbed, where the oracle bypass is, and what data structures the existing code expects for the 13 records and frag1 lanes. Map every "stub / placeholder / oracle / TODO" — that's your TODO list.

2. **Capture BPs in the runtime cert path.** Patched `native-replay-rs --trace-call-hw` is on `root@162.244.80.97`. Set BPs at:
   - `0x78c690e660` (frag2 fold entry) — capture x0..x8 + dump x0/x1/x2 contents (64-128B each). The 13 records and raw32 seed should be visible.
   - Inside the frag2 fold loop (find the per-iteration PC from disasm in cert-rust-repro's stage_c_frag2_fold.rs) — capture each (record_ascii_64[i], raw32_before[i]) tuple.
   - `0x78c6912388` (frag2 fold exit) — capture final raw32[13].
   - `0x78c69163bc` (frag2 visible string writer exit) — confirm `frag2_upper64`.
   - The frag1 producer site — find from `stage_05_real_algorithm.rs` and `slot2_pool_to_mixer.rs` references; if uncertain, BP at slot_9 entry (xxhash32 path) as the most likely producer.
   
   Run for all 5 challenges; save raw captures to `analysis/raw_captures/frag_<challenge>.json`.

3. **Diff + port.** Cross-challenge diff: which bytes of the 13 records vary, which are constant? How do challenge bytes map into record bytes? Most likely candidates:
   - Records contain challenge ASCII literally at some offset
   - Records are challenge + counter + constant secret
   - Records are challenge-keyed xxhash32 output filling the 64-byte slot
   
   Implement `records_from_challenge(&[u8;16]) -> [[u8;64]; 13]` in Rust. Plug into `stage_05` replacing `ground_truth_cert_upper48()`. Run the 5-vec test. If 5/5 → **CAMPAIGN COMPLETE**. Set facts: `cert_phase1_records_recovered_2026_05_12` + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.

## Constraints & gotchas

- **No git commits.**
- **Run all replay/capture on remote `root@162.244.80.97`**; rsync results back to local workdir.
- **DO NOT modify the unicorn replay**. Use native-replay-rs only — the unicorn one has a frozen-challenge bug.
- **The "oracle bypass" in stage_05** is the integrity hint. Search for `ground_truth_cert_upper48` and trace what callers expect; that's your interface contract for Phase 1.
- **The 13 records might have CRC32 or xxhash32 components** (worker H-N12 saw `crc32cx` insns in disasm; slot_8_wrapper_xxhash.rs and slot_9_direct_xxhash.rs are co-located). Try xxhash32(challenge_keyed_seed_i) for record_i first; if not, escalate.
- **All prior path artifacts are still useful**: H-N11's 30 BP records (`/home/sdancer/nmss-emu-neon-primitive-bp/analysis/`), H-N13's formatter captures (`/home/sdancer/nmss-emu-cert-formatter-bp/analysis/`). Don't re-capture if existing captures suffice.

## Relevant files / references

- **H-N13 blocker (read first)**: `/home/sdancer/nmss-emu-cert-formatter-bp/analysis/cert_formatter_bp_blocker.md`
- **H-N12 disasm**: `/home/sdancer/nmss-emu-cert-vtable-port/analysis/{vtable_0x224208_disasm.txt, jit_cert_producer_disasm.txt, cert_func_window_disasm.txt, vtable_decomp_notes.md}`
- **cert-rust-repro**: `/home/sdancer/nmss-emu/cert-rust-repro/` — read this thoroughly first
- **Ground truth (5 chal→cert pairs)**:
  - `0000000000000000 → 4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47`
  - `0123456789ABCDEF → 3083EFBA67F78ADBBD44EA727AF751E1F7380A33CC78B030`
  - `1111111111111111 → 3933F258F2EDF72D3CDD7E31D075D1F34AE73DF72D60A16B`
  - `7BDA93D2F45D36C0 → 90237F0E03DF6993A54669AA7CF27E36304273143AD6A030`
  - `AABBCCDDEEFF0011 → 8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`
- **Patched native-replay-rs**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` (and source under `src/`)

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_frag1_sp968_port_progress_2026-05-12.jsonl`. Stages: `scaffolding_mapped`, `bp_captures_5x`, `records_decomposed`, `phase1_rust_drafted`, `oracle_bypass_removed`, `5_of_5_match_CAMPAIGN_COMPLETE`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) 5/5 pure-Rust match → **CAMPAIGN COMPLETE**. Set facts `cert_phase1_records_recovered_2026_05_12` + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.
- (b) Records derived but lanes incomplete → set partial fact, document gap.
- (c) Records show no recognizable transform pattern → write blocker with the empirical (challenge, record_i) table for follow-up analysis. Propose deeper BPs upstream of `0x78c690e660`.
- (d) Pre-existing cert-rust-repro framework has structural blockers preventing Phase 1 plug-in (e.g. tightly coupled to oracle) → write blocker enumerating the structural changes needed.
