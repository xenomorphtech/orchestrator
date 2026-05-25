## typed-v9-medium-conf — Discovery + add medium-confidence catalog bindings (FRzRotationBr, FRzInteractionBr, FRzTeleportRp)

## Role & workdir
Rust extension worker. Workdir: `/home/sdancer/dark-december-typed-v9-medium-conf` (worktree of `/home/sdancer/darkdecember/`, branch `typed-v9-medium-conf`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_rust_typed_v9_medium_conf` (new)
- **sub_goal_key**: `discovery-plus-batch-add-medium-conf-bindings`

## Why this turn exists
Cycle 467 K=6 planner audit identified three unpromoted medium-confidence single-exact-match bindings from cycle-412 rz-binding catalog (`/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`):
- **`FRzRotationBr`** at S2C len=40 (Serialize=32, high-conf, exact unique match)
- **`FRzInteractionBr`** at S2C len=28 (Serialize=20, medium-conf, exact unique)
- **`FRzTeleportRp`** at S2C len=50 (Serialize=42, medium-conf, exact unique)

Planner rule: if v8-overlap closed ≥74.7%, run a discovery cycle to size these three and add the ones with non-trivial frame counts. v8 closed at 74.77% → proceed.

This is **discovery + batch add**: count frame populations first, then add only variants with ≥10 frames. If combined coverage delta is <0.4pp, the goal `dark_december_typed_coverage_saturated` should be marked next cycle.

## Hypothesis
At least 2 of the 3 candidate length classes (S2C len=40, len=28, len=50) carry ≥30 frames each in the first_quest corpus. Adding the corresponding variants lifts coverage 74.77% → ≥75.5% (+0.7pp combined). All v8 typed rows must remain unchanged.

## Falsification (3 outcomes)
- (a) **At least 2 variants added with ≥30 frames each + cargo build OK + typed coverage ≥75.5% + v8 regression check passes** → SUCCESS. Fact: `dark_december_decoder_rust_typed_v9_<coverage>_medium_conf_added`.
- (b) **Build breaks OR previously-typed frames change classification** → revert + report. Fact: `dark_december_decoder_rust_typed_v9_break`.
- (c) **Discovery finds insufficient frames (≤30 per bucket OR combined <0.4pp lift)** → mark `dark_december_typed_coverage_saturated` as fact value AND report what was found. Fact: `dark_december_decoder_rust_typed_v9_saturated`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v8-overlap baseline** from `/home/sdancer/dark-december-typed-v8-overlap/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Discovery sub-step (≤3 min)**: count frames at S2C len=40, len=28, len=50 in `first_quest_s2c.tcpstream.bin`. Report:
   - Frame count per length class.
   - Top-5 `decoded[0..4]` prefixes per class to verify single-shape assumption (or detect multiplexing).
   - If any class has <10 frames OR shows >3 dominant prefixes (multiplexed), SKIP it.

3. **Add eligible variants** in `src/lib.rs` (only the classes that pass discovery thresholds):
   ```rust
   FRzRotationBr   { tail: Vec<u8> },  // S2C len=40
   FRzInteractionBr{ tail: Vec<u8> },  // S2C len=28
   FRzTeleportRp   { tail: Vec<u8> },  // S2C len=50
   ```
   - Classifier rules:
     - `direction == S2C && raw.len() == 40 && decoded.len() == 33` → FRzRotationBr
     - `direction == S2C && raw.len() == 28 && decoded.len() == 21` → FRzInteractionBr
     - `direction == S2C && raw.len() == 50 && decoded.len() == 43` → FRzTeleportRp
   - Annotate `// MEDIUM (cycle-412 size-only binding; FRAGILE on internal field interpretation)` for each.
   - Match-arms for `id()`/`x()`/`z()`/`rot()` return `None`.

4. **Validate**: cargo build --release; run on `first_quest_{c2s,s2c}.tcpstream.bin`; compute typed coverage. Print:
   - Total frames, typed frames, coverage %.
   - New variant frame counts.
   - Confirmation that v8-typed-variant frame counts are UNCHANGED.

5. **Diff vs v8-typed CSV** (regression check): `out_v8_baseline/typed_packets.csv` vs `out_v9_orch/typed_packets.csv` — every row present in v8 must be present in v9 with identical fields; v9 only ADDS rows.

6. **Write `analysis/typed_v9_medium_conf_2026-05-16.md`** (≤120 lines):
   - Discovery table: 3 length classes × {count, top-5 prefixes, dominant-shape verdict}.
   - Eligibility decision per class.
   - Coverage table (v8 74.77% → v9 X.XX%).
   - Per-variant frame counts.
   - Regression confirmation.
   - **Saturation verdict**: if combined lift <0.4pp, recommend marking goal saturated next cycle.

7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_V9_MEDIUM_CONF_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤120 lines.
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **IMPORTANT (cycle-465 stall lesson)**: After writing the artifact, fact-set, and final TYPED_V9_MEDIUM_CONF_DONE marker, EXIT IMMEDIATELY.
- Do NOT change v8 variant classifications.
- The lengths use effective-`+8` framing (Serialize+8 = wire_len): 32+8=40, 20+8=28, 42+8=50.

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v8-overlap/`
- Cycle-412 rz-binding analysis: `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md` (rows for FRzRotationBr, FRzInteractionBr, FRzTeleportRp)
- Cycle-468 v8-overlap artifact: `/home/sdancer/dark-december-typed-v8-overlap/analysis/typed_v8_overlap_2026-05-16.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- success-fact: `dark_december_decoder_rust_typed_v9_<coverage>_medium_conf_added` (a)
- block-facts: `dark_december_decoder_rust_typed_v9_break` (b), `dark_december_decoder_rust_typed_v9_saturated` (c)
