## typed-test-suite — Add cargo regression test + JSON exporter to typed Rust decoder

## Role & workdir
Rust engineering worker. Workdir: `/home/sdancer/dark-december-typed-test-suite` (worktree of `/home/sdancer/darkdecember/`, branch `typed-test-suite`).

## Current goal / sub-goal
- **goal_key**: `dark_december_decoder_test_suite_and_json` (new)
- **sub_goal_key**: `add-regression-test-and-json-export`

## Why this turn exists
Typed Rust decoder saturated at 96.13% across 57 variants (cycle 480). Engineering hygiene work makes the decoder usable for downstream consumers (minimap-sniffer, replay tooling, web visualizers):

1. **Regression test** — assert byte-exact CSV equivalence between current decoder run and a frozen baseline. Without this, future variant edits could silently regress earlier closures.
2. **JSON exporter** — `serde_json` output for each typed Packet so non-Rust consumers can use the decoder's output stream.

Both are pure engineering with bounded scope; neither moves the coverage metric.

## Hypothesis
Adding `#[derive(Serialize)]` to Packet + a `--emit-json` flag + a `tests/replay_byte_exact.rs` integration test using the v11 baseline CSV as ground truth ships in one turn. Existing replay behavior is unchanged.

## Falsification (3 outcomes)
- (a) **Both deliverables landed: cargo test passes, --emit-json flag produces valid JSON, original CSV output unchanged byte-for-byte** → SUCCESS. Fact: `dark_december_decoder_test_suite_and_json_shipped`.
- (b) **Build breaks OR existing CSV output changes** → revert + report. Fact: `dark_december_decoder_test_suite_break`.
- (c) **One of the two deliverables works but the other doesn't** → name what's defensible. Fact: `dark_december_decoder_test_suite_partial`.

## Success criteria — SINGLE TURN

**Primary**:

1. **Copy v11 baseline** from `/home/sdancer/dark-december-typed-v11-long-tail/` (Cargo.toml, Cargo.lock, src/, .gitignore). Verify `cargo build --release` succeeds.

2. **Add serde dependency** to Cargo.toml (`serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`). Bump version if appropriate.

3. **Derive `Serialize`** for `Packet`, `Direction`, and any associated types. For `Vec<u8>` fields, configure base64 or hex encoding (whichever is cleaner — pick one).

4. **Add `--emit-json` CLI flag** that prints one JSON-line-per-packet to stdout instead of CSV. Existing default behavior (CSV emission) MUST be unchanged.

5. **Write `tests/replay_byte_exact.rs`**:
   - Capture the current CSV output of `cargo run --release -- --c2s first_quest_c2s.tcpstream.bin --s2c first_quest_s2c.tcpstream.bin` into a baseline file at `tests/fixtures/typed_packets_v11_baseline.csv`.
   - Test asserts: re-running produces identical CSV bytes.
   - Test asserts: total typed frames == 46,129 (the v11 fact); FRzMoveBr count == 23,568; FRzC2SLen10Placeholder count == 3,607.

6. **Validate**:
   - `cargo build --release` ✓
   - `cargo test --release` ✓ (all tests pass)
   - Run with `--emit-json` on a small subset; pipe to `jq` if available to validate JSON.
   - Diff CSV output before/after — must be byte-identical.

7. **Write `analysis/test_suite_and_json_2026-05-16.md`** (≤80 lines):
   - What was added (test, JSON flag).
   - Build + test results.
   - JSON sample (3 packet types).
   - Note any rough edges (e.g., binary-tail encoding choice).

8. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`. Print `TYPED_TEST_SUITE_DONE`.

## Constraints
- **HARD memory budget: 500 MB.**
- **NO new RE work. NO variant additions. This is engineering hygiene only.**
- **HARD output cap**: artifact ≤80 lines.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- **IMPORTANT**: After writing the artifact, fact-set, and final TYPED_TEST_SUITE_DONE marker, EXIT IMMEDIATELY.
- The test must be self-contained: it must capture the baseline at first-run if missing, then assert equality on subsequent runs. OR commit the baseline CSV as a test fixture.
- Existing CSV output MUST NOT change. Verify with diff before declaring success.

## References
- Source crate (COPY FROM): `/home/sdancer/dark-december-typed-v11-long-tail/`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_{c2s,s2c}.tcpstream.bin`
- v11 fact: `dark_december_decoder_rust_typed_v11_96_13_long_tail_added` (verify count assertions match)
- success-fact: `dark_december_decoder_test_suite_and_json_shipped` (a)
- block-facts: `dark_december_decoder_test_suite_break` (b), `dark_december_decoder_test_suite_partial` (c)
