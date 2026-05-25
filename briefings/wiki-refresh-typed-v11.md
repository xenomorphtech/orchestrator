## wiki-refresh-typed-v11 — Update PROTOCOL_WIKI.md with v3→v11 typed-decoder saturation

## Role & workdir
Documentation worker. Workdir: `/home/sdancer/dark-december-protocol-wiki` (existing worktree, branch `protocol-wiki`).

## Current goal / sub-goal
- **goal_key**: `dark_december_protocol_wiki_typed_v11_refresh` (new)
- **sub_goal_key**: `document-typed-decoder-96-13-state`

## Why this turn exists
PROTOCOL_WIKI.md was last updated cycle 444 (19 closures). Since then 11 typed-coverage push cycles (v3→v11) lifted the Rust decoder from 64.63% → 96.13% (+31.50pp / +15,117 frames), adding 57 typed variants. The wiki is stale on this front. This cycle refreshes it with:
- v3→v11 ascent table
- Current typed-variant catalog (57 entries grouped by family)
- Documentation of the explicit-Placeholder naming pattern (cycle 474+) that resolved cycle-472 planner's invariant concern
- Pointer to latest worktree `darkdecember/typed-v11-long-tail`

## Hypothesis
A 80-150 line append to PROTOCOL_WIKI.md (new section "Typed Rust Decoder — v11 State") consolidates the ascent and variant catalog in one place, making the wiki current as of 2026-05-16.

## Falsification (3 outcomes)
- (a) **Section added + committed + pushed + readable** → SUCCESS. Fact: `dark_december_protocol_wiki_typed_v11_added`.
- (b) **Wiki file unwritable OR commit/push breaks** → revert + report. Fact: `dark_december_protocol_wiki_typed_v11_break`.
- (c) **Partial — section written but ≤30 variants documented** → name what's defensible. Fact: `dark_december_protocol_wiki_typed_v11_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Read** existing `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md`. Identify the natural insertion point for a new "Typed Rust Decoder" section (likely near the end, after the closure catalog).

2. **Inspect the v11 source** at `/home/sdancer/dark-december-typed-v11-long-tail/src/lib.rs` to enumerate all 57 typed variants by name. Use the file directly — don't grep with patterns that might miss long names.

3. **Append a new section** "## Typed Rust Decoder — v11 State (2026-05-16)" with:
   - **Coverage table**: v3 (64.63%) → v4 → v5 → v6 → v7 → v8 → v9 → v10 → v11 (96.13%) with per-cycle Δpp.
   - **Variant catalog** grouped by family:
     - HIGH-confidence named bindings (cycle-403 anchors): FRzMoveRq, FRzMoveBr
     - FRAGILE bindings (cycle-446 caveat): FRzSkillMoveSyncRq, FRzMoveDuringSkillRq, FRzControlProjectileRq
     - MEDIUM bindings: FRzTriggerActiveBr, FRzCoolTimeResetNoti, FRzRotationBr, FRzTeleportRp, FRzEntityNotify9700, FRzEntityNotify9400, FRzMovementCoreReply8400, FRzOverlapRq, FRzAckResult
     - Explicit PLACEHOLDERS (cycle 474+): FRzC2SLen10Placeholder + 41 long-tail variants from v11.
   - **Explicit-Placeholder pattern**: 5-10 line documentation of why and how (preserves "typed=named+RE-grounded" invariant via labeling rather than exclusion).
   - **Branch pointer**: `darkdecember/typed-v11-long-tail` is the latest.
   - **Saturation note**: remaining 3.87% (1,858 frames) lives in mini-buckets <50 frames each; further extension requires live-tap state or new symbol corpus.

4. **Validate**: confirm the file still parses as valid markdown (no broken tables, no truncated sections). Print first 5 and last 5 lines.

5. **Commit + push** to `darkdecember/protocol-wiki` branch. Use a clear commit message naming the cycle.

6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `WIKI_REFRESH_TYPED_V11_DONE`.

## Constraints
- **HARD memory budget: 200 MB** (this is pure documentation work).
- **HARD output cap**: ≤150 lines of new wiki content.
- **ONE Codex turn budget: ≤10 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **IMPORTANT**: After writing, committing, pushing, fact-setting, and final WIKI_REFRESH_TYPED_V11_DONE marker, EXIT IMMEDIATELY. Do NOT continue with additional documentation work.
- Do NOT rewrite existing wiki content — APPEND a new section only.

## References
- Existing wiki: `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md` (cycle-444 consolidation)
- v11 source: `/home/sdancer/dark-december-typed-v11-long-tail/src/lib.rs`
- v11 artifact: `/home/sdancer/dark-december-typed-v11-long-tail/analysis/typed_v11_long_tail_2026-05-16.md`
- Memory: `/home/sdancer/.claude/projects/-home-sdancer-orchestrator/memory/project_dd_typed_coverage_saturated.md`
- success-fact: `dark_december_protocol_wiki_typed_v11_added` (a)
- block-facts: `dark_december_protocol_wiki_typed_v11_break` (b), `dark_december_protocol_wiki_typed_v11_partial` (c)
