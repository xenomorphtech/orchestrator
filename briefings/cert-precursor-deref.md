# cert-precursor-deref — Hypothesis test for cert lane HKDF precursor

## Role
Hypothesis-test worker for the cert lane. cert-rust-reimpl plateaued at 17/64 by treating field36/field40 as raw u32 precursor bytes. NEW HYPOTHESIS: function `0x57ccc4` uses field36/field40 to INDEX a descriptor table; the actual precursor lives at the dereferenced payload pointer. Test this by walking ONE descriptor entry end-to-end.

## Goal
Walk descriptor at module-relative `0x4c21a8` in F7B3 module → read its payload pointer (expected `0x78cc5f2950`) → extract the precursor bytes there → run them through HKDF-Expand-Label → SHA256 with raw32_13 → compare against the 7BDA anchor `BECA86489D2D6F7E305BEF0116BAEF2FDC4FE34B035E8D1F1D880D6D8C6F42D5`.

## Workdir
`/tmp/cert-precursor-deref-6fa7ca84/` — all writes here only. Read-only on `/home/sdancer/nmss-emu/`.

## Inputs (pre-located by orchestrator — confirm but don't redo from scratch)

- **F7B3 module base VA**: `0x78cc288000` (from `trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/maps.txt`, file `F7B3B00F8A5`).
- **Descriptor VA** (`module_base + 0x4c21a8`): `0x78cc74a1a8`.
  - Falls within map `78cc738000-78cc76c000 r--p 004b0000`.
  - Snapshot shard: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/memdump/78cc738000.bin` (212992 bytes = 0x34000).
  - File offset for descriptor: `0x78cc74a1a8 - 0x78cc738000 = 0x121a8`.
- **Expected payload pointer (VA)**: `0x78cc5f2950`.
  - Falls within map `78cc408000-78cc738000 r-xp 00180000`.
  - Snapshot shard: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/memdump/78cc408000.bin` (3342336 bytes = 0x330000).
  - File offset for payload: `0x78cc5f2950 - 0x78cc408000 = 0x1ea950`.
- **Outer ctx (32 bytes)**: `10890ae5790000b410810ae5790000b4089405c57a0000b4010907f51c000000` (from `cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs` line 89).
- **7BDA anchor (post-SHA256, upper hex)**: `BECA86489D2D6F7E305BEF0116BAEF2FDC4FE34B035E8D1F1D880D6D8C6F42D5`.
- **raw32_after_13 for 7BDA (hex)**: `EA860C1F7F07BE6C279FF6227960AAE0EE123C074CCB860DF76ACB55AC4E353D` (from `cert-rust-repro/tests/phase_d_hkdf_sweep_v2.rs` line 19-20).
- **HKDF output length**: `8` bytes (NOT 32 — the live recipe uses `traffic_upd_output_len = 8`).
- **HKDF label**: literal raw `"traffic upd"` (`TrafficUpdLabelMode::Raw` — NO `tls13 ` prefix in this lane).

## Recipe (confirmed in cert-rust-repro)
```
final_piece   = HKDF-Expand-Label(prk=precursor, label="traffic upd", ctx=outer_ctx_first32, len=8)
final_raw32   = SHA256(final_piece || raw32_after_13)
final_upper64 = hex_upper(final_raw32)   # must equal 7BDA anchor for PASS
```

`HkdfLabel` = `u16(out_len, BE) || u8(label_len) || label_bytes || u8(ctx_len) || ctx_bytes`. RFC 5869 HKDF-Expand with HMAC-SHA256.

## Two-turn structure

### Turn 1 — read snapshot shards, extract candidate precursor
1. Confirm shard `78cc738000.bin` has size 212992; read 24 bytes at offset `0x121a8`.
2. Verify the four u32s little-endian are **5, 1, 16, 0** (header). Verify u64 at offset 16 of the descriptor (file offset `0x121a8 + 16 = 0x121b8`) equals `0x78cc5f2950`. If not, REPORT what you actually see — DO NOT invent.
3. Confirm shard `78cc408000.bin` has size 3342336; read 64 bytes at offset `0x1ea950`.
4. Report descriptor header hex (16 bytes), pointer u64 hex, and the 64-byte candidate precursor block.
5. One-paragraph assessment: does it look like real key material (high entropy, no obvious code/zeros) or junk?

DO NOT run HKDF in turn 1. DO NOT write a Cargo project yet. Pure read.

### Turn 2 — implement and verify
1. Use the precursor bytes from turn 1's report (do NOT re-read shards).
2. In `/tmp/cert-precursor-deref-6fa7ca84/`, scaffold a Rust binary. Easiest: `Cargo.toml` with `[dependencies] cert-rust-repro = { path = "/home/sdancer/nmss-emu/cert-rust-repro" }` and reuse its `hkdf_expand_label` infra. OR just use `hmac = "0.12"` + `sha2 = "0.10"` and inline the 25-line HKDF-Expand-Label.
3. Compute `final_piece = HKDF-Expand-Label(prk=precursor_bytes, label="traffic upd", ctx=outer_ctx, len=8)`.
4. Compute `final_raw32 = SHA256(final_piece || raw32_after_13_bytes)`.
5. Compare upper-hex to the 7BDA anchor. Print both side by side.
6. **Try precursor lengths 8, 16, 32, 64** (4 candidates from the dereferenced page) — report all four results.

### Verdict
- **PASS** = full 64-hex-char match against `BECA8648...42D5`.
- **NEAR_MISS** = leading-byte match (`BECA8648...` matches but tail differs — even 4-8 leading hex chars are meaningful).
- **MISS** = no meaningful prefix match.

If MISS or NEAR_MISS, propose ONE specific next refinement (one paragraph): different descriptor entry / different label / different ctx slice / different precursor offset within the page / different precursor length.

## Reference files (read-only)
- `/home/sdancer/nmss-emu/cert-rust-repro/tests/cert_offline_reproducer_pending_x0.rs` — has the API + outer_ctx fixture.
- `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_offline_cert_token.rs` — has `hkdf_expand_label` + full recipe (lines 195-252).
- `/home/sdancer/nmss-emu/cert-rust-repro/tests/phase_d_hkdf_sweep_v2.rs` — has raw32_13 + anchor constants.
- `/home/sdancer/nmss-emu/analysis/checkpoints/cert_offline_reproduction_status_2026-05-03.md` — current state.

## Constraints
- READ-ONLY on `/home/sdancer/nmss-emu/` (snapshots and source).
- All cargo work, scratch files, output artifacts inside `/tmp/cert-precursor-deref-6fa7ca84/`.
- DO NOT modify cert-rust-repro source.
- DO NOT touch other agents (cert-rust-reimpl, cert-ptrace, cert-re).
- DO NOT commit anything anywhere.

## Reporting (turn 2 final)
Provide: hex of precursor (each length tried), hex of computed `final_upper64` (each length), hex of anchor, match-byte counts, verdict, and (if not PASS) the next-refinement proposal.
