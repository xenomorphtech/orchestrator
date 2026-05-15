# cert-cluster1-32B-derivation (H-N15) — recover the challenge → cluster1_inner_32 transform

**You ARE allowed and expected to write code.** Rust (cert-rust-repro plug-in), Python (capture diff analysis).

## Role & workdir

The cert algorithm is fully structurally recovered: `cert = hex_upper(SHA256_compress_from_IV(ASCII("F5F1E084") || SHA256(ASCII("C6D521A7") || cluster1_inner_32) || 0x80 || padding)[4..28])`. Pure-Rust implementation in `cert-rust-repro` passes 5/5 IF given the per-challenge 32-byte `cluster1_inner_32`. Your mission: **find how `cluster1_inner_32` is derived from the 16-char challenge ASCII**. Workdir: `/home/sdancer/nmss-emu-cert-cluster1-32B-derivation/`.

This is the FINAL path. One unknown left; everything else is done.

## Why this path

H-N14 stop case (b) findings (see `/home/sdancer/nmss-emu-cert-frag1-sp968-port/analysis/cert_frag1_sp968_port_findings.md`):

- 24-byte cert oracle FULLY REMOVED from `stage_05`. `phase_d_e2e_5vec_checkpoint` passes 5/5 via pure-Rust SHA-256 chain.
- Two ASCII 8-char constants `"C6D521A7"` and `"F5F1E084"` are binary-embedded magic stamps (challenge-invariant).
- Only oracle left: `cluster1_inner_32_fixture_for_challenge(challenge_hex) -> [u8; 32]` — a 5-row lookup table.
- ~30 simple hash hypotheses tested negative (SHA-256/MD5/SHA-512/CRC32/xxh32 over various challenge encodings + ASCII prefixes + session token combinations).
- Producer of `cluster1_inner_32` lies in disasm window `0x78c68c8a54..0x78c68db3fc`.
- Challenge-invariant 32-bit constants observed in captures: **`0x92C646FB`, `0xCCA6216B`, `0xB3D3C8D9`, `0xDA4E7F25`**. These look custom — not stock SHA/xxh/MD5 initializers.
- session_token_hex32 = `"2FCF997702C244969BFEAF7F0D6AAA1C"` (challenge-invariant, available at runtime; matches device_info+0x210 secret).

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5 — last remaining piece).
- **Sub-goal:** Identify `fn cluster1_inner_32(challenge: &[u8; 16]) -> [u8; 32]`.

## Success criteria

- **Minimum**: Capture sufficient BP data in the `0x78c68c8a54..0x78c68db3fc` window across 5 challenges to localize where the 32 bytes are written. Save `analysis/cluster1_inner_32_writers_5x.jsonl`.
- **Stretch**: Identify the producer algorithm. Replace `cluster1_inner_32_fixture_for_challenge` in `cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` with a pure-algorithm Rust fn. Run `cargo test phase_d_e2e_5vec_checkpoint` and confirm pass.
- **Campaign close**: 5/5 → set facts `cert_cluster1_inner_32_derivation_recovered_2026_05_12` + `nmss_cert_5_5_pure_rust_reproduced` + escalate to user as CAMPAIGN COMPLETE.

## Concrete tasks (ordered)

1. **Run upstream BPs.** Set HW-BPs in the `0x78c68c8a54..0x78c68db3fc` window via patched `native-replay-rs --trace-call-hw` on remote. Targets: every PC at 0x100 stride from `0x78c68c8a54` to `0x78c68db3fc` (≈232 PCs). Capture register state + q-registers + memory dumps at any address near the upstream cluster1_inner_32 buffer. The 32 bytes get progressively built — find the PC where the FIRST byte of cluster1_inner_32 lands.

2. **Cross-challenge diff at each BP.** For each PC, identify whether the captured state varies challenge-to-challenge. The challenge ASCII (16 chars) must flow through somewhere. Track its first appearance, then track subsequent transformations. Save `analysis/upstream_diff_per_bp.md` with a per-PC variance table.

3. **Hypothesize + verify.** The 4 challenge-invariant constants `0x92C646FB, 0xCCA6216B, 0xB3D3C8D9, 0xDA4E7F25` (16 bytes total = 128 bits) might be:
   - A custom hash IV (4×u32 = 128-bit state like Tiger or BLAKE2s_64-bit)
   - An AES key (128-bit = 16 bytes) — try AES-128-ECB(challenge_padded_to_16B, key=these_constants) → 32B = AES_block || padded result
   - A custom polynomial / S-box
   - A SipHash key
   
   Test these specifically (the 30 prior failed hypotheses didn't include using these 4 constants as key/IV material). Save `analysis/algorithm_hypothesis_tests.md` with results.

4. **Port to Rust + validate.** If algorithm identified: implement `fn cluster1_inner_32(challenge: &[u8; 16]) -> [u8; 32]` in `cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs`. Remove the fixture. Run `cargo test phase_d_e2e_5vec_checkpoint`. **5/5 → CAMPAIGN COMPLETE.**

## Constraints & gotchas

- **No git commits.**
- **The 4 constants** (`0x92C646FB 0xCCA6216B 0xB3D3C8D9 0xDA4E7F25`) are the strongest signal. Try them as: AES-128 key, ChaCha20 key (4×u32 of 8×u32), SipHash key (k0=2u64 from concat), custom hash IV (state[0..4]).
- **Challenge-invariant constants in the cipher** mean the algorithm has a fixed-key/seed flavor. With 5 (challenge, output) pairs and a known structure (AES/custom), the inverse is often deterministic.
- **Use the existing 5/5 cluster1_inner_32 fixture as ground truth**: the values are already in `cluster1_inner_32_fixture_for_challenge` in stage_two_step_sha256_cert.rs. Any hypothesis can be tested by computing your candidate `f(challenge)` and comparing with the fixture row.
- **Run all replays on remote `root@162.244.80.97`**. Use sshpass + env $ARM64_PASSWORD.

## Relevant files / references

- **H-N14 findings (read first)**: `/home/sdancer/nmss-emu-cert-frag1-sp968-port/analysis/cert_frag1_sp968_port_findings.md`
- **H-N14 captures**: `/home/sdancer/nmss-emu-cert-frag1-sp968-port/analysis/sp968_writer_chain_5vec_2026-05-12.json`
- **Pure-Rust cert algorithm (modify)**: `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs`
- **Independent verification test**: `/home/sdancer/nmss-emu/cert-rust-repro/tests/captured_sp968_5vec_verify.rs`
- **5 ground-truth (challenge, cert) pairs**: as in prior briefings — chal `0000000000000000` → cert `4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47`, etc.
- **Patched native-replay-rs (on remote)**: `/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_cluster1_32B_progress_2026-05-12.jsonl`. Stages: `bp_window_swept`, `upstream_diff_done`, `algorithm_hypothesis_verified`, `rust_port_drafted`, `5_of_5_CAMPAIGN_COMPLETE`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) 5/5 pure-Rust match (no fixture) → **CAMPAIGN COMPLETE**. Set facts: `cert_cluster1_inner_32_derivation_recovered_2026_05_12` + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.
- (b) Algorithm structurally identified (e.g. AES-128 with these constants as key) but Rust port not yet 5/5 → set partial fact.
- (c) BP captures done, all reasonable algorithm hypotheses tested → write `analysis/cluster1_32B_blocker.md` with the table of (PC, register state, captured byte) per challenge + every algorithm tried. The next worker can attack from a different angle (full disasm).
- (d) Cannot capture sufficient state (HW-BPs miss the actual byte-write site, e.g. it's via NEON store-pair) → write blocker, recommend memory-watchpoint capture.
