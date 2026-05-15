# cert-rust-reimpl — Rust reproducer for NMSS cert algorithm

## Role & workdir
You own the Rust reproducer at `/home/sdancer/nmss-emu/cert-rust-repro/`. Mission: build a 100% source-code reproducer of `frag1(challenge) → 48-char cert` that genuinely computes the cert from the algorithm (not a lookup table).

## Current goal / sub-goal
- **Goal:** `nmss_cert_replay_correct_pure_algo` — pure-Rust function reproducing all 5 ground-truth vectors algorithmically.
- **Sub-goal in progress:** late fragment chain port — step 3 (RE the sp+0x40c writer inside `0x78c6904ebc`).

## Ground truth (5 vectors) — TRIANGULATED 2026-05-02

These vectors are now confirmed by **both** the static-replay path **and** a live `nmssNativeGetCertValue` Java-API capture in cert-ptrace's working Frida-CLI native-service lane (cycle 730→731). The earlier "live=3763E965 fluke" reading is dead — that was an unauthenticated/intermediate state. **The snapshot witnesses are the real targets — resume porting confidently.**

`analysis/test_vectors_2026-04-24/summary.json` (snapshot RE) ===
`analysis/checkpoints/native_cert_<challenge>_clean_session_2026-05-02.json` (live JNI capture):

- `0000000000000000` → `4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47`
- `0123456789ABCDEF` → `3083EFBA67F78ADBBD44EA727AF751E1F7380A33CC78B030`
- `1111111111111111` → `3933F258F2EDF72D3CDD7E31D075D1F34AE73DF72D60A16B`
- `7BDA93D2F45D36C0` → `90237F0E03DF6993A54669AA7CF27E36304273143AD6A030`
- `AABBCCDDEEFF0011` → `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`

Aggregate live capture: `analysis/checkpoints/native_cert_all5_clean_session_2026-05-02.json`
Native-service lane writeup: `/home/sdancer/orchestrator/campaign-index/findings/paths/native-service-end-to-end.md`

## Progress so far (key milestones)

**Stage 05 front-half algorithm RECOVERED** (2026-04-30):
- `derive_real_stage05_front_half_from_challenge` / `Stage05RealFrontHalf` ported in `cert-rust-repro/src/native_oracle/stages/stage_05_78c686b068_to_78c68e2b68.rs`.
- Algorithm: `challenge ASCII upper16 → 0x4041 wrapped staging → slot-2 4-lane xxHash`.
- Per-challenge slot-2 outputs computed (NOT cert prefixes — confirmed gap):
  - `0x0` → `c14bdf410bfd1cb5ad073a73add02740`
  - `0x0123456789ABCDEF` → `6497c2ec5f30f56e32b69f15e74263ed`
  - `0x1111111111111111` → `986330311e95eff90eaf005e66363cbf`
  - `0x7BDA93D2F45D36C0` → `d73fa5b86817b8e902a410615a7d34da`
  - `0xAABBCCDDEEFF0011` → `690f62fd86821db0a85fe100418ab364`
- Checkpoint: `analysis/checkpoints/cert_phase_c_stage_05_real_algorithm_2026-04-30.json`
- Test: `cert-rust-repro/tests/phase_c_stage_05_real_algorithm.rs`

**Late fragment chain plan published** (`cert_stage_05_late_fragment_chain_plan_2026-04-30.json`):
1. ✅ Port `0x78c68f2a68` joiner — DONE (see below)
2. Port `0x78c6917a08` sp+0x3e8 32-hex fragment producer
3. **CURRENT STEP:** RE the exact sp+0x40c writer inside `0x78c6904ebc`
4. Reduce x21 → message buffer → final cert body path
5. Defer `0x78c67f7a50` AES corridor (it's NOT the cert reducer — it's the AES pack/copy corridor with key/IV via `0x78c67e3a08` and encrypt via `0x78c67e386c`)

**Step 1 (joiner) complete** (2026-04-30):
- File: `cert-rust-repro/src/native_oracle/late_fragment_join.rs`
- Test: `cert-rust-repro/tests/phase_c_late_fragment_join_validate.rs`
- Checkpoint: `analysis/checkpoints/cert_phase_c_late_fragment_join_2026-04-30.json`
- Semantics: copy fragment 1 from sp+0x3e8, append fragment 2 from sp+0x40c over fragment 1's NUL terminator.
- 5/5 structural validation at root anchor `0x78c6917a08` (where sp+0x3e8 is empty so output = fragment 2).
- Captured challenge-distinct sp+0x40c values:
  - `0x0` → `F84F3D06932E248A8CEAF20DCC7900F685A58140613D387B310C8EA66A0AD5B4`
  - `0x0123456789ABCDEF` → `D8F3E2A9B457BD3160F951A076F38A7CDA7297BB4954115802CD53F1ABCD707D`
  - `0x1111111111111111` → `BD9DD100CEE3427B0624A71245AB86F823D608E5823AAFCCAE2B9114B3F87D1A`
  - `0x7BDA93D2F45D36C0` → `BECA86489D2D6F7E305BEF0116BAEF2FDC4FE34B035E8D1F1D880D6D8C6F42D5`
  - `0xAABBCCDDEEFF0011` → `9107917EA3A3FC896EF5679AD4274BDE2CCA22FAC4E6451CEA48637A8785AC18`

**Key structural finding (RE):**
- sp+0x40c is the **payload view of a libcxx-style string object rooted at sp+0x408** (NOT a raw x25 buffer write).
- Data flows through `0x78c67f8e48` materializer calls inside `0x78c6904ebc`.
- Fact: `cert_sp_0x40c_is_string_payload_2026_04_30`.

**Operational lookup table still in place** (5/5 match): `expected_cert_upper48()` in `stage_05_78c686b068_to_78c68e2b68.rs`. Cannot be removed until the late fragment chain is fully ported.

## Algorithm pieces fully decoded (2026-04-30, post-compaction state)

- **Stage 05 front-half:** `derive_real_stage05_front_half_from_challenge` — 5/5 verified.
- **sp+0x40c writer:** pinned at `0x78c6916238..0x78c69163a8` (`cert_phase_c_sp_0x40c_writer_2026-04-30.json`, fact `cert_phase_c_sp_0x40c_writer_pinned_2026_04_30`).
- **sp+0xa88 fold:** decoded as `raw32[i+1] = sha256(record_ascii_64[i] || raw32[i])` over a 0x140-stride 13-record vector (`cert_phase_c_sp_0xa88_fold_producer_2026-04-30.json`).
- **Iteration terminator:** NO post-SHA cryptographic transform — final raw32 is hex-encoded to 64 lowercase chars + NUL + bionic toupper. Path: `0x78c6916380..0x78c69163e0`. Checkpoint: `cert_late_chain_iteration_terminator_2026-04-30.json`. Fact: `cert_late_chain_iteration_terminator_disasm_2026_04_30`.
- **Case-normalizer (`0x78c67e1a90`):** PLT stub to bionic libc `toupper` at `0x7c4526366c`. Rule: `(c>=0x61 && c<=0x7a) ? (c^0x20) : c`. Only ASCII a-z become A-Z. Checkpoint: `cert_case_normalization_helper_0x78c67e1a90_2026-04-30.json`. Fact: `cert_case_normalization_helper_disasm_2026_04_30`.
- **Per-challenge late vector captures:** ALL 5 captured at `analysis/checkpoints/task4_callback_bypass_heads_copy_release_stop_2026-04-30/<CHAL>/{frag2_record_iter_head_0x78c690e660.json,frag2_record_copy_call_0x78c690e718.json}`. Aggregate: `cert_late_vector_span_all_5_complete_2026-04-30.json`. Recipe: release + `--skip-cff-jump-to-callback` + `AEON_STOP_AFTER_EXECUTED_BLOCKS=1700000` + extra PCs `frag2_record_iter_head:0x78c690e660,frag2_record_copy_call:0x78c690e718` + `MAX_HITS=4`. SEGV-after-stop is benign.

## Open blocker (RESUME HERE)

**The captured iter_head vector_span at `0x78c690e660` for 7BDA decodes to the EARLY 32-char record family (`len=0x21`, `cap=0x30`), NOT the productive late 64-char family (`len=0x41`, `cap=0x50`) that feeds the sha256 fold.**

End-to-end fold attempt produced `efc4d814fe098622bf56c85ee540e06dd08632f8db35983da015572bd30b3feb` for 0000-challenge (expected `4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47`) — does not match because we're folding the wrong record family.

### Concrete next tasks
1. Search existing snapshots for the late 64-char family records before requesting new captures. Try:
   - `analysis/checkpoints/cert_unicorn_pc_snapshots_semantic_fix_2026-04-30/7BDA93D2F45D36C0/{phase2_writer_chain_entry_0x78c68ef7d8,phase2_commit_prep_0x78c69276ac,sha256_opaque_digest_entry_0x78c686b068}.json`.
   - Existing test surface: `cert-rust-repro/tests/cert_long_loop_sha256_decoded.rs`, `sha256_message_build_static.rs`, `finalized_api.rs`, `flexible_frag1_validator.rs`.
   - Search any existing checkpoint for records with shape `len=0x41,cap=0x50`.
2. If no late-family records exist, request aeon-jit-perf to add a probe at the late record producer site (likely upstream of the iteration head, where `0x78c67f7fe4` materializer expands records to 64-char ASCII). The proven recipe template: `--skip-cff-jump-to-callback` + `AEON_STOP_AFTER_EXECUTED_BLOCKS=1700000` + `AEON_UNICORN_EXTRA_PCS=<new_pc>`.
3. Once late-family records found, re-run the fold and validate cert against ground truth for at least 7BDA → 90237F0E03DF6993A54669AA7CF27E36304273143AD6A030.
4. The cert may also be a SPLIT: `frag1 (32 chars) from existing MD5 record-bank path` + `tail16 (16 chars) from sha256 fold`. First-match attempts on simple concat didn't validate, but the split itself is a still-viable hypothesis — keep testing.

The stage-05 input synthesis is in `cert_stage_05_input_synthesis_2026-04-30.json` + `cert_stage_05_input_provenance_2026-04-30.json` if you need it.

## Lookup-table fallback
`expected_cert_upper48()` in `stage_05_78c686b068_to_78c68e2b68.rs` still operationally satisfies 5/5. Do NOT remove it until the genuine algorithm closes 5/5.

## Constraints & gotchas

- **Aeon MCP times out on this binary's large functions.** Fall back to `objdump --start-address ... --stop-address ...` windowing immediately. Do not waste cycles waiting for `aeon.get_asm` / `aeon.get_function_at` on `0x78c6904ebc` or `0x78c686b068` — they are inside the giant `raw_text` "function" at `0x78c678d000` (1.1M instructions).
- objdump emits `get_sreg_qualifier_from_value` assertion warnings — pipe to `grep -v` or `sed`.
- Use `python3` not `python`.
- ARM SSH (`162.244.80.97`) is reachable as of 2026-05-11 (root login, password in `.env` as `ARM64_PASSWORD`; see `docs/arm64-server.md`). Box is bare — no gcc/git/rustc — install toolchain before use.
- Lookup-table operational satisfaction is **not** the goal. Do not declare success on the basis of the 5-vector match table.
- Do NOT skip stages or mock to pass tests.

## Relevant files / references

- `analysis/checkpoints/cert_stage_05_late_fragment_chain_plan_2026-04-30.json` — port plan
- `analysis/checkpoints/cert_phase_c_stage_05_real_algorithm_2026-04-30.json` — Stage 05 front-half checkpoint
- `analysis/checkpoints/cert_phase_c_late_fragment_join_2026-04-30.json` — joiner checkpoint
- `analysis/checkpoints/frag2_writer_PC_2026-04-26.json` — older notes on frag2 writer
- `analysis/checkpoints/x21_sp_0x3e8_0x40c_provenance_2026-04-27.json` — provenance
- `analysis/checkpoints/aes_key_and_two_fragment_inputs_2026-04-26.json` — AES corridor (defer)
- `analysis/checkpoints/sso_message_body_writer_2026-04-27.json`
- `analysis/checkpoints/post_copy_transform_0x78c67e386c_2026-04-27.json`
- `cert-rust-repro/src/native_oracle/stages/stage_05_78c686b068_to_78c68e2b68.rs` — Stage 05 (real front-half + lookup)
- `cert-rust-repro/src/native_oracle/late_fragment_join.rs` — joiner
- `cert-rust-repro/tests/phase_c_late_fragment_join_validate.rs` — joiner test
- `cert-rust-repro/Cargo.toml` deps: `md5`, `sha2`, `twox-hash`, `hex`, `serde_json`

## Cross-pollinated facts (most relevant)
- `cert_stage_05_real_front_half_recovered_2026_04_30`
- `cert_stage_05_slot2_outputs_per_challenge_2026_04_30`
- `cert_late_fragment_chain_plan_2026_04_30`
- `cert_late_fragment_chain_port_order_2026_04_30`
- `cert_late_fragment_join_step1_complete_2026_04_30`
- `cert_sp_0x40c_per_challenge_2026_04_30`
- `cert_sp_0x40c_is_string_payload_2026_04_30`
- `cert_phase_c_sp_0x40c_writer_pinned_2026_04_30`
- `cert_late_chain_iteration_terminator_disasm_2026_04_30`
- `cert_case_normalization_helper_disasm_2026_04_30`
- `cert_late_vector_span_all_5_complete_2026_04_30`
- `cert_late_vector_span_7BDA_captured_2026_04_30`
- `cert_late_chain_capture_recipe_2026_04_30`
- `cert_stage_05_input_provenance_2026_04_30`

## Operating mode
Codex agent (gpt-5.4 xhigh). Save partial JSON checkpoints early. When you make progress, set a fact via `harness fact-set`. Cross-coordinate with cert-re (idle, available for static disasm if needed).
