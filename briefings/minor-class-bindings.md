## minor-class-bindings — Bind the 21 C2S minor-class buckets to Rz typenames

## Role & workdir
Pure-Python statistical worker. Workdir: `/home/sdancer/dark-december-minor-class-bindings` (worktree of `/home/sdancer/darkdecember/`, branch `minor-class-bindings`).

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_minor_class_typename_bindings` (new)
- **sub_goal_key**: `match-cycle-397-buckets-to-rz-serialize-sizes`

## Why this turn exists
Cycle 397 closed `dark_december_c2s_minor_classes` with 21 named buckets covering 2090/2146 frames (97.4%). Cycle 412 used a +8 framing model (later corrected to +7 by cycle 435) to bind 5 high-confidence Rz typenames + 10 medium. The 21 minor-class buckets from cycle 397 were never bound to specific typenames.

The corrected +7 framing model lets us re-run the cycle-412 size-match methodology on the minor buckets. Each bucket has a wire length; predicted Serialize size = wire_len - 7. Match against the 822-typename catalog from rz-typename-binding to identify candidates.

## Hypothesis
At least 5 of the 21 cycle-397 minor-class buckets bind cleanly (unique Serialize size match in the 421 CL_GS Rq catalog) to specific typenames, with confidence ≥ medium.

## Falsification (3 outcomes)
- (a) **≥5 minor buckets bound at medium-or-high confidence** → SUCCESS. Fact: `dark_december_c2s_minor_class_<n>_bound`.
- (b) **No minor bucket clears unique size match** → +7 framing or the cycle-412 catalog is too narrow to bind these. Fact: `dark_december_c2s_minor_classes_unbindable`.
- (c) **Partial** (1-4 bindings) → name what's defensible. Fact: `dark_december_c2s_minor_class_partial_<n>`.

## Success criteria — SINGLE TURN

**Primary**: write `/home/sdancer/dark-december-minor-class-bindings/analysis/minor_class_bindings_2026-05-15.md` with:

1. **Load cycle-397 buckets** from `/home/sdancer/dark-december-c2s-minor-classes/analysis/c2s_minor_classes_2026-05-15.md`. Extract the 21 (wire_length, count, bucket_name) tuples.
2. **Load cycle-412's typename Serialize-size table** from `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_support_2026-05-15.json`. Filter to CL_GS Rq entries with fixed Serialize sizes.
3. **Apply corrected +7 framing**: for each bucket wire_length, compute predicted_serialize_size = wire_length - 7.
4. **Match table**: for each bucket, list all CL_GS Rqs whose Serialize size equals predicted_serialize_size. Confidence:
   - HIGH: unique match in the catalog + signature byte matches (e.g., dominant decoded prefix matches a known msg_subtype).
   - MEDIUM: 2-3 matches in the catalog.
   - LOW: 4+ matches OR the dominant prefix doesn't disambiguate.
5. **Output `analysis/minor_class_bindings.csv`** with columns: bucket_name, wire_length, frame_count, predicted_serialize_size, candidates (semicolon-separated), confidence.
6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `MINOR_CLASS_BINDINGS_DONE` on the final line.

## Constraints & gotchas
- **HARD memory budget: 250 MB.** Pure table lookup.
- **NO disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤300 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Use the cycle-435 corrected framing: `wire_len = Serialize_size + 7`. Do NOT use the cycle-412 +8 model.
- For the size catalog: if cycle-412's JSON encodes sizes under the +8 model (i.e., predicted_wire_len column), subtract 1 to get the Serialize size.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-minor-class-bindings/` (branch `minor-class-bindings`).
- Cycle-397 minor classes: `/home/sdancer/dark-december-c2s-minor-classes/analysis/c2s_minor_classes_2026-05-15.md`
- Cycle-412 typename Serialize sizes: `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_support_2026-05-15.json` + `rz_binding_2026-05-15.md`
- Cycle-435 framing correction: `/home/sdancer/dark-december-raw-reconstruct/analysis/raw_reconstruct_2026-05-15.md`
- Cycle-444 protocol wiki: `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md`
- 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- success-fact key: `dark_december_c2s_minor_class_<n>_bound` (a)
- block-fact keys: `dark_december_c2s_minor_classes_unbindable` (b), `dark_december_c2s_minor_class_partial_<n>` (c)
