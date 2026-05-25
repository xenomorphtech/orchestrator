## s2c-len39 — Sub-bucket the 4,110-frame S2C len=39 movement_event_reply_mixed bucket

## Role & workdir
Pure-Python statistical worker. Workdir: `/home/sdancer/dark-december-s2c-len39` (worktree of `/home/sdancer/darkdecember/`, branch `s2c-len39`).

## Current goal / sub-goal
- **goal_key**: `dark_december_s2c_len39_subbuckets` (new)
- **sub_goal_key**: `disambiguate-the-2nd-largest-s2c-bucket`

## Why this turn exists
Cycle 408's s2c-inventory left the **S2C len=39 / 4,110-frame** bucket as the second-largest unresolved class (after len=41). Cycle 408 noted it has **147 distinct decoded prefixes** with the dominant `8400e1e2` carrying only 12.8% of the bucket — highly multiplexed. No follow-up cycle has decomposed this bucket.

This is fresh substrate, independent of the cycle-435 framing controversy, and would explain ~11.7% of S2C frames if successfully sub-bucketed.

## Hypothesis
The 4,110 S2C len=39 frames partition into ~5-10 sub-buckets by 4-byte decoded prefix, each ≥100 frames, where each sub-bucket has internal byte-position regularity matching the cycle-395 / cycle-408 signature pattern. Top sub-buckets correspond to broadcast variants (Br, Noti, Rp) sharing the same Serialize size (32 bytes decoded = 7B wire overhead).

## Falsification (3 outcomes)
- (a) **≥5 sub-buckets with ≥100 frames each, byte-position regularity within each** → SUCCESS. Fact: `dark_december_s2c_len39_<n>_subbuckets_named`.
- (b) **No sub-bucket clears 5% of the parent (~205 frames)** → bucket is too heterogeneous; all 147 prefixes are independent. Fact: `dark_december_s2c_len39_pure_multiplexed`.
- (c) **Partial** (2-4 sub-buckets) → name what's defensible. Fact: `dark_december_s2c_len39_partial_<n>_subbuckets`.

## Success criteria — SINGLE TURN

**Primary**: write `/home/sdancer/dark-december-s2c-len39/analysis/s2c_len39_2026-05-15.md` with:

1. **Filter S2C len=39 frames** from `streams/first_quest/first_quest_s2c.tcpstream.bin` using `darkdec_decoder.py`. Confirm 4,110 frames, decoded length = 32 bytes.
2. **Sub-bucket by decoded 4-byte prefix** (dec[0..4]). Histogram. Top-10 prefixes with counts + share.
3. **For each sub-bucket with ≥50 frames**: per-byte variance table (cycle-390 / cycle-408 methodology). Identify low-variance positions = type/subtype codes.
4. **Cross-reference with C2S len=10/45 ACK timing**: for each sub-bucket, compute temporal lift against C2S len=10 (3607 frames, FRzCharacterLookVisibleChangeRq) and C2S len=45 (FRzMoveRq). High lift = response to that request class.
5. **ASCII detection**: for each sub-bucket, scan for ≥4-char printable runs (chat / NPC dialog content).
6. **Candidate semantic names** per sub-bucket (e.g. `move_ack`, `interact_result`, `aura_state_change`, etc.).
7. **Output CSV** `analysis/s2c_len39_subbuckets.csv` with columns: prefix_hex, frame_count, top5_modal_bytes, lift_vs_c2s_len10, lift_vs_c2s_len45, ascii_runs, candidate_name.
8. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `S2C_LEN39_DONE` on the final line.

## Constraints & gotchas
- **HARD memory budget: 300 MB.** 4,110 frames × 32 bytes is small.
- **NO disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤300 lines.
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Apply cycle-435 corrected framing: the decoded view IS the protocol payload. Decoded len=32 = Serialize body for these messages.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-s2c-len39/` (branch `s2c-len39`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- Cycle-408 s2c-inventory: `/home/sdancer/dark-december-s2c-inventory/analysis/s2c_inventory_2026-05-15.md`
- Cycle-412 c2s-s2c-join lift matrix: `/home/sdancer/dark-december-c2s-s2c-join/analysis/c2s_s2c_pairs.csv`
- Cycle-435 framing memory: `[[project_dark_december_wire_framing_plus8]]` (REVISED)
- 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- success-fact key: `dark_december_s2c_len39_<n>_subbuckets_named` (a)
- block-fact keys: `dark_december_s2c_len39_pure_multiplexed` (b), `dark_december_s2c_len39_partial_<n>_subbuckets` (c)
