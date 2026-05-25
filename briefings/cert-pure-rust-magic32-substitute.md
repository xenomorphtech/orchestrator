# cert-pure-rust-magic32-substitute — Path C

## Role & workdir
Codex worker, workdir `/home/sdancer/nmss-emu-cert-pure-rust-magic32-substitute`. Branch `cert-pure-rust-magic32-substitute` forked from `main` (commit c654108).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `pure-rust-cert-with-captured-magic32-against-14-wire-pairs`

## Why this path
- Lane N **captured MAGIC32** = `41fe832e642ff1991e6ca7553277a34c` (deterministic 3+ runs via memcpy hook).
- `cert-rust-repro` already reproduces 5/5 emu certs **with a baked MAGIC32** of `2FCF997702C244969BFEAF7F0D6AAA1C`.
- The crate exposes `CLUSTER1_PREFIX_SELECTOR_MAGIC` at `src/native_oracle/stages/stage_two_step_sha256_cert.rs:128` — substituting the captured MAGIC32 in this single constant and re-running held-out validation answers the gating question for the goal.
- Lane P (Unicorn) failed at extraction layer, NOT at algorithm — so the algorithmic question Lane P set out to test is still open. This is the cleanest way to answer it.

## Success criteria
- Done = fact `nmss_clientless_fresh_login_replay_complete_2026_05_18 = true` IF ≥10 of 14 captured (challenge, Token) wire pairs are reproduced by the pure-Rust cert function when MAGIC32 = `41fe832e642ff1991e6ca7553277a34c` (or any one-letter case-variant; document which case the algorithm wants).
- Verdict at `analysis/cert_pure_rust_magic32_substitute_verdict.md`. Final line `CERT_PURE_RUST_MAGIC32_SUBSTITUTE_DONE`.

## Concrete tasks (do in order)

1. **Inventory the 14 wire pairs.** Read `/home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v*_verdict.md` (v2 → v16) — each documents `(challenge_hex16, Token_hex48, sessionID_hex16)` from a captured op901. Assemble into `tests/wire_pairs_14.rs` as a `&[(challenge, token, sessionID)]` slice. **Verify length = 14**; if some verdicts contain multiple pairs, dedupe by Token.

2. **Substitute MAGIC32.** Patch `/home/sdancer/nmss-emu-cert-pure-rust-magic32-substitute/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs:128` to set `CLUSTER1_PREFIX_SELECTOR_MAGIC: &str = "41fe832e642ff1991e6ca7553277a34c"`. Run the existing test `cargo test --release --test captured_sp968_5vec_verify` — it MUST still pass on the 5 emu vectors (which are MAGIC32-agnostic per `compute_const_32b()` derivation). If it fails, your substitution broke an invariant; investigate before continuing.

3. **Add the 14-pair held-out validator** at `tests/wire_pairs_14_verify.rs`. For each (challenge, expected_token) pair, run the full cert pipeline (`stage_two_step_sha256_cert::cert_for_challenge_with_magic32(challenge, MAGIC32)` or equivalent — read the crate to find the right entry point) and compare to `expected_token`. Output: count of matches `K / 14`.

4. **Goal-met check.** If K ≥ 10: set fact `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`. Write verdict. Single commit on branch `cert-pure-rust-magic32-substitute`. **Final line `CERT_PURE_RUST_MAGIC32_SUBSTITUTE_DONE`.**

5. **If K = 0 (or < 10) — diagnostic sweep BEFORE retiring.** Try MAGIC32 case variants: ALL_UPPER (`41FE832E642FF1991E6CA7553277A34C`), ALL_LOWER (already), Camel/mixed. Try the case-toggle brute test pattern from `cert-rust-repro/tests/case_toggle_brute.rs` adapted to the 14-pair set. Try reversing-byte-order. If any variant gets K ≥ 10 → that's the answer; record which case the algorithm consumes. If no variant works → write the verdict explaining "algorithm changed between cycle-141 baseline install and cycle-1467 install OR the captured MAGIC32 is one piece of a larger preimage" and stop. Do NOT escalate to harder paths; the orchestrator queues Path B (algorithm fingerprinting) for that diagnostic outcome.

## Falsification criterion
0 of 14 wire pairs reproduce across captured MAGIC32 + every one-letter case/encoding variant → algorithm differs between cycle-141 and cycle-1467 install, OR a session-bound field is mixed in beyond MAGIC32.

## Constraints & gotchas
- **Do NOT modify the substrate.** No device interaction, no Lane M re-runs. Pure-Rust only on this server.
- **Do NOT change `compute_const_32b()`** — its derivation from MAGIC32 is verified (5/5 emu certs); just replace the constant.
- **Be precise about MAGIC32 case.** Captured value is lowercase `41fe832e...` but the existing baked value is UPPERCASE. The algorithm uses `.as_bytes()` so case matters. Test both before declaring failure.
- **Don't extend scope.** If K ≥ 10, STOP — the goal is closed. Do not try to "also reproduce the live cert" — that's a Lane M problem unrelated to this path.
- **Single commit, --no-verify forbidden, no `--amend`.** Pre-existing repo rules.

## Relevant files / references
- Worktree: `/home/sdancer/nmss-emu-cert-pure-rust-magic32-substitute`
- Crate root: `cert-rust-repro/` inside the worktree
- Constant to patch: `cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs:128`
- Existing 5-vec test: `cert-rust-repro/tests/captured_sp968_5vec_verify.rs`
- 14-pair source verdicts: `/home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v[2-16]_verdict.md`
- Memory: `[[native-replay-rs]]` (the 5/5 reproducer this builds on)
- Key facts: `magic32_value_extracted_2026_05_18`, `cert_algorithm_matches_cycle_141_142_recipe_2026_05_18`, `nmss_cert_5_5_pure_rust_reproduced` (the existing pass)
- Wire target example (v11): challenge=`176062C5A333E9E7`, Token=`BA00FE3EAB6937AD8183100FEB4D14B7B76C67EAD7C38D9C`
