# cert-rust-validate — Validation worker for cert-rust-repro

## Role & workdir
You are a **validation worker** for the Rust reproducer at `/home/sdancer/nmss-emu/cert-rust-repro/`. You verify the parameterized `try_cert_token_for_challenge` API used in `cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs` by writing additional Rust unit tests that confirm core behavioral invariants.

**Workdir:** `/home/sdancer/nmss-emu/cert-rust-repro`

**Note (operational):** This worker is the **live-migration test** for `codex_app_server` mode. The orchestrator will audit your turns for clean reporting and continuity across two prompts. Be precise and concise.

## Reference file (read end-to-end first)
`cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs`

That existing test demonstrates the parameterized API:
- Constructs a `CertSnapshotState` from on-disk JSON checkpoints (`record_payloads_ascii32`, `selector_rows`, `outer_ctx_first32`, `traffic_upd_*`).
- Calls `try_cert_token_for_challenge(challenge, &snapshot_state, &side_x0) -> Option<CertToken>`.
- `side_x0` is a `[u8; 44]` byte array; bytes `[36..40]` and `[40..44]` carry the two `u32_le` selector fields (`field36`, `field40`) consumed by `select_precursor_primary_from_side_x0`.

The relevant exported symbols live in `cert_rust_repro` (the crate root re-exports from `native_oracle`):
- `try_cert_token_for_challenge`
- `CertSnapshotState`, `CertToken`, `OfflineSelectorRow`, `TrafficUpdLabelMode`

## Goal
Add a **new** Rust test file under `cert-rust-repro/tests/` (e.g. `cert_validate_determinism.rs`) containing two `#[test]` functions that prove:

1. **Determinism** — calling `try_cert_token_for_challenge` twice with the **same** `challenge`, snapshot state, and `side_x0` produces **identical** `CertToken` output bytes (compare by field equality on the returned `CertToken`, e.g. via `PartialEq`/`Eq` already derived on `CertToken`, or by `serde_json::to_vec` of both tokens).

2. **Distinctness** — calling with **two different** `side_x0` values (same challenge, same snapshot) produces **different** `CertToken` outputs. Pick concrete byte values that vary the selector fields at `[36..40]` / `[40..44]` so they exercise distinct `(field36, field40)` selections (the existing test uses `(0x1, 0x4)`; pick two **other** valid pairs from `classify_field36`/`classify_field40` in `src/native_oracle/stages/stage_offline_cert_token.rs`).

You may copy the snapshot-state construction helpers (`parse_record_payloads_ascii32`, `parse_selector_rows`, the JSON path constants) from the reference test into your new file — that is fine.

## Constraints (HARD)
- **Do NOT modify** `cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs` or any source file under `cert-rust-repro/src/`. Only **add** new files.
- **Do NOT modify** `Cargo.toml`. Integration tests under `tests/` are auto-discovered.
- Keep the new test file self-contained — duplicate the parsing helpers if needed.
- Use `CHALLENGE_7BDA = "7BDA93D2F45D36C0"` (the same challenge as the reference test) so the snapshot state is known to be a valid input.

## Run & report
1. From `/home/sdancer/nmss-emu/cert-rust-repro/`, run:
   ```
   cargo test --test <new_test_file_stem> 2>&1 | tail -60
   ```
   then also confirm the full suite still builds:
   ```
   cargo test --test cert_offline_reproducer_pending_x0 2>&1 | tail -10
   ```
2. Report:
   - Whether both new tests **PASSED**.
   - The exact `cargo test` exit code.
   - The last ~30 lines of `cargo test` output.
3. If a compile error reveals an API mismatch, fix **only** your new test file (never source). If the API truly does not support what you proposed, note the gap clearly and stop — do not invent workarounds.

## Done criteria
- One new test file added under `cert-rust-repro/tests/`.
- Two new `#[test]` functions: one determinism, one distinctness.
- `cargo test --test <new_stem>` exits **0** with both tests in the PASS column.
- Existing `cert_offline_reproducer_pending_x0` test still passes.
