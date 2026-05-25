## coord-param-disambig — Disambiguate FRzMoveBr's 6 coord_param f32 slots into named semantics

## Role & workdir
Pure-Python statistical-analysis worker. Workdir: `/home/sdancer/dark-december-coord-param-disambig` (worktree of `/home/sdancer/darkdecember/`, branch `coord-param-disambig`).

## Current goal / sub-goal
- **goal_key**: `dark_december_frzmove_br_coord_params` (new)
- **sub_goal_key**: `assign-semantics-to-6-coord-param-f32-slots`

## Why this turn exists
Cycle 415 named `FRzMoveBr::Serialize`'s 33 bytes broadly: `msg_type u16 + actor_or_move_handle u64 + 6×coord_param f32 + move_flag u8`. The "6 coord_param f32" label is collective: cycle 403's empirical extraction (`x@9, z@17, rot@25` in decoded view) maps to **coord_param_0, _2, _4** in the Serialize layout, but the intervening slots **_1, _3, _5** are unnamed. The typed Rust decoder (cycle 422) emits them as opaque floats in `extra_json`.

Naming those 3 unnamed slots is the natural per-field finishing pass. With 23,568 FRzMoveBr frames in the corpus, statistical structure analysis can distinguish (a) bounded angles (yaw/pitch/roll), (b) wide-range coords (Y altitude), (c) small velocity components, (d) padding/zero fields.

## Hypothesis
The 6 coord_param slots decompose as:
- coord_param_0 = `pos_x` (already known)
- coord_param_1 = `pos_y` (altitude) — wide range like x/z
- coord_param_2 = `pos_z` (already known)
- coord_param_3 = `rot_yaw` OR `rot_pitch` — bounded angle range
- coord_param_4 = `rot_yaw` (already known per cycle 403's `rot`)
- coord_param_5 = `vel_magnitude` or `move_speed` — small float, often constant

## Falsification (3 outcomes)
- (a) **≥4 of 6 slots named with statistical evidence** (range, distribution, correlation with sibling slots) → SUCCESS. Fact: `dark_december_frzmove_br_coord_params_<n>_named`.
- (b) **Slots 1/3/5 show no meaningful structure** (uniform high-entropy or pure padding) → naming impossible from this corpus; would need live capture. Fact: `dark_december_frzmove_br_coord_params_opaque`.
- (c) **Partial naming** (1-3 of 6 named) → name what's defensible, document why the rest stay unnamed. Fact: `dark_december_frzmove_br_coord_params_partial_<n>`.

## Success criteria — SINGLE TURN, do all before stopping

**Primary**: write `/home/sdancer/dark-december-coord-param-disambig/analysis/coord_param_disambig_2026-05-15.md` with:

1. **Load all 23,568 FRzMoveBr frames** + the 6,022 FRzMoveRq frames using `darkdec_decoder.py`. For each frame, extract the 6 coord_param f32 values at:
   - FRzMoveBr (33B Serialize): decoded offsets `[8, 12, 16, 20, 24, 28]` (Serialize offsets `[10, 14, 18, 22, 26, 30]` minus 2 for the dropped packet_id, minus 1 for adjacent-XOR collapse → `[7, 11, 15, 19, 23, 27]` actually — verify against cycle 403's `x@9, z@17, rot@25` empirical mapping).
   - FRzMoveRq (37B Serialize): same offsets +4 due to the inserted u32 `seq_or_move_kind`.

2. **Per-slot distribution analysis**:
   - `min / max / mean / std / median / p1 / p99`
   - `% finite, % near-zero (|v|<1e-6), % bounded |v|<2π, % bounded |v|<100`
   - `% constant (mode count / total)`
   - distinct-value count

3. **Range classification** per slot:
   - **coord-like** if range ≈ [-100000, 100000] and high distinct count → x / y / z position
   - **angle-like** if range ⊂ [-π, π] or [0, 2π] or [-180, 180] → yaw / pitch / roll
   - **velocity-like** if range bounded small with quantization → vel component
   - **padding-like** if mostly 0 or constant → reserved / unused

4. **Cross-slot correlation**: compute Pearson r² between (slot_i values) and (slot_j values) for all 15 pairs. If two slots are highly correlated, they may share semantics (e.g., x and dx).

5. **Per-actor consistency check**: group frames by `actor_or_move_handle`; for each actor with ≥10 frames, compute per-slot variance over time. If slot_5 has near-zero variance per actor but high cross-actor variance, it's likely an actor-level constant (e.g., movement-class ID).

6. **Compare FRzMoveBr vs FRzMoveRq** distributions for the same slot. If they match, the field has the same semantics in both directions. If they differ, document.

7. **Final naming table** with confidence:
   - coord_param_0 → `pos_x` (HIGH, baseline from cycle 403)
   - coord_param_1 → `<name>` (CONF, evidence)
   - coord_param_2 → `pos_z` (HIGH, baseline from cycle 403)
   - coord_param_3 → `<name>` (CONF, evidence)
   - coord_param_4 → `rot_yaw` (HIGH, baseline from cycle 403)
   - coord_param_5 → `<name>` (CONF, evidence)

8. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `COORD_PARAM_DISAMBIG_DONE` on the final line.

## Execution flow

**Step 1** — Load both frame sets using `darkdec_decoder.py` (cycle 383). Extract slots into a numpy/pandas frame.
**Step 2** — Per-slot stats.
**Step 3** — Range classification.
**Step 4** — Cross-slot correlation matrix.
**Step 5** — Per-actor variance test.
**Step 6** — Direction comparison.
**Step 7** — Naming table + verdict + fact-set.

## Constraints & gotchas
- **HARD memory budget: 500 MB.** 23,568 frames × 6 floats = small. Pandas/numpy is fine.
- **NO disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤400 lines.
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED** — do all 8 steps before stopping; do not pause at task boundaries.
- The cycle 403 `x@9, z@17, rot@25` mapping is empirical truth — anchor your slot offset calculations against that observation. The +8 wire framing model from cycle 412 + the FRzMoveBr 33-byte Serialize layout from cycle 415 must be consistent with cycle 403's empirical offsets.
- Cross-pollination memory: [[project_dark_december_wire_framing_plus8]] documents the +8 outer frame. [[project_dark_december_wire_decoder]] documents adjacent-XOR.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-coord-param-disambig/` (branch `coord-param-disambig`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- Baseline (cycle 403 empirical): `/home/sdancer/orchestrator/darkdec_output_streams/{player_track,entity_tracks}.csv`
- FRzMoveBr Serialize layout (cycle 415): `/home/sdancer/dark-december-frzmove-br-decode/analysis/frzmove_br_decode_2026-05-15.md`
- Rust typed decoder (cycle 422) — for cross-validation: `/home/sdancer/dark-december-rust-decoder-typed-v2/`
- C2S len=45 field map (cycle 395 — has heading/scalar slot data): `/home/sdancer/dark-december-c2s-len45-s2c-join/analysis/c2s_len45_fields_2026-05-15.md`
- success-fact key: `dark_december_frzmove_br_coord_params_<n>_named` (a)
- block-fact keys: `dark_december_frzmove_br_coord_params_opaque` (b), `dark_december_frzmove_br_coord_params_partial_<n>` (c)
