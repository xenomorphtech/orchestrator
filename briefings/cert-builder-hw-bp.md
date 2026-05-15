# cert-builder-hw-bp (H-N8) — HW-BP inside the real cert builder at 0x78c689575c

**You ARE allowed and expected to write code.** Python (HW-BP driver), shell, eventually Rust.

## Role & workdir

Install 6 HW-BPs inside the real cert builder at runtime PC `0x78c689575c` to localize **the actual cryptographic core** (currently believed to be at `0x78c68e2b68`, a ≥5000-insn function). The objective is to bisect with HW-BPs until the cert is observable in a tight enough region to either symbolic-lift OR algorithm-fit. Workdir: `/home/sdancer/nmss-emu-cert-builder-hw-bp/`.

## Why this path

H-N7 (cycle 66) hard-gated on porting `0x78c689528c`, but revealed:
- That function is a switch DISPATCHER, not the cert builder. The w0==2 path (all 5 ground truths) delegates to `0x78c689575c`.
- The real cert builder is **`0x78c689575c`** — 553+ visible insns, continues into next dump page, calls `bl 0x78c68e2b68` (≥5000-insn gated core) and `bl 0x78c694f124`.
- Avalanche stats confirm a real cryptographic mixer.
- **A 32-char hex secret `"2FCF997702C244969BFEAF7F0D6AAA1C"` is at `device_info+0x210`** — likely the HMAC/cipher key.
- HMAC-MD5/SHA1/SHA256 with secret-hex-or-bytes-as-key vs challenge-permutations all fail. So the cert is NOT a standard MAC over (challenge, secret).

The cert is something more elaborate, but with a known key and known cryptographic-mixer signature. HW-BP at strategic boundaries inside the builder will localize the exact algorithm primitive.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (eventual 5/5 closes campaign).
- **Sub-goal:** Identify the cryptographic primitive at `0x78c68e2b68` (or wherever the actual hash/cipher runs). Capture its inputs (key material, message) and outputs. With known I/O, port becomes algorithm-fit (try AES/SHA/Speck/Chacha20/etc. against captured inputs).

## Success criteria

- **Minimum**: 6 HW-BPs at strategic points inside `0x78c689575c` fire for all 5 challenges. Capture regs + stack windows + std::string derefs at each. Save `analysis/builder_internal_bp_captures.jsonl`.
- **Stretch**: Cert observable at a BP earlier than the final write — narrows the producer further. Or: at the entry to `0x78c68e2b68`, capture its inputs (key, message, IV) and identify the primitive via algorithm-fit against captured (input, output) pairs.
- **Full**: Cryptographic primitive identified (e.g. "AES-CTR with `2FCF99...AAA1C` as key, IV from device_info+0x...") → port becomes trivial. Set fact `cert_primitive_identified_2026_05_11`.

## Inputs you have

- **H-N7 disasm of cert builder**: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_builder_0x78c689575c_disasm.txt` (553 visible insns).
- **H-N7 disasm of gated core**: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_core_0x78c68e2b68_disasm_first20k.txt` (first 20k chars).
- **H-N7 function bounds**: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_producer_function_bounds.json`.
- **H-N7 blockers doc**: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_producer_port_blockers.md` (with the 6-HW-BP plan).
- **H-N6 HW-BP captures**: `/home/sdancer/nmss-emu-encoder-internal-bp/analysis/internal_bp_captures.jsonl` — has regs+memory at the producer-call sites; verify the device_info offsets H-N7 found (+0xa0 APK, +0x1f8 package, +0x210 secret).
- **Patched native-replay-rs**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` (supports `--trace-call-hw <hex>`).
- **5 ground-truth pairs**: from H-N4's encoder_io_2026-05-11.jsonl + H-N3's ground_truth_v2.

## Next 3 ordered tasks

1. **Plan the 6 BPs**. From H-N7's blockers doc, the suggested 6 BPs are: (a) entry of 0x78c689575c, (b) pre-bl 0x78c68e2b68, (c) post-bl 0x78c68e2b68, (d) pre-bl 0x78c694f124, (e) post-bl 0x78c694f124, (f) pre-final-formatter. Verify these PCs from cert_builder_0x78c689575c_disasm.txt. Save `analysis/bp_plan.json`. Important: also dump the device_info heap allocation at +0x28 (the 6.3 KiB blob H-N7 didn't capture) — it's an INPUT to the builder.

2. **Run captures**. 4 HW-BP slots/kernel — split into 2 passes (BPs 1-4 then 3-6). 5 challenges × 2 passes = 10 runs. Capture: regs (x0-x30+sp+pc), 256 bytes at sp+0x180..0x280, regs that look like pointers (deref 128 bytes each), std::string at x21+0x50 (challenge), std::string at x21+0x68 (cert if present), device_info[+0xa0/+0x1f8/+0x210/+0x28]. Save `analysis/builder_internal_bp_captures.jsonl`.

3. **Cryptographic primitive identification**. At BPs (b) and (c) (pre/post `bl 0x78c68e2b68`), check whether the output is the cert OR an intermediate. If output is cert: 0x78c68e2b68 IS the primitive — focus there. Algorithm-fit: try (input=device_info+0x28 or challenge or both, key=`2FCF99...AAA1C` or device_info+0x210 bytes-hex-decoded, output=captured 24-byte value) against AES-128-CTR, AES-128-CBC, ChaCha20, Speck128, SipHash, Curve25519/HKDF, BLAKE2, KMAC, etc. Use `cryptography` Python library + brute-force IV/nonce/mode permutations. Report which primitive matches if any.

## Constraints & gotchas

- **No git commits.**
- **HW-BP slots limit (4 per kernel)**: 2-pass capture per challenge.
- **0x78c68e2b68 sub-callee is ≥5000 insns**: don't try to disasm it all; trust the HW-BP captures to characterize its I/O.
- **Device-info heap at +0x28 is 6.3 KiB**: that's a LOT of input bytes. The crypto primitive may use the WHOLE 6.3 KiB as state, NOT just the 32-char secret at +0x210.
- **Falsified by H-N7**: simple HMAC-MD5/SHA1/SHA256 of `(challenge, secret)` doesn't work. So either the input shape is different (includes APK+package+device_info[+0x28]) OR the algorithm is non-HMAC (block cipher in some mode).
- **NEON**: H-N7's "no NEON inside producer" was about the dispatcher 0x78c689528c. The builder at 0x78c689575c MAY have NEON; check the disasm.

## Relevant files / references

- H-N7 deliverables: `/home/sdancer/nmss-emu-cert-producer-port/analysis/` (especially cert_producer_port_blockers.md with the BP plan)
- H-N6 BP captures: `/home/sdancer/nmss-emu-encoder-internal-bp/analysis/internal_bp_captures.jsonl`
- H-N4 encoder I/O: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- H-N3 ground_truth_v2: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/ground_truth_v2_2026-05-11.json`
- Patched binary: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_builder_hw_bp_progress_2026-05-11.jsonl`. Stages: `bp_plan_verified`, `device_info_28_captured`, `5x_pass1_done`, `5x_pass2_done`, `primitive_identified_or_not`.

## Operating mode

In-process Agent (background). 4h budget. STOP on:
- (a) Cryptographic primitive identified (e.g. "AES-128-CTR(key=secret, msg=device_info+0x28+challenge)") → set fact, escalate to H-N9 (port the primitive — trivially achievable with crypto library).
- (b) Primitive not identified but input/output narrowed to a tight region (e.g. 24-byte input → 24-byte output via 0x78c68e2b68) → write blockers with the captured I/O, propose more-targeted algorithm-fit.
- (c) BPs don't fire OR cert doesn't appear at any BP → hard-gate, propose H-N9-alt (capture all 6.3 KiB of device_info heap to reduce input-space ambiguity).
