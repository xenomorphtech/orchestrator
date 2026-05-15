# xor-field-interp — Decode with masked-chained model + question the field-type assumption

## Role & workdir
Pure-Python decoding + structural-analysis worker. Workdir: `/home/sdancer/dark-december-xor-field-interp`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `decode-and-determine-actual-field-types`

## Why this turn exists
- Cycle 346 `move-synthesize`: wire layout 100% — 37B (Rq) / 33B (Br) with packet IDs 0x0385/0x0386. Fields labeled "f32/u32" (UNCERTAIN type).
- Cycle 347 `xor-key-recover`: simple cyclic XOR ruled out.
- Cycle 349 `xor-chained-state`: pure chained XOR ruled out; revealed c0d2/c3d1 cipher diff is `03 03 03 03 03 03 03 03 03 03`.
- Cycle 352 `xor-mask-layer`: tested chained XOR + per-byte packet-id-low-byte mask. The mask explained the `03` signature DECISIVELY (only `mask=packet_id_low_byte` gives 03-everywhere). BUT 512 (mask, msg_type) combinations failed because every server frame had a "invariant float slot" producing implausible floats (e+38, e-30, etc.).

**Critical insight**: the falsification rests on interpreting bytes 14-17 (and similar 4-byte slots) as **float32 coordinates**. The move-decode briefing explicitly labeled them `f32/u32` — uncertain. **If they're actually u32** (entity sub-IDs, packed quantized angles, bit flags, timestamps, etc.), the "float implausibility" falsification is invalid and the masked-chained model is correct.

## Hypothesis
The wire cipher IS masked chained XOR: `c_wire[i] = (chained_xor(p))[i] XOR packet_id_low_byte`. The fields are NOT all f32 — at least some are u32. Decoding under this model with the correct field types yields plausible, structurally-consistent plaintext across all 22 long frames.

Specifically: under chained XOR with k[0]=0xc0, k[1]=0xd1 (forced by packet ID 0x0385 + mask 0x85), and 6 unknown h0..h5 (low handle bytes), we can:
1. Compute the FULL plaintext for each c0d2 frame as a function of h0..h5
2. Examine each 4-byte field position's plaintext: does it vary smoothly across frames (likely float) or vary discretely with structure (likely u32 / packed)?
3. Reconcile the "invariant float slot" of prior worker: re-interpret as u32 and check if values cluster meaningfully (e.g. all in same range, same low bits, etc.)

## Falsification (3 outcomes)
- (a) **Field type re-interpretation yields plausible plaintext under masked-chained model** (e.g. position 0 = f32 coord X with varying smooth values across frames; positions 1-2 = u32 with structured bits; etc.) → SUCCESS. Fact: `dark_december_wire_xor_field_types_corrected_decoded_<n>`.
- (b) **All field positions fail BOTH f32 and u32 plausibility (high entropy / no structure)** under any choice of h0..h5 → cipher has a non-XOR component beyond the mask. Fact: `dark_december_wire_xor_mask_non_xor_confirmed`.
- (c) **Field type ambiguous (some positions plausible, others not, no clean cut)** → need ground-truth game data to disambiguate (e.g. live capture with known player positions). Fact: `dark_december_wire_xor_field_types_ambiguous`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-xor-field-interp/analysis/field_interp_2026-05-15.md` with:
1. **Restate the masked-chained model** with k[0]=0xc0, k[1]=0xd1 (Stand) or k[0]=0xc3, k[1]=0xd2 (Move). Derive k[2..7] symbolically as functions of h0..h5 (the unknown handle bytes).
2. **For each 4-byte field position** (offsets 10, 14, 18, 22, 26 in broadcast frames; 14, 18, 22, 26, 30 in request frames): show the PLAINTEXT byte pattern as a function of (h0..h5) for the 9-frame c0d2 family.
3. **Identify which positions are CONSTANT across the c0d2 family** (same plaintext bytes in 9/9 frames). Those positions have a fixed value tied to the actor handle — strong evidence the actor is the same across all 9 frames.
4. **For positions that VARY across the c0d2 family**: examine the plaintext byte distribution. If consecutive byte values differ by small amounts → likely float coord (player moving smoothly). If bytes are random-looking → either truly random fields OR cipher is wrong.
5. **For each field position, attempt BOTH interpretations**: (i) f32 plausibility, (ii) u32 structural patterns (e.g. monotonic counters, small ints, bit-packed data, common high bytes).
6. **Cross-check with c3d1**: under the model, the 2 c3d1 frames should have the same handle bytes 2..9 as the c0d2 family. The mask shifts byte 8 (since k[0] is now 0xc3 not 0xc0), so handle byte 6 = `0xea ^ h5` instead of `0xe9 ^ h5`. **This implies the c3d1 frames are about a DIFFERENT entity** (different handle), not the same one. Re-examine that assumption.
7. **Final verdict** with reasoning, plus the closing fact via `harness fact-set`.

Print `XOR_FIELD_INTERP_DONE` on the final line.

## Execution flow — atomic, ≤20 min wall time, 500 MB

**Step 1** — Load frames.jsonl; reconstruct the masked-chained recurrence; reproduce the cycle-352 worker's per-frame `normalized masked prefix` to verify model parameters.

**Step 2** — For the 9-frame c0d2 family, compute the symbolic plaintext for all 35 bytes as a function of (h0..h5). The h0..h5 form a 6-byte free-parameter space; everything else is determined.

**Step 3** — Discriminate fields by VARIANCE across the c0d2 family:
- Positions where ALL 9 frames have the same masked ciphertext → constant plaintext → fixed-value field (handle, message-class flags, etc.)
- Positions where frames differ → variable field (timestamps, coordinates, sequence counters)

**Step 4** — For each VARIABLE-position 4-byte slot, try both interpretations:
- **As f32**: collect the 9 plaintext values (for each candidate h0..h5), check if they form a smooth trajectory (e.g. for a moving player, consecutive coord values differ by ~ 1-100 units max).
- **As u32**: check for monotonic structure (timestamps grow), small ranges (0..1000 ticks), or bit-packed structure (e.g. low byte is a flag, upper 24 bits are an ID).

**Step 5** — The prior worker's "invariant float slot 1" is bytes 14-17. Compute its plaintext under the masked-chained model:
- For c0d2 family: bytes 14-17 of c_masked are constant (e.g. `10 f5 b9 49` per the cycle-352 artifact). Plaintext bytes 14-17 = function of (h0..h5) via the chained recurrence.
- Try: assume position 14-17 is u32 timestamp/counter. The 9 frames span time → counter values should be monotonically increasing or have structured spacing.

**Step 6** — Decisive test: pick h0..h5 values that minimize the "implausibility" cost function summed over ALL field positions AND all 22 frames simultaneously. If a clear local minimum exists with sensible values, that's the recovered handle and the model is validated.

**Step 7** — Write artifact + set closing fact + DONE.

## Constraints & gotchas
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO new memdump. NO pyelftools. NO Frida. NO live device. NO MCP aeon.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤20 min wall time.**
- The prior worker's invariant slot analysis at `/home/sdancer/dark-december-xor-mask-layer/analysis/mask_layer_2026-05-15.md` (especially lines 40-72) is the GROUND TRUTH constraint to overcome — those rows show which slots looked implausible *as floats*. Re-examining them as u32 is the explicit test.
- Brute-force over h0..h5 (2^48) is too large — do NOT attempt full sweep. Use structural constraints to narrow each h_i to small candidate sets (e.g. assume top handle byte is 0x00 OR a small set; assume timestamp-byte values are monotonic; etc.).

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-xor-field-interp/`
- Frame corpus: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` (47 frames; 11 client_39 + 11 server_35)
- Move/Stand layout: `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md` (NOTE: types labeled `f32/u32` — uncertainty is real)
- Prior mask-layer analysis (READ — has the full c_masked normalization): `/home/sdancer/dark-december-xor-mask-layer/analysis/mask_layer_2026-05-15.md`
- Prior chained-state analysis: `/home/sdancer/dark-december-xor-chained-state/analysis/chained_state_recover_2026-05-15.md`
- success-fact key: `dark_december_wire_xor_field_types_corrected_decoded_<n>` (a)
- block-fact keys: `dark_december_wire_xor_mask_non_xor_confirmed` (b), `dark_december_wire_xor_field_types_ambiguous` (c)
