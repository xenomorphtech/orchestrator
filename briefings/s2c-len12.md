## s2c-len12 — Quick analysis of the 588-frame S2C len=12 ack_fixed_5b bucket

## Role & workdir
Pure-Python worker. Workdir: `/home/sdancer/dark-december-s2c-len12` (worktree of `/home/sdancer/darkdecember/`, branch `s2c-len12`).

## Current goal / sub-goal
- **goal_key**: `dark_december_s2c_len12_decode` (new)
- **sub_goal_key**: `field-decode-the-588-frame-ack-bucket`

## Why this turn exists
Cycle 408's S2C inventory documented `len=12 / 588 frames / dominant prefix 12030000 at 93.9%` as the third-largest S2C class. Highly homogeneous = easy to field-decode. Cycle 412 placed this under "low confidence" with no binding. Quick win: decode the 5-byte body field-by-field and bind a typename.

## Hypothesis
The 588 len=12 frames are server ACKs to client requests. Body = `msg_subtype u8 + 4-byte payload`. The 4 payload bytes are likely an entity_id u32 or seq_num u32.

## Falsification (3 outcomes)
- (a) **All 5 body bytes named at type level + temporal lift binds to a specific C2S request class** → SUCCESS. Fact: `dark_december_s2c_len12_named`.
- (b) **Bytes uniformly high-entropy** → opaque. Fact: `dark_december_s2c_len12_opaque`.
- (c) **Partial** → name what's defensible. Fact: `dark_december_s2c_len12_partial`.

## Success criteria — SINGLE TURN

**Primary**: write `/home/sdancer/dark-december-s2c-len12/analysis/s2c_len12_2026-05-15.md` with:

1. **Filter 588 S2C len=12 frames** from `streams/first_quest/first_quest_s2c.tcpstream.bin` via `darkdec_decoder.py`. Confirm decoded length = 5 bytes.
2. **Per-byte variance** dec[0..4]: distinct count, modal value, top-3.
3. **u32 sweep** at dec[1..5]: interpret as u32 LE; check if values match any entity_id from `/home/sdancer/orchestrator/darkdec_output_streams/entities_summary.csv`.
4. **Temporal lift** against C2S length classes (use cycle-412 c2s_s2c_pairs.csv): which C2S class precedes len=12 within ±20 ordinals? High lift = ACK class.
5. **Bind to GS_CL Rp/Notify**: search cycle-412 typename catalog for fixed-size Rps with Serialize = 5 (under +7 framing).
6. Verdict (a)/(b)/(c) + closing fact + `S2C_LEN12_DONE`.

## Constraints
- HARD memory: 100 MB. 588 frames trivial.
- ONE Codex turn ≤10 min. SINGLE-TURN COMPLETION.
- Use cycle-435 corrected framing.

## References
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- Cycle-408 inventory: `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
- Cycle-412 binding: `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`
- Cycle-412 join lift: `/home/sdancer/dark-december-c2s-s2c-join/analysis/c2s_s2c_pairs.csv`
- Entity ID reference: `/home/sdancer/orchestrator/darkdec_output_streams/entities_summary.csv`
- 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
