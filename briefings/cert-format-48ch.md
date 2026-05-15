# cert-format-48ch — port the 32-byte → 48-char ASCII cert formatter

**You ARE a worker sub-agent. You are allowed and expected to write code.** The orchestrator-role memory ("never write code directly") applies to the orchestrator instance only, NOT to you.

## Role & workdir
Disassemble the formatter at `libnmsssa.so:0x123288` from the snapshot, identify the transformation from a 32-byte intermediate to the 48-char ASCII cert, port it to Rust, and verify against at least one ground-truth pair when plugged into the existing `cert-rust-repro` crate's `raw32_after_13`. Workdir: `/home/sdancer/nmss-emu-cert-format-48ch/`.

## Why this is a fast win
- Per fact `cert_jni_return_is_48char_ascii_2026_05_03`, `libnmsssa.so:0x123288` is the post-processor that produces the 48-char ASCII JNI return.
- Per fact `cert_remaining_gap_2026_05_11`, the existing `cert-rust-repro` crate already produces a 32-byte intermediate `raw32_after_13`; the remaining gap is just this formatter + side_x0.
- Per fact `cert_callback_chain_resolved_2026_05_03`: "0x123288 in libnmsssa.so is NOT the cert producer — it just copies a std::string returned by a CALLBACK stored in global slot +2896". So 0x123288 *may* be a thin wrapper rather than the format-conversion itself — verify which by reading disasm. If thin: the actual formatter is further upstream (somewhere in the deleted-module callback at 0x6cc2430ed0).
- Sibling path `callback-body-port` is working on the full callback. Your work is complementary: if 0x123288 IS the formatter, you close a chunk independently; if it's just a copier, your disasm + writeup still tells `callback-body-port` exactly where the format conversion lives, saving them work.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` (metric 0.85 → up to ~1.05 if you close this stage).
- **Metric contribution:** +0.2 on transformation_recovered if the formatter is faithfully ported. +N on `pure_rust` if plugged into cert-rust-repro and any vector matches.

## Success criteria
- Rust function `fn format_cert(raw32: &[u8; 32]) -> String` (or signature matching what disasm reveals) that mirrors the transformation at `libnmsssa.so:0x123288`. Place in a new crate `cert_format/` in your worktree.
- Cargo test: take the cert-rust-repro `raw32_after_13` for at least one ground-truth challenge, run your formatter, compare to the 48-hex ground-truth at `analysis/test_vectors_2026-04-24/summary.json`.
- If the disasm reveals 0x123288 is a thin copier (not the formatter), document that finding precisely (with PC ranges) and STOP — the gap is upstream of you.

## Inputs

- **Snapshot libnmsssa shard**: find it under `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/memdump/`. The libnmsssa module base in the snapshot is **`0x78c6830c70`** per fact `cert_F7B3B00F8A5_module_base_correct` (NOT 0x78cc288000 — that's the F7B3B00F8A5 deleted module). Function at module-offset `0x123288` is at VA `0x78c6830c70 + 0x123288 = 0x78c6953ef8`. Find the shard that covers `0x78c6953ef8` (likely a `78c6___000.bin` shard around 0x78c6900000–0x78c6a00000).
- **Existing reproducer** (read-only reference): `/home/sdancer/nmss-emu/cert-rust-repro/`. Look at the `raw32_after_13`-producing code to understand the intermediate's exact shape and provenance.
- **Ground truth**: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json` (5 pairs, snapshot-state, 48-hex uppercase).
- **Maps**: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/maps.txt`.

## Next 2–3 concrete tasks (ORDERED)

1. **Find + disasm the function at `libnmsssa.so:0x123288`.** Compute the snapshot VA (`0x78c6830c70 + 0x123288 = 0x78c6953ef8`), find the covering shard in the memdump dir, run `objdump -D -b binary -m aarch64 --adjust-vma=<shard_base> <shard>.bin > analysis/libnmsssa_0x123288.disasm`. Identify the function bounds (entry → first `ret`). Save bytes + disasm to `analysis/libnmsssa_0x123288_<size>.bin` and `_disasm.txt`.

2. **Decide: formatter or thin-copier?** Read the disasm. If it does std::string-copy from a source pointer (no transformation), document that with PC ranges and STOP — the actual formatter lives upstream (inside the deleted-module callback at 0x6cc2430ed0, which sibling path `callback-body-port` is working on). If it DOES perform a 32→48-char transformation (likely hex-encode pattern: take 32 bytes, output 64 hex chars, then somehow extract 48? OR take 24 bytes, hex-encode to 48 chars), identify the exact transformation rule.

3. **Port to Rust + verify.** Create a `cert_format/` crate in your worktree. Implement the formatter as a pure function. Write a test that consumes `raw32_after_13` for `AABBCCDDEEFF0011` (or whichever challenge produces a known internal state per `cert-rust-repro`'s fixtures) and asserts the produced cert equals `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`. If ANY vector matches, set fact `cert_format_48ch_rust_port_2026_05_11` with the file path.

## Constraints & gotchas

- **No git commits, no git state changes.** Edit only in your worktree.
- **Write code freely.** Treat this as a normal sub-agent reverse + port task.
- **The 48-char "ASCII" cert is mostly hex** (e.g. `8F868A5849...` is 48 chars, all `[0-9A-F]`). So it's `hex(some_24_bytes_subset_of_raw32).upper()` — i.e. take 24 bytes from the 32-byte intermediate and hex-encode uppercase. Discover which 24 bytes (e.g. `raw32[4..28]`? `raw32[0..24]`? — algorithm answers this).
- **STOP-and-report cases**: (a) you cannot find a shard covering `0x78c6953ef8` (means libnmsssa.so isn't in the snapshot we think — escalate); (b) 0x123288 disasm is clearly a thin copier with no transformation — report PCs of the actual format-conversion site for the sibling worker; (c) your port produces wrong-shape output (not 48 chars / not hex) — report exact mismatch.

## Relevant files / references

- Sibling path: `callback-body-port` (worker `a19390e77af0b0a8e`, worktree `/home/sdancer/nmss-emu-callback-body-port/`). Stay out of its files.
- Hypothesis row in `/home/sdancer/orchestrator/analysis/hypotheses.md` (cert-format-48ch).
- Harness DB at `http://127.0.0.1:3000` (env `.env`). Relevant facts: `cert_jni_return_is_48char_ascii_2026_05_03`, `cert_callback_chain_resolved_2026_05_03`, `cert_remaining_gap_2026_05_11`, `cert_F7B3B00F8A5_module_base_correct`.

## Progress log
Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_format_48ch_progress_2026-05-11.jsonl`.

## Operating mode
In-process Agent (background). Iterate tasks 1→2→3. Time budget: ~2h.
