# md5-sha-fit — fit cert = MD5(...)[:16] || SHA-256(...)[8 bytes] (or similar hybrid)

**You ARE allowed and expected to write code.** Python (hashlib, struct), Rust if needed.

## Role & workdir

Algorithm-fit the cert as a **HYBRID construction** combining MD5 and SHA-256. Worker `a853c41fedb0d74b8` (H-N8, killed cycle 78 for stall but had this final delivery) found that at BP `0x78c68a07b0` in the cert builder, registers hold the standard **MD5 compression state**: `MD5_compress(STD_IV, chal+secret+chal_64bytes)`. Cert is 24 bytes; MD5 produces 16 bytes; so the cert is `MD5(...) || 8_extra_bytes`. The 8 extra bytes are likely from **SHA-256** (per gadget-scan's confirmed 70 SHA-256 K-table xrefs). Workdir: `/home/sdancer/nmss-emu-md5-sha-fit/`.

## Why this path

H-N8 (cycle 67-78, killed) discovered the cert construction is MD5+SHA-256 hybrid:
- **BP 0x78c68a07b0** captures MD5 compression state mid-flight (regs x15/x16/x5 hold MD5 state[3]/[1]/[2]).
- The input was `chal+secret+chal` as ASCII, 64+ bytes (needs MD5 padding into block 2).
- Cert is 24 bytes = MD5(16) + 8 extra.

H-N7's earlier HMAC-MD5/SHA1/SHA256 sweep failed because:
- Input shape wasn't `chal+secret+chal` (concatenation order matters)
- Cert isn't pure HMAC-MD5 output (it's truncated/extended)
- The 8 extra bytes come from a separate primitive (SHA-256 most likely)

Algorithm-fit's 254k-combo sweep falsified standard primitives but didn't test this specific hybrid shape: **MD5(chal||secret||chal)[:16] || SHA-256(...)[8 bytes]**.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5 if hybrid identified).
- **Sub-goal:** Identify the exact input string + variant for both MD5 and SHA-256 components.

## Success criteria

- **Minimum**: Test the H-N8-suggested input shape `MD5(chal_ASCII + secret_ASCII + chal_ASCII)` against the 5 ground-truth certs' first 16 bytes. Report pass-count.
- **Stretch**: If ≥3/5 MD5 part matches → identify the 8-byte tail source (SHA-256 of what?). Sweep input combos for the SHA-256 tail. Save `analysis/hybrid_fit.json` with the full algorithm spec when 5/5 matches.
- **Campaign close**: 5/5 → set fact `cert_primitive_md5_sha_hybrid_2026_05_11` with the full input formula.

## Inputs you have

- **H-N8 final insight (this briefing)**: BP 0x78c68a07b0 captures `MD5_compress(STD_IV, chal+secret+chal_64bytes)` registers. The killed-worker's reply hinted "Let me re-check what MD5(message=chal_ASCII+secret_ASCII+chal_ASCII) produces with PROPER padding (it's 64 bytes, so the standard MD5 has padding into block 2)".
- **5 ground-truth (challenge, cert) pairs**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- **Secret hex** (at device_info+0x210): `2FCF997702C244969BFEAF7F0D6AAA1C` (32 ASCII chars, 16 bytes decoded). Both forms candidate-relevant.
- **Package**: `com.netmarble.thered`
- **APK path** (per H-N4): `/data/app/~~cGPM14AP6lPy6m9_u22NmA==/com.netmarble.thered-6XSqEE8z5mpf3XJtmS-BlQ==/base.apk`
- **Device model**: `rk3588_s`
- **Device_info+0x1d0 32B blob**: `01ef33bf458c7645fdc44ab75a72f1ed4fa1b6fbf99aa89b22213f88bcd1e679`
- **Challenges (from encoder_io_2026-05-11.jsonl)**: `0000000000000000`, `0123456789ABCDEF`, `1111111111111111`, `7BDA93D2F45D36C0`, `AABBCCDDEEFF0011`
- **80MB of H-N8 captures**: `/home/sdancer/nmss-emu-cert-builder-hw-bp/analysis/chal_*.json` — has reg snapshots at various BPs. Could verify MD5 state interpretation by checking x15/x16/x5/?? values against expected post-MD5-compress state.

## Next 3 ordered tasks

1. **Test the immediate hypothesis**: `cert[:32]_hex` (first 16 bytes hex-encoded = first 32 cert chars) ?= `MD5(chal_ASCII + secret_ASCII + chal_ASCII)` for all 5 challenges. Try: secret as ASCII (32 chars), secret as decoded (16 bytes), challenge as raw ASCII (16 chars), challenge with prepended space (17 chars). Save `analysis/md5_part_fit.json` with pass-count per variant.

2. **If MD5 part matches ≥3/5**: identify the SHA-256 tail. The 8 extra cert bytes (cert[16..24]) are likely SHA-256 truncated. Sweep: SHA-256(same-input)[0..8], SHA-256(same-input)[4..12], SHA-256(secret||MD5_result)[0..8], etc. Save `analysis/sha256_tail_fit.json`.

3. **If full 5/5 hybrid match**: write Rust port `cert_hybrid/src/lib.rs` with the verified algorithm. Run against 5 ground truths. **CAMPAIGN COMPLETE**. Set fact `cert_primitive_md5_sha_hybrid_2026_05_11`.

## Constraints & gotchas

- **No git commits.**
- **MD5 vs SHA-256 conflict with gadget-scan**: gadget-scan said "NO MD5 T-table found", but H-N8 said MD5 state at BP. Possible explanations: MD5 constants are loaded inline as movz/movk (not from rodata); OR H-N8 misidentified regs (could be SHA-1 state, which has similar shape). Test BOTH MD5 and SHA-1 for the 16-byte head.
- **Standard MD5 IV**: `0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476` (state[0..3]).
- **MD5 padding for 64-byte input**: 0x80 + zeros + 8-byte length = needs a second block. Standard MD5 handles this; just use `hashlib.md5(input).digest()`.
- **Challenge ASCII can be lower or upper** — test both.
- **Concatenation order** matters: try chal+secret+chal, chal+secret, secret+chal, secret+chal+secret, chal+secret+package, etc.

## Relevant files / references

- H-N8 captures (~82MB): `/home/sdancer/nmss-emu-cert-builder-hw-bp/analysis/chal_*.json`
- H-N4 ground truth: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- Gadget-scan SHA-256 evidence: `/home/sdancer/nmss-emu-gadget-scan/analysis/gadget_hits.json` + `crypto_table_xrefs.json`
- H-N7 disasm: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_builder_0x78c689575c_disasm.txt` (553+ insns of real builder, may show MD5 init sequence)
- The killed-H-N8 partial output is at `/tmp/claude-1001/-home-sdancer-orchestrator/a28f7421-b023-4c5e-8069-a5c3d1870a5e/tasks/a853c41fedb0d74b8.output` (full session JSONL; DO NOT cat the whole file but you can grep it for "BP 0x" or "MD5" if useful).

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/md5_sha_fit_progress_2026-05-11.jsonl`.

## Operating mode

In-process Agent (background). 2h budget. STOP on:
- (a) 5/5 hybrid match → **CAMPAIGN COMPLETE**. Set fact, escalate to user.
- (b) ≥3/5 MD5-part match but tail not identified → set partial fact, propose H-N10 for SHA-256 tail-input localization.
- (c) MD5-part 0/5 across all variants → MD5 hypothesis from killed-H-N8 was wrong; primitive is SHA-256-pure or something else; report and propose alt path.
