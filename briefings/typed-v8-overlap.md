## typed-v8-overlap — Add FRzBeginOverlapRq + FRzEndOverlapRq variants for C2S len=37 (~280-400 frames)

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v8-overlap` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v8-overlap`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v8_overlap` (new)
- **sub_goal_key**: `add-overlap-rq-pair-for-c2s-len37`

## Why this turn exists
Cycle 466 typed-v7-8400 landed at 74.18% (+1.10pp). Cycle-412 named C2S len=37 as `FRzBeginOverlapRq / FRzEndOverlapRq` (Serialize=29, exact unique size match in 822-typename catalog). Per K=6 planner audit (cycle 467): this is a pure size-class binding, safe under cycle-446 fragility caveat (which only flagged FIELD-level interpretation in len=53/42/8, NOT size-class membership). Adding both variants in one cycle lifts coverage 74.18% → ~75.0% (+0.6-0.8pp).

**Two-variant scope** is a slight departure from the prior single-variant pattern; both names map to the SAME length class (37) and the same underlying Serialize size — they must be jointly disambiguated by an internal byte we'll determine from the corpus (or, if no clean discriminator, types both as the same variant `FRzOverlapRq` and notes the ambiguity).

## Hypothesis
C2S len=37 has ~280-400 frames distributed across the begin/end pair. A classifier rule `direction == C2S && wire_len == 37 && decoded.len() == 30` types ≥280 frames; coverage lifts 74.18% → ≥74.6% (gate). Disambiguation: examine `decoded[0]` or `decoded[1]` for a subtype byte that splits begin vs end (precedent: 9700/9400 family uses dec[0]).

## Falsification (3 outcomes)
- (a) **Variant(s) added + cargo build OK + ≥280 typed + coverage ≥74.6% + v7 regression check passes** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v8_<coverage>_overlap_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v8_break`.
- (c) **Coverage moves but <74.6% OR fewer than 200 frames classified** → name what's defensible (one combined variant) + report. Fact: `dark_december_decoder_rust_typed_v8_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v7-8400 baseline** from `/home/sdancer/dark-december-typed-v7-8400/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Discovery sub-step (≤2 min)**: count C2S len=37 frames in `first_quest_c2s.tcpstream.bin` via the existing decoder. Report:
   - Total C2S len=37 frame count.
   - Top-5 `decoded[0..2]` byte pairs to identify a begin/end discriminator.
   - If a clean 2-way split exists → emit two variants (`FRzBeginOverlapRq`, `FRzEndOverlapRq`). If not → emit ONE combined `FRzOverlapRq` variant and note ambiguity.

3. **Add variant(s)** in `src/lib.rs`:
   ```rust
   // Either two variants if clean discriminator found:
   FRzBeginOverlapRq { discriminator: u8, tail: Vec<u8> },
   FRzEndOverlapRq   { discriminator: u8, tail: Vec<u8> },
   // OR one combined variant if no clean split:
   FRzOverlapRq      { subtype: u8, tail: Vec<u8> },
   ```
   - Classifier rule (placed after FRzMovementCoreReply8400): `direction == Direction::C2S && raw.len() == 37 && decoded.len() == 30`.
   - Annotate `// MEDIUM (cycle-412 size-only binding under +7-effective framing; safe per cycle-446 caveat; FRAGILE on internal field interpretation pending Rz reflection).`
   - Match-arms for `id()`/`x()`/`z()`/`rot()` return `None`.

4. **Validate**: cargo build --release; run on `first_quest_{c2s,s2c}.tcpstream.bin`; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - Overlap frame count(s).
   - Confirmation that v7-typed-variant frame counts are UNCHANGED.

5. **Diff vs v7-typed CSV** (regression check): `out_v7_baseline/typed_packets.csv` vs `out_v8_orch/typed_packets.csv` — every row present in v7 must be present in v8 with identical fields; v8 only ADDS rows.

6. **Write `analysis/typed_v8_overlap_2026-05-16.md`** (≤80 lines):
   - Discovery: C2S len=37 frame count + top-5 `decoded[0..2]` split.
   - Coverage table (v7 74.18% → v8 X.XX%).
   - Overlap frame count(s) by variant.
   - Regression confirmation.
   - FRAGILE note: size-class binding is safe; internal field interpretation TBD.

7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V8_OVERLAP_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤80 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **IMPORTANT (cycle-465 stall lesson)**: After writing the artifact, fact-set, and final TYPED_V8_OVERLAP_DONE marker, EXIT IMMEDIATELY. Do NOT continue running additional commands.
- Do NOT change v7 variant classifications.

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v7-8400/`
- Cycle-412 rz-binding analysis: `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md` (line 96 documents the len=37 binding)
- Cycle-466 v7-8400 artifact: `/home/sdancer/dark-december-typed-v7-8400/analysis/typed_v7_8400_2026-05-16.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- success-fact: `dark_december_decoder_rust_typed_v8_<coverage>_overlap_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v8_break` (b), `dark_december_decoder_rust_typed_v8_partial` (c)
