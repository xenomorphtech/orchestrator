# thered-cert-claude — re-anchor: cert reproduction is ALREADY achieved (levels 1-3)

## ⚠️ READ THIS FIRST — correction of prior stalled-meta closure

Your 68fdaf9 "STALLED-META: hypothesis exhaustion report" was scoped too broadly. The user corrected it (2026-05-20):

> "thered-cert-claude reached an invalid conclusion yet again, stating that challenge->cert reproduction isn't possible when that was already achieved before... we actually derived the algo that produces same input->output at some cycle before."

**Cert reproduction has been achieved at three independent levels. Only level 4 (generalizable algorithmic recovery for unseen challenges) is unsolved.** Frame all future work in those terms.

| Level | Artifact | Verified | Substrate | Status |
|---|---|---|---|---|
| 1 | `/home/sdancer/nmss-emu/cert-rust-repro/` | 5/5 ground-truth vectors via `cargo test --test phase_c_chain_integration_status` | **Pure Rust, any host** | ✅ DONE |
| 2 | `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/` (commit `66cd2d7`) | 5/5 via `NMSS_REPLAY_SNAPSHOT=<dir> cargo test cert_vector_` | aarch64 + `trampoline_proc_memdump_5558` snapshot | ✅ DONE |
| 3 | Path O HW BP runtime oracle (your commit 6584a07) | Token validated end-to-end | Live device | ✅ DONE |
| 4 | Generalizable algorithmic recovery: extract Stage 05 transform inside `0x78c686aaa0` | n/a | n/a | ❌ UNSOLVED |

Level 1 caveat (from `cert_rust_repro_FINAL_SUMMARY_2026-04-30.md` self-audit): Stage 05 in cert-rust-repro is implemented as a hardcoded 5-challenge match table (`expected_cert_upper48`), not the real hash. `derive_stage_05_input` IS a real recovered formula. So the function is 5/5 *correct by construction* for the 5 known challenges; 6th-unseen-challenge would fail.

**For `nmss_clientless_fresh_login_replay` this is sufficient if** the server's challenges fall within the 5 known (or session_id is deterministic such that we can extend the table). Verify by capturing more (challenge, cert) pairs from the device and seeing whether they match the existing 5 or add new ones.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay`
- **sub_goal_key**: `cert-rust-repro-extend-coverage` (replaces lane-y3-libunreal-token-producer-bp)

## Hypothesis (re-framed)

The cert problem is **decomposed into two sub-goals**, both already partly addressed:

**(A)** Make `cert-rust-repro` cover the server's actual challenge space.
- Capture more (challenge, cert) pairs from live device captures (we have ~4 from prior op901 captures: `19EA6511F5DE482E98DAD71A48CF2AA7` MAGIC32 session with Token = `BA289103263DCA30E15357D7C5FEFB4077382AF000DA78FA`, challenge `358CD9791D78742E`).
- For each new pair, add an entry to `stage_05_78c686b068_to_78c68e2b68.rs` `expected_cert_upper48` match. If `derive_stage_05_input` formula holds, the test will pass.
- **Falsification**: if `derive_stage_05_input` does NOT produce the right register tuple for a new challenge (deterministic check before the match table), the formula needs revision.

**(B)** Recover the actual Stage 05 transform (level 4) for full algorithmic clientless. This is the path documented as "structurally unreachable" in your exhaustion report and that conclusion may stand — but it's a *different* goal than level 1.

## Success criteria (revised)

For **(A)**: Each new captured (challenge, cert) pair adds to the match table; `cargo test` still 5+N/5+N green. Fact: `cert_rust_repro_extended_to_N_pairs_<date>`.

For **(B)**: Genuinely a research goal, may remain stalled-meta. If you write another exhaustion report it must explicitly NOT claim cert reproduction in general is impossible — only that the Stage 05 transform recovery is.

## Next concrete tasks (in order)

1. **Verify `cert-rust-repro` actually works on your host**. From `/home/sdancer/nmss-emu/cert-rust-repro/` (or copy to your worktree if needed), run `cargo test --test phase_c_chain_integration_status -- --nocapture`. Confirm 5/5 pass. This is the anchor — if this fails, surface it immediately as a critical fact.

2. **Acknowledge in `analysis/lane_y3_correction_2026_05_20.md`** that the prior stalled-meta closure (68fdaf9) was overscoped. Document the level 1-4 decomposition. Commit on branch `magic32-live-snapshot-replay` with message `lane-y3 CORRECTION: cert reproduction levels 1-3 are achieved; only level 4 (generalizable algo for unseen challenges) is open`.

3. **Survey existing captured (challenge, cert) pairs** in `wire-decoder-rs/` and recent op901 captures. If any pair is NOT in the cert-rust-repro 5, set a fact `cert_rust_repro_coverage_gap_<date>` listing them — that's the next concrete extension work.

## Constraints & gotchas

- **DO NOT** retire to "stalled-meta" without distinguishing levels 1-3 vs 4. The user explicitly flagged this conflation.
- **`adb localhost:5558` ONLY**. No other device addresses.
- **NO Frida on libUnreal.so** (anti-cheat). HW BP via ptrace only.
- **NO `pm clear`** — would invalidate MAGIC32 and force 8GB patch redownload.
- **Hard RSS cap 512MB** on disasm tools (per memory).
- **Goals NEVER blocked** per `[[goals-never-blocked]]` — but ALSO not falsely-blocked. Surface the actual scope of what's achieved.

## Relevant files / references

- `/home/sdancer/nmss-emu/cert-rust-repro/` — pure Rust 5/5 reproducer (level 1).
- `/home/sdancer/nmss-emu/analysis/cert_rust_repro_FINAL_SUMMARY_2026-04-30.md` — full writeup with honest scope.
- `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_05_78c686b068_to_78c68e2b68.rs` — Stage 05 implementation (the lookup-table caveat).
- `/home/sdancer/nmss-emu-fulltrace/native-replay-rs/` — snapshot reproducer at commit `66cd2d7` (level 2).
- Your worktree commits: `6584a07` (Path O TOKEN VALIDATED, level 3) and `68fdaf9` (the over-scoped closure being corrected here).
- Memory pointers: `[[cert-rust-repro]]`, `[[native-replay-rs]]`, `[[goals-never-blocked]]`, `[[impossibility-caution]]`.
