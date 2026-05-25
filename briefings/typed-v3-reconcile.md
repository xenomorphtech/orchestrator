## typed-v3-reconcile — Mark fragile bindings + add +7 framing note to typed Rust decoder

## Role & workdir
Rust documentation worker. Workdir: `/home/sdancer/dark-december-typed-v3-reconcile` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v3-reconcile`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v3_reconcile` (new)
- **sub_goal_key**: `mark-cycle-446-fragile-bindings-in-typed-decoder`

## Why this turn exists
Cycle 446's minor-class-bindings worker showed that under cycle-435's corrected +7 framing, the cycle-412 +8 bindings for **C2S len=53 (FRzMoveDuringSkillRq), len=42 (FRzControlProjectileRq), and len=8 (heartbeat)** do not survive. Only **len=45 (FRzMoveRq/FRzStandRq) and len=41 (FRzMoveBr family)** still hold via cycle-403 empirical anchors. The shipping rust-decoder-typed-v2 (cycle 422) treats all 5 high-confidence bindings as equally valid; this is now known to be over-confident for 3 of them.

This is a quick documentation-only reconciliation — no decoder behavior change. The typed decoder still works; the documentation is updated.

## Hypothesis
Adding source-code comments + an updated `analysis/rust_typed_decoder_v3_2026-05-15.md` that marks 3 of the 7 bindings as "size-match-only, fragile under +7 framing" allows downstream consumers to weight them correctly. The 64.63% typed coverage stat doesn't change; only the confidence annotations do.

## Falsification (3 outcomes)
- (a) **Source comments + reconciliation doc written + `cargo build --release` still succeeds + replay diff stays empty** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v3_reconciled`.
- (b) **Build breaks** → revert; documentation-only changes shouldn't break Rust. Fact: `dark_december_decoder_rust_typed_v3_break`.
- (c) **Partial** (some comments added, build OK, but doc incomplete) → name what's defensible. Fact: `dark_december_decoder_rust_typed_v3_partial`.

## Success criteria — SINGLE TURN

**Primary**: 

1. **Copy cycle-422 baseline** from `/home/sdancer/dark-december-rust-decoder-typed-v2/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify cargo build OK.

2. **Annotate `src/lib.rs`** at each of the 7 typed variants with a one-line comment:
   - `// HIGH (cycle-403 empirical anchor + cycle-435 +7 framing)` for `FRzMoveRq`, `FRzMoveBr`
   - `// FRAGILE (cycle-412 size-match under +8; cycle-446 reports +7 framing collision) — verify with controlled live capture` for `FRzSkillMoveSyncRq`, `FRzMoveDuringSkillRq`, `FRzControlProjectileRq`
   - `// MEDIUM (cycle-419 temporal-lift binding, independent of framing model)` for `FRzTriggerActiveBr`, `FRzCoolTimeResetNoti`

3. **Replay-validate**: cargo build + run on first_quest streams + diff vs `/home/sdancer/orchestrator/darkdec_output_streams/`. Must stay empty.

4. **Write `analysis/rust_typed_decoder_v3_2026-05-15.md`** (≤150 lines):
   - Summary of cycle-446's finding
   - Binding confidence table (7 rows)
   - "No behavior change" note (the classifier still emits the same packets; only documentation updated)
   - Recommendation for next steps (live-capture validation)

5. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V3_RECONCILE_DONE` on the final line.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤150 lines.
- **ONE Codex turn budget: ≤10 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- This is documentation-only; if you find yourself rewriting the classifier, STOP and just add comments.

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-rust-decoder-typed-v2/`
- Cycle-446 finding: `/home/sdancer/dark-december-minor-class-bindings/analysis/minor_class_bindings_2026-05-15.md`
- Cycle-435 framing correction: `/home/sdancer/dark-december-raw-reconstruct/analysis/raw_reconstruct_2026-05-15.md`
- Cycle-444 protocol wiki: `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md`
- Streams + baseline: as in cycle-422
- success-fact: `dark_december_decoder_rust_typed_v3_reconciled` (a)
- block-facts: `dark_december_decoder_rust_typed_v3_break` (b), `dark_december_decoder_rust_typed_v3_partial` (c)
