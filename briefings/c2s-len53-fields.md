## c2s-len53-fields — Verify mirror-block hypothesis on FRzMoveDuringSkillRq (C2S len=53)

## Role & workdir
Pure-Python statistical worker. Workdir: `/home/sdancer/dark-december-c2s-len53-fields` (worktree of `/home/sdancer/darkdecember/`, branch `c2s-len53-fields`).

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_len53_mirror_check` (new)
- **sub_goal_key**: `verify-or-falsify-mirror-pattern-in-frzmoveduringskillrq`

## Why this turn exists
Cycle 438 closed `dark_december_c2s_len61_insertion_block` with the **surprising finding** that FRzSkillMoveSyncRq's 16-byte insertion at `dec[17..32]` is NOT a new payload — it's a **mirror of request-family signature fields** (subtype, header quad, class_b signature word) plus one opaque trailer u32. This pattern is unusual and may generalize.

The natural verification target: cycle-412 bound C2S len=53 (104 frames) to `FRzMoveDuringSkillRq`. Per the cycle-407 shifted-insertion model, len=53 is also a len=45-family composite with an 8-byte insertion (not 16-byte like len=61). If the mirror-pattern generalizes, the 8 inserted bytes should mirror 2 of the 4 signature fields from len=45.

## Hypothesis
The 8-byte insertion in C2S len=53 frames (decoded offsets dec[17..24] under cycle-407's strict alignment) is **partial mirror** of the len=45 request-family signature: either (mirror_subtype_u32 + mirror_header_quad), or (mirror_class_b + opaque_u32), matching half of cycle-438's len=61 16-byte pattern. The remaining len=45 fields resume at dec[25..] shifted by +8.

## Falsification (3 outcomes)
- (a) **The 8 inserted bytes are a mirror of 2 of the 4 cycle-438 mirror fields** → SUCCESS, generalizes the mirror pattern. Fact: `dark_december_c2s_len53_mirror_confirmed_<n>_fields_named`.
- (b) **The 8 inserted bytes are NOT a mirror** (high-entropy, no copy of any len=45 field) → mirror pattern is specific to len=61 / FRzSkillMoveSyncRq; doesn't generalize. Fact: `dark_december_c2s_len53_not_mirror`.
- (c) **Partial** (some bytes mirror, some don't) → document the shape. Fact: `dark_december_c2s_len53_mirror_partial_<n>`.

## Success criteria — SINGLE TURN, do all before stopping

**Primary**: write `/home/sdancer/dark-december-c2s-len53-fields/analysis/c2s_len53_fields_2026-05-15.md` with:

1. **Filter 104 C2S len=53 frames** from `streams/first_quest/first_quest_c2s.tcpstream.bin` using `darkdec_decoder.py`. Confirm decoded length = 46 bytes.
2. **Cross-reference cycle-395 len=45 baseline** for the named field positions (token@0, subtype@1, marker@6, class_b@9..12, coord@17, heading@25, subtype@33, tail@37). At an 8-byte insertion, the len=45 tail fields shift to dec[25] (was 17) / dec[33] (was 25) / dec[41] (was 33) / dec[45] (was 37) under the +8 model.
3. **Test 8-byte mirror at dec[17..24]** for each cycle-438 mirror candidate:
   - dec[17..20] ?= mirror of dec[5..8] (header quad)
   - dec[17..20] ?= mirror of dec[9..12] (class_b signature)
   - dec[17..20] ?= mirror of dec[33] zero-extended (subtype)
   - dec[21..24] ?= mirror of either of the above
4. **Per-byte variance** on dec[17..24] across all 104 frames: distinct count, modal value, range.
5. **u32 / f32 sweep** at dec[17] and dec[21] for both alignments (strict +0 / off-by-one +1).
6. **Cross-check shifted tail**: verify dec[25..28] f32 reproduces cycle-395's coord@17 (= request_x_f32) by joining with player_track.csv x values.
7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `C2S_LEN53_FIELDS_DONE` on the final line.

## Constraints & gotchas
- **HARD memory budget: 200 MB.** 104 frames is trivial.
- **NO disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤300 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Apply cycle-435 framing: decoded view IS Serialize body. cycle-415 layout had off-by-one — re-anchor against cycle-395 empirical len=45 field positions.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-len53-fields/` (branch `c2s-len53-fields`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- Cycle-395 len=45 field map: `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
- Cycle-407 len=61 shifted-insertion model: `/home/sdancer/dark-december-c2s-len61-decode/analysis/c2s_len61_decode_2026-05-15.md`
- Cycle-438 mirror finding (PRIMARY REFERENCE): `/home/sdancer/dark-december-c2s-len61-fields/analysis/c2s_len61_fields_2026-05-15.md`
- Cycle-435 framing correction: `/home/sdancer/dark-december-raw-reconstruct/analysis/raw_reconstruct_2026-05-15.md`
- Rz binding (cycle 412): `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`
- success-fact key: `dark_december_c2s_len53_mirror_confirmed_<n>_fields_named` (a)
- block-fact keys: `dark_december_c2s_len53_not_mirror` (b), `dark_december_c2s_len53_mirror_partial_<n>` (c)
