# algorithm-fit — exhaustive crypto-primitive fit against already-captured data

**You ARE allowed and expected to write code.** Python (cryptography, hashlib, pyaes, chacha20, etc.).

## Role & workdir

Algorithm-fit the cert: try every plausible cryptographic primitive against H-N4's 5 (challenge, cert) pairs, using the discovered key material (32-char hex secret at device_info+0x210) in BOTH ASCII (32-byte) and decoded (16-byte) forms. Critical: H-N7 only tried ASCII-form-as-HMAC-key — the **decoded 16-byte form has NOT been tested** in standard primitives. Workdir: `/home/sdancer/nmss-emu-algorithm-fit/`.

## Why this path

K=6 planner cycle 68 identified this as a high-EV cheap-parallel play vs H-N8's expensive HW-BP capture. We have:
- 5 verified `(challenge, cert)` ground-truth pairs (H-N4's encoder_io_2026-05-11.jsonl).
- 32-char hex secret `"2FCF997702C244969BFEAF7F0D6AAA1C"` at device_info+0x210 (likely HMAC/cipher key).
- Static blobs: package = `"com.netmarble.thered"`, APK path (per-install).
- Empirical signatures (cycle 67 H-N7): avalanche stats confirm real cryptographic mixer; 5000-insn NEON-free function size consistent with **SHA-256 unrolled** OR **AES with T-tables**.

H-N7 falsified: HMAC-MD5/SHA1/SHA256 keyed by **ASCII-form** secret vs challenge-concat permutations. **NOT YET TESTED**: decoded 16-byte secret, AES variants, HKDF, SipHash, CMAC, Speck.

## Goal / sub-goal

- **Goal:** `nmss_cert_primitive_identified` (currently 0.0 → 1.0 if a primitive matches all 5 vectors).
- **Sub-goal:** Identify which cryptographic primitive + input shape + key form reproduces all 5 (challenge, cert) ground truths.

## Success criteria

- **Minimum**: Document the primitive sweep — primitives tested, input shapes tested, key forms tested. Save `analysis/sweep_log.jsonl` with one row per (primitive, input-shape, key-form, hit-count) tuple.
- **Stretch**: ≥3/5 vectors match → likely the primitive (residual misses from input-shape detail). Set fact `cert_primitive_candidate_2026_05_11 = <name>`.
- **Full**: 5/5 match → primitive identified. Set fact `cert_primitive_identified_2026_05_11 = <name with full param list>`. **CAMPAIGN COLLAPSES TO TRIVIAL RUST PORT.**

## Inputs you have

- **5 ground-truth pairs**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl` (each row: `{challenge, expected_cert, replayed_cert, ...}`).
- **Cross-check**: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/ground_truth_v2_2026-05-11.json`.
- **Key material**: 32-char hex secret `"2FCF997702C244969BFEAF7F0D6AAA1C"`. Decoded 16 bytes: `2f cf 99 77 02 c2 44 96 9b fe af 7f 0d 6a aa 1c`.
- **Package name**: `"com.netmarble.thered"`.
- **APK path**: see H-N7's deliverables; varies per install but constant across H-N4's 5 captures.
- **Challenge format**: from H-N4 — 17-char std::string with leading space (e.g. `" 0000000000000000"`).
- **Cert format**: 48-character ASCII hex (24 bytes binary).

## Sweep menu (ORDERED by likelihood)

1. **AES-128 keyed by decoded 16-byte secret**:
   - AES-128-ECB(secret, challenge-as-16-bytes) || AES-128-ECB(secret, challenge-as-second-16-bytes) → 32 bytes, truncate to 24.
   - AES-128-CTR(key=secret, nonce=??, plaintext=challenge-or-package-or-apk) → 24 bytes.
   - AES-128-CBC(key=secret, IV=??, plaintext=??).
   - AES-128-CMAC(key=secret, msg=challenge or package||challenge or apk||challenge).
2. **HKDF-SHA256(salt=secret, ikm=package||challenge, info=apk_path) → first 24 bytes**. Try all permutations of salt/ikm/info.
3. **HMAC variants keyed by DECODED 16-byte secret** (NOT ASCII — that's what H-N7 already falsified): HMAC-MD5, HMAC-SHA1, HMAC-SHA256, HMAC-SHA512, all truncated to 24 bytes. Try msg = challenge, package||challenge, apk||challenge, challenge||package, etc.
4. **SipHash**: SipHash-2-4(key=first 16 bytes of secret, msg=challenge variants); SipHash-1-3 variant; HighwayHash.
5. **Speck128**: keyed encryption with decoded secret as key.
6. **Custom**: SHA-256(secret || challenge) || SHA-256(challenge || secret) etc → trunc 24.
7. **Two-stage**: SHA1(secret || challenge) → 20-byte → AES-encrypt with same → 16-byte → format.

For each: try LE and BE encoding of x6/x7 challenge representations. Try ASCII-with-and-without leading-space.

## Next 3 ordered tasks

1. **Write the sweep harness**. Single Python script `scripts/01_sweep.py` that loops over all primitives × input-shapes × key-forms. Uses `cryptography`, `hashlib`, `pyaes` (or `Crypto.Cipher.AES`). For each tuple, compute the candidate cert for all 5 challenges, count matches. Save to `analysis/sweep_log.jsonl`.

2. **Run the sweep**. Expected runtime: 30-60 minutes for ~10k combos.

3. **Validate hits**. Any primitive with ≥3/5 matches: report. ≥4/5: high confidence. 5/5: campaign collapse. Save full match details to `analysis/primitive_match.json`.

## Constraints & gotchas

- **No git commits.**
- **Use the decoded 16-byte form** as the primary key — H-N7 already falsified ASCII form.
- **Encoder input is x6:x7 little-endian-packed ASCII** (per H-N4) — not the raw challenge bytes. So `challenge = "01234567"` → x6 = `'0' | '1'<<8 | '2'<<16 | ...` = 0x3736353433323130. Test both raw-ASCII and LE-packed forms as input.
- **The leading-space std::string** at x21+0x50 was `" 0000000000000000"` (17 chars). The actual algorithm may or may not include the leading space. Try both.
- **Cert is 48 ASCII hex chars** (= 24 raw bytes). Algorithm output → hex-encode → cert.

## Relevant files / references

- H-N4 encoder I/O: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- H-N3 ground_truth_v2: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/ground_truth_v2_2026-05-11.json`
- H-N7 falsified hypotheses: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_producer_port_blockers.md` (note exactly which forms H-N7 tested)
- Decoded secret: `2fcf997702c244969bfeaf7f0d6aaa1c` (16 bytes)

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/algorithm_fit_progress_2026-05-11.jsonl`.

## Operating mode

In-process Agent (background). 2h budget. STOP on:
- (a) 5/5 match → **CAMPAIGN COLLAPSES TO RUST PORT**. Set fact, escalate.
- (b) ≥3/5 match → likely primitive identified with residual input-shape detail. Set fact, propose H-N9 to refine.
- (c) All primitives miss → algorithmic-fit hypothesis falsified; H-N8 must do the work.
