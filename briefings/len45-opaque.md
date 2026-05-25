## len45-opaque — Crack the 3 opaque slots in C2S len=45 movement (offsets 13/21/29)

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-len45-opaque` (worktree of `/home/sdancer/darkdecember/`, branch `c2s-len45-slot-cracker`).

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_len45_opaque_slots` (new)
- **sub_goal_key**: `name-offsets-13-21-29-in-frzmoverq`

## Why this turn exists
Cycle 395's `c2s-len45-s2c-join` named 8/8 *signature* positions in FRzMoveRq but left **3 opaque slots** at decoded offsets 13, 21, 29 (each a 4-byte word). Every f32 / u32 / i32 interpretation tested against player x / z / yaw / movement deltas scored r² < 0.05. The slots are not geometric.

Cycle 415's FRzMoveRq Serialize layout (37 bytes from the prior `dark-december-move-decode` worker) describes the message as `msg_type(u16) + seq_or_move_kind(u32) + actor_or_move_handle(u64) + 6×coord_param(f32) + move_flag(u8)`. After the +1 adjacent-XOR offset, decoded offsets 13/21/29 fall inside the 6×coord_param block. So either (a) cycle 415's coord_param labels are wrong for those positions (the slots are NOT f32 coords), or (b) the slots ARE f32 but carry non-kinematic data (animation tween parameters, skill cooldown timers, server-clock samples).

## Hypothesis
The 3 opaque slots are **structural/categorical**, not geometric:
- Offset 13 is a `u16` sequence counter monotonic per-actor (the high 2 bytes are reserved/zero or a small enum).
- Offset 21 is a quantized i16/i16 animation pair: 2-byte animation_id (low) + 2-byte phase_value (high).
- Offset 29 is a CRC / tail-checksum over the preceding 28 bytes of the decoded body.

This is **orthogonal** to cycle 395's regressor tests, which only checked f32/u32/i32 against kinematic variables.

## Falsification (3 outcomes)
- (a) **All 3 slots interpreted with a stated mechanism** (sequence counter, animation pair, checksum, or other structural rule) AND the rule is verified on ≥70% of the 6022 frames → SUCCESS. Fact: `dark_december_c2s_len45_3_slots_named`.
- (b) **None of the structural interpretations clear the 70% threshold** → slots are encrypted, hashed, or session-keyed; structural interpretation is wrong. Fact: `dark_december_c2s_len45_slots_opaque_confirmed`.
- (c) **Partial** (1–2 of 3 named structurally, the rest fail) → name what fits, document what doesn't. Fact: `dark_december_c2s_len45_partial_slots_<n>_named`.

## Success criteria — SINGLE TURN, do all before stopping
**Primary**: write `/home/sdancer/dark-december-len45-opaque/analysis/len45_opaque_slots_2026-05-15.md` with:

1. **Load 6022 C2S len=45 frames** using darkdec_decoder.py (cycle-383). Build a DataFrame: ord, actor_id (from FRzMoveRq decoded offset 6 = actor_or_move_handle u64), decoded body bytes 0–37.
2. **For offset 13 (4-byte word)**:
   - test as u32 (continuous): variance, autocorrelation
   - test as u16+u16 split (offsets 13..15 and 15..17): per-actor monotonic delta sequence — for each actor_id, sort frames by ord and compute Δ between consecutive u16 values. If ≥70% of actors show monotonic non-decreasing mod 65536 sequence, it's a per-actor sequence counter.
   - test as u8×4: byte-wise distinct value counts
3. **For offset 21 (4-byte word)**:
   - test as i16+i16: are the values in a bounded range like [-1024..1024] consistent with animation IDs or fixed-point phase?
   - test as f32: even if not correlated with player kinematics, are values bounded in a small range (e.g., -1..1 for normalized phase)?
   - cross-correlate with subtype@33 (named in cycle 395 as movement_subtype_u8 with 7 values): does offset 21 cluster by subtype?
4. **For offset 29 (4-byte word)**:
   - test as CRC32 over decoded body bytes [0:29] using standard polynomials (CRC32, CRC32C, Adler32, Fletcher32, CRC16+CRC16 split, XOR-fold-32).
   - test as a hash over the previous 6 coord_param f32 values: simple XOR-sum, IEEE-754 bitcast XOR, addition checksum.
   - test as u32 with bounded variance: if values fit in [0, N] for small N, possibly a frame-counter.
5. **For each slot**, report the best-fit interpretation + confidence (% of frames matching the rule) + 3 example frames.
6. **Cross-validate against cycle 415's FRzMoveRq layout**: if a slot's interpretation contradicts the "f32 coord_param" label, document this as a refinement to the cycle-415 layout.
7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `LEN45_OPAQUE_DONE` on the final line.

## Execution flow

**Step 1** — Load frames + build DataFrame keyed by actor_id.
**Step 2** — Per-slot exhaustive structural tests (per the hypothesis families above).
**Step 3** — Generate per-slot fit report with thresholds.
**Step 4** — Reconcile with cycle-415 layout.
**Step 5** — Write artifact + fact-set + DONE.

## Constraints & gotchas
- **HARD memory budget: 250 MB.** 6022 frames is small.
- **NO disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤400 lines.
- **ONE Codex turn budget: ≤25 min wall time. SINGLE-TURN COMPLETION REQUIRED** — do all 7 steps before stopping; do not pause at step boundaries.
- The CRC32 test must use `zlib.crc32` + `binascii.crc32` (CRC32B / IEEE 802.3 polynomial). For CRC32C use `crcmod` with the Castagnoli polynomial if available; otherwise fall back to pure-Python implementation.
- Reference artifacts (READ-ONLY):
  - cycle-395 c2s-len45 field analysis: `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
  - cycle-415 FRzMoveRq Serialize layout: `/home/sdancer/dark-december-frzmove-br-decode/analysis/frzmove_br_decode_2026-05-15.md`

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-len45-opaque/` (branch `c2s-len45-slot-cracker`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- success-fact key: `dark_december_c2s_len45_3_slots_named` (a)
- block-fact keys: `dark_december_c2s_len45_slots_opaque_confirmed` (b), `dark_december_c2s_len45_partial_slots_<n>_named` (c)
