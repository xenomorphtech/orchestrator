## c2s-len61-fields — Decode the 16-byte insertion block in FRzSkillMoveSyncRq (C2S len=61)

## Role & workdir
Pure-Python statistical worker. Workdir: `/home/sdancer/dark-december-c2s-len61-fields` (worktree of `/home/sdancer/darkdecember/`, branch `c2s-len61-fields`).

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_len61_insertion_block` (new)
- **sub_goal_key**: `name-fields-in-the-16-byte-insertion-block`

## Why this turn exists
Cycle 407 closed `dark_december_c2s_len61_field_semantics` with the verdict that **C2S len=61 (952 frames) is a len=45-family composite with a 16-byte INSERTION at decoded offsets [17..32]**, after which the len=45 fields resume shifted by +16. Cycle 412 bound this length class to `CL_GS::FRzSkillMoveSyncRq`. Cycle 415 documented the inserted-block at dec[13..14] has a near-fixed selector `0x04a5` in 933/952 frames.

The fields inside that 16-byte insertion block are **unnamed**. Per cycle-435's corrected framing (DECODED view IS Serialize body, msg_subtype u8 at dec[0]), this is fresh substrate.

## Hypothesis
The 16-byte insertion block at decoded offsets [17..32] of C2S len=61 frames carries the `RzSkill` argument that `RzPktAction::RQSkillMoveSync(int64, int64, FVector&, FVector&, int8&)` writes per the cycle-406 RzPktAction handler signature. Layout candidates:
1. **(skill_id u32 / target_id u64 / 4-byte flags)** — discrete-action payload.
2. **(start_pos.x f32, start_pos.y f32, start_pos.z f32, end_pos.x f32)** — first 16 bytes of a (FVector start, FVector end) pair.
3. **(skill_class u16, skill_id u16, target_id u64, flag u32)** — RzSkill object layout.

## Falsification (3 outcomes)
- (a) **All 16 inserted bytes named at least at type level (u8/u16/u32/u64/f32 with statistical evidence)** → SUCCESS. Fact: `dark_december_c2s_len61_inserted_block_<n>_named`.
- (b) **Bit patterns at all 16 offsets are unphysical / opaque** → block is encrypted, session-keyed, or compressed. Fact: `dark_december_c2s_len61_inserted_block_opaque`.
- (c) **Partial naming** (2-8 of 16 bytes named) → name what's defensible. Fact: `dark_december_c2s_len61_inserted_block_partial_<n>`.

## Success criteria — SINGLE TURN, do all before stopping

**Primary**: write `/home/sdancer/dark-december-c2s-len61-fields/analysis/c2s_len61_fields_2026-05-15.md` with:

1. **Filter 952 C2S len=61 frames** using `darkdec_decoder.py`. Confirm decoded length = 54 bytes. Extract the 16-byte block at decoded offsets `[17..32]` per cycle-407's shifted-insertion model.
2. **Per-byte variance**: for each of the 16 offsets, distinct-value count, top-3 values, modal frequency. Cycle 415 noted offset 13 had selector `0x04a5` in 933/952 — confirm and extend to the rest.
3. **u32 / f32 sweep**: at offsets `[17, 21, 25, 29]` (4-byte aligned within the insertion block), interpret as u32 LE and f32 LE. Report distinct counts, ranges, near-zero %, finite %.
4. **Cross-reference with the cycle-433 corrected understanding**: the FRzSkillMoveSyncRq is a Rq (not Br), so it has the leading 4-byte `seq_or_move_kind` from cycle-415's Rq layout (which we now suspect is also misaligned). Try BOTH alignments: (a) cycle-407 strict offsets [17..32]; (b) [16..31] in case of off-by-one.
5. **Skill-id candidates**: if the cycle-406 catalog has `RzSkill::GetType`-style symbols, check whether the constant `0x04a5` matches any specific skill class. Cross-pollinate from `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`.
6. **FVector candidates**: if 12 of the 16 bytes are f32 reads in a plausible coordinate range (matching the same scale as FRzMoveBr's pos_x at decoded offset 9), that's evidence for a (start_pos: FVector, end_pos.x: f32) layout.
7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `C2S_LEN61_FIELDS_DONE` on the final line.

## Constraints & gotchas
- **HARD memory budget: 250 MB.** 952 frames × 16 bytes is tiny.
- **NO disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤300 lines.
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Cycle-435 framing correction is in effect: the DECODED view IS the Serialize body for these messages. There's no inner packet_id at raw_body[0..1]. Cycle 415 / cycle 412 layouts had off-by-one — don't trust their offsets blindly; re-anchor everything against cycle-403's empirical x/z/rot positions.
- Cross-pollination memory: [[project_dark_december_wire_framing_plus8]] (REVISED), [[project_dark_december_wire_decoder]].

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-len61-fields/` (branch `c2s-len61-fields`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- Cycle-407 len=61 model: `/home/sdancer/dark-december-c2s-len61-decode/analysis/c2s_len61_decode_2026-05-15.md`
- Cycle-435 framing correction: `/home/sdancer/dark-december-raw-reconstruct/analysis/raw_reconstruct_2026-05-15.md`
- RZ symbol catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- Rz binding (cycle 412): `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`
- success-fact key: `dark_december_c2s_len61_inserted_block_<n>_named` (a)
- block-fact keys: `dark_december_c2s_len61_inserted_block_opaque` (b), `dark_december_c2s_len61_inserted_block_partial_<n>` (c)
