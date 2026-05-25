# cert-rust-scan — Path B fast brute-force scan of 489MB dump

## Role & workdir
Performance-scan worker. Workdir: `/home/sdancer/nmss-emu-device-webview-signin`. Existing v3 dump at `analysis/artifacts/mem_snaps_v3/` (755 files, 489MB raw, 26MB tgz already pulled).

## Why this turn exists
Cycle 1037 Track A v3 validated substrate (4× op902=0) and proved the dump contains all relevant strings (cert ASCII, sessionID, PID). But the Python sliding-window scan only tested donor format `0x80+N×0x00+BE_bit_length` (N=24/32/40/48) — 0 hits. Cert function uses a DIFFERENT SHA-input encoding than donor.

Worker proposed Path B as next: a fast Rust scanner that searches the 489MB dump under MULTIPLE alternative padding/encoding hypotheses. Cycle 1024 planner estimated Rust ≈10s where Python is ≈7 hours per pass — gives us 100×+ headroom to test many encodings cheaply.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay`
- **sub_goal_key**: `cert-rust-scan` (Path B)

## Hypothesis
The 64-byte SHA-256 input block IS in the 489MB dump but under a different encoding than donor's `0x80+zeros+BE_0x0140`. A fast Rust scanner that tries (i) raw 64B windows (no pad assumption), (ii) different message lengths, (iii) different endian/byte-swap variants, (iv) different IVs, against all 4 known (sessionID, cert) pairs will find at least one block whose compress matches a cert.

## Falsification criterion
Rust scanner runs across all 489MB × all 4 certs × ≥6 distinct encoding hypotheses and produces 0 matches → strong evidence the cert pipeline does NOT use `single_sha256_compression_block(...)[4..28]` shape at all, and the donor README's algorithm description is wrong. At that point H-CL3 (LKM kprobe) or static RE (Path A) becomes mandatory.

## Hard rules
- **No userspace injection** (no Frida, no ptrace) — we already have the dump on disk.
- **Pure analysis**: read `mem_snaps_v3/` + scan, NOTHING on-device.
- **30 min wall cap. 1 GB RAM cap.** Don't load all 489MB into RAM at once; mmap per file.
- Worker host = THIS machine (sdancer's orchestrator host, x86_64). Rust scanner runs HERE, not on device.

## Plan

### Step 1 — write the Rust scanner (`/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/cert_scan/`)
Create a Cargo project that:
1. Loads all 4 ground-truth (sessionID, cert) pairs as targets (see wiki `06-ground-truth-pairs.md` or fact `cert_mem_fourth_ground_truth_pair_2026_05_17`):
   - `D0D2691014BE1858` → `6045D74EB176E0C4A4DB23F0CEBEBEB7EE7644730D213147`
   - `CE37A6C13EE831D6` → `9D6C5B875902C1062670232A76DFE5AC1191E64A97767C55`
   - `8242F664BFE2DD2D` → `53FB1F7A8BAE0A2F543AC4329EF4B68FC21BE8B2540C0AA0`
   - `B043FD34B7B482AA` → `A918EF762C442C57330E4B6ABF551914AB8F68F3361B0687`
2. Memory-maps each `.bin` file in `analysis/artifacts/mem_snaps_v3/` (only the snapshot belonging to that run's pair — match by directory naming).
3. For each 64-byte window at every byte offset (~7.5M windows per file):
   - Run ONE SHA-256 compression block from canonical IV (use `sha2` crate's `compress256` if exposed, or hand-roll the round function — single block is ~64 ARM-style ops).
   - Take digest bytes [4..28] → 24-byte cert candidate.
   - Compare against each pair's cert (hex-decoded).
   - On match: print `MATCH file=<f> off=<o> pair=<n>` and save the input block.
4. After byteswap-per-word variant (since donor's "byteswap32_per_word" is documented).
5. Optional encoding variants to also try if time permits:
   - SHA-256 compress with NIST IV vs zero-IV vs custom-IV
   - Take bytes [0..24] vs [4..28] vs [8..32]
   - Hash the 64-byte window normally (not single-block compress) and compare.

### Step 2 — build + run
```bash
cd /home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/cert_scan
cargo build --release
target/release/cert_scan ../mem_snaps_v3/ > ../cert_rust_scan_results.txt
```

Time budget: 60-120s for the full scan. If significantly slower, profile and tighten the inner loop (avoid per-window allocations).

### Step 3 — analyze hits
If MATCH found:
- Save the 64-byte block hex to `analysis/artifacts/cert_block_captured.bin` + JSON descriptor.
- Verify by running cert_rust_repro with a hand-crafted session.json containing only `sp_0x968_block_hex64 = <captured>` → expect output == matching cert.
- Stage 4 of goal closes → set fact `cert_block_captured_via_rust_scan_2026_05_17` and `nmss_clientless_socks5_replay_proven` candidate trigger.

If NO MATCH across all variants:
- Save the full enumeration in `cert_rust_scan_results.txt`.
- Set fact `cert_rust_scan_no_match_in_v3_dump_2026_05_17` = true with verdict.
- Verdict reasoning: cert function uses an encoding outside the tested family → escalate to LKM kprobe (Path D) or static RE (Path A).

## Outputs
- `analysis/artifacts/cert_scan/` (Cargo project, ~200 lines of Rust)
- `analysis/artifacts/cert_rust_scan_results.txt` (per-pair hit list / 0-hit verdict)
- On SUCCESS: `analysis/artifacts/cert_block_captured.bin` + descriptor JSON
- Facts (one of):
  - `cert_block_captured_via_rust_scan_2026_05_17` = true (with file+offset+pair)
  - `cert_rust_scan_no_match_in_v3_dump_2026_05_17` = true (with enumeration of tested encodings)
- Final line: `CERT_RUST_SCAN_DONE`

## References
- `analysis/artifacts/mem_snaps_v3/` — the dump (489MB raw, 755 files)
- `analysis/artifacts/cert_mem_v3_op901.json` — this-run pair + meta
- `/home/sdancer/nmss-emu/cert-rust-repro/README.md` — verified algorithm shape (single compress, [4..28] byteswap)
- `/home/sdancer/nmss-emu/cert-rust-repro/src/main.rs` — reference impl
- `/home/sdancer/nmss-wiki/05-cert-algorithm.md` — distilled algorithm doc
- `/home/sdancer/nmss-wiki/06-ground-truth-pairs.md` — all 4 pairs
- Fact `cert_mem_v3_no_donor_format_block_in_dump_2026_05_17` (what was already tested + falsified)
