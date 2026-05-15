# cert-formatter-bp (H-N13) — capture formatter I/O inside 9781e236 cipher chain

**You ARE allowed and expected to write code.** Patch native-replay-rs (Rust), analyze captures (Python), implement Rust port.

## Role & workdir

Set HW-BPs INSIDE the actual cert-producing call chain in module `9781e236` (NOT libUnreal, NOT CFF3FAD10) — at cipher entry, inner cipher entry, and the snprintf-style formatter `0x78c686a9a8`. Capture format string + args + post-call dumps across all 5 challenges. The formatter chain produces the 48-char cert character-by-character. With per-call (fmt, args, output) tuples across 5 chals, the algorithm is identifiable empirically.

**Workdir**: `/home/sdancer/nmss-emu-cert-formatter-bp/` (create with `git worktree add`).

## Why this path

H-N12 stop case (c) findings (see `/home/sdancer/nmss-emu-cert-vtable-port/analysis/cert_vtable_port_blocker.md`):

- **The `blr x8` at 0x20b6e4 was a DESTRUCTOR call**, not the cipher. The cert already existed at `[x21+0x68]` before that BP.
- **Real cert producer fn-ptr = `0x78c689528c`** in module `9781e236` (a *different* anti-cheat blob — also deleted post-load; spans `0x78c678d000..0x78c6bda000`, 4MB).
- **Call site**: `blr x8` at PC `0x78cd4a1548` (module-rel 0x20b548 in CFF3FAD10) with `w0=2, x1=x23, x2=x21=cert-object`.
- **Inner chain**: `0x78c689528c` (switch on w0; case 2) → `0x78c689575c` (inner cipher, ~500 insns) → uses formatter **`0x78c686a9a8`** with multiple format templates at .rodata `0x78c6b3a2d0`.
- **Cert is NOT hex-encoded binary** — searched every `pointer_previews` for any 4/8/12/16/24-byte raw form of the cert; zero matches. Cert is character-synthesized by the formatter chain.
- **Class**: `CNSKitCertValue` (mangled `15CNSKitCertValue` in .rodata).
- **Format templates discovered**: `"%p%s%p"`, `"%p%s%s%s%s"`, `"%d%s%d%s%d"`, `"%s%s%s%d.."`, `"%d%s%d%s%d%s%d%s%s"`, `"%s, "`, `" d=%x,%x "`, `"type2:"`, `"0x%x,"`.

This reconnects to H-N8's killed-worker findings: lib9781e236 IS hit at runtime, just via dynamic fn-ptr. H-N10's falsification "lib9781e236 PCs never hit in replay" was wrong; they're hit indirectly.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5).
- **Sub-goal:** Capture (fmt_string, args, output) tuples at every formatter invocation across 5 challenges → reconstruct composition algorithm → port to Rust → validate.

## Success criteria

- **Minimum**: 5/5 challenges replayed with BPs at `0x78c689528c`, `0x78c689575c`, `0x78c686a9a8` (every hit), and inner-cipher-exit. Save to `analysis/formatter_captures_2026-05-12.jsonl` with `{challenge, bp, hit_idx, regs, mem_dumps, x0_string}` records.
- **Stretch**: Diff formatter args across challenges → identify the deterministic transform from challenge to cert chars. Save `analysis/formatter_decomp.md`.
- **Campaign close**: 5/5 pure-Rust → set fact `nmss_cert_5_5_pure_rust_reproduced` + escalate.

## Concrete tasks (ordered)

1. **Confirm BP infrastructure**. The patched native-replay-rs at `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` already supports `--trace-call-hw <hex>`. Check whether the `pointer_previews` feature already captures pointed-to memory at register values, OR if you need to add a `--dump-mem-at-reg <reg+offset:size>` flag for the formatter args.

2. **Run BP captures**. For each of the 5 challenges (`0000000000000000`, `0123456789ABCDEF`, `1111111111111111`, `7BDA93D2F45D36C0`, `AABBCCDDEEFF0011`):
   - Run native-replay-rs with HW-BPs at `0x78c689528c`, `0x78c689575c`, `0x78c686a9a8`, and inner-cipher-exit (~0x78c689575c + ~500 insns, need to determine; check disasm at `/home/sdancer/nmss-emu-cert-vtable-port/analysis/jit_cert_producer_disasm.txt`).
   - For `0x78c686a9a8` hits: dump `*x0` (fmt string) and `*x1`, `*x2`, `*x3`, `*x4`, `*x5` (args, up to 64B each). After the call also dump the output buffer.
   - Save all to per-challenge JSON files in `analysis/raw_captures/`.

3. **Cross-challenge diff**. For each formatter hit (same hit_idx across challenges):
   - Same fmt string across 5 chals? If yes, that's the deterministic template at that position.
   - Args: which bytes vary, which are constant?
   - Output bytes: how do they map to challenge bytes?
   
   This is the core reverse-engineering step. Save `analysis/formatter_decomp.md` with a per-hit table.

4. **Port to Rust**. Implement `fn produce_cert(challenge: &[u8; 16]) -> [u8; 48]` that:
   - Computes whatever deterministic transforms produce the formatter args from the challenge.
   - Runs a small `format!()` chain equivalent to the captured calls.
   - Outputs the 48-char ASCII cert.
   - Validate against 5 ground-truth (challenge, cert) pairs.

5. **If 5/5 → CAMPAIGN COMPLETE.** Set facts and escalate.

## Constraints & gotchas

- **No git commits.**
- **Module `9781e236` is in an rwxp page** (mutable executable — typical anti-cheat technique). The shards `78c678d000.bin` (1MB) and `78c6896000.bin` (3MB) are in the snapshot. Module spans `0x78c678d000..0x78c6bda000`.
- **The formatter `0x78c686a9a8` may not be stock vsnprintf**. If it emits uppercase hex for `%x` (vs lowercase that stock libc does), it's a custom impl. Disasm if Rust port disagrees.
- **CNSKitCertValue object at x21=`0xb400007a650b9930`** has these fields (from H-N12 decomp):
  - +0x00 vtbl*, +0x08 inner state ptr, +0x10 module rwdata ptr
  - +0x18 = 0xa29 (cert opcode), +0x1c chal-varying u32
  - +0x20..0x24 small ints (5, 2), +0x30..0x40 chal-varying
  - +0x40 q-reg NEON state (chal-varying)
  - +0x68 the 48-char cert std::string (size 0x30)
  - +0x80 " <challenge ASCII>" (leading space, std::string)
  - +0x98, +0xb0, +0xc8 chal-varying intermediate std::strings
- **Run capture at the orchestrator level**: ensure each replay sees the SAME challenge consumed end-to-end (use --challenge flag).
- **DO NOT use the unicorn replay** — it has a frozen-snapshot challenge bug (H-N10 finding). Use native-replay-rs only.
- **lib9781e236 IS hit in replay** — H-N10's previous claim "never hit" was wrong; the call is via dynamic fn-ptr from rwxp page, indirected through CFF3FAD10's encoder. Hits are real but were missed because PC-based grep won't catch the indirect call site.

## Relevant files / references

- **H-N12 blocker (read first)**: `/home/sdancer/nmss-emu-cert-vtable-port/analysis/cert_vtable_port_blocker.md`
- **H-N12 disasm**: `/home/sdancer/nmss-emu-cert-vtable-port/analysis/{vtable_0x224208_disasm.txt, jit_cert_producer_disasm.txt, cert_func_window_disasm.txt, ba194_step1_disasm.txt, ba990_step2_disasm.txt, vtable_decomp_notes.md}`
- **5/5 cert-object captures** from H-N11: `/home/sdancer/nmss-emu-neon-primitive-bp/analysis/raw_captures/` (437MB) — useful for cross-checking struct interpretation.
- **Ground truth (5 chal→cert pairs)**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl` OR oracle fact `oracle_service_running_2026_05_11`. Certs:
  - `0000000000000000 → 4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47`
  - `0123456789ABCDEF → 3083EFBA67F78ADBBD44EA727AF751E1F7380A33CC78B030`
  - `1111111111111111 → 3933F258F2EDF72D3CDD7E31D075D1F34AE73DF72D60A16B`
  - `7BDA93D2F45D36C0 → 90237F0E03DF6993A54669AA7CF27E36304273143AD6A030`
  - `AABBCCDDEEFF0011 → 8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`
- **Patched native-replay-rs**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` + source at `src/`.
- **Module 9781e236 shards**: `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/memdump/78c6{78d,896}000.bin`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_formatter_bp_progress_2026-05-12.jsonl`. Stages: `bp_infra_verified`, `captures_5x_done`, `formatter_args_diffed`, `algorithm_hypothesized`, `rust_port_draft`, `5_of_5_match_CAMPAIGN_COMPLETE`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) 5/5 cert match → **CAMPAIGN COMPLETE**. Set facts: `cert_formatter_algorithm_identified_2026_05_12` + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.
- (b) ≥3/5 match → set partial fact, document residual edge cases.
- (c) Captures successful but no recognizable composition pattern emerges → write `analysis/cert_formatter_bp_blocker.md` with the captured data and prescribe the next level (symbolic exec or full disasm of inner cipher).
- (d) HW-BPs at the dynamic-fn-ptr targets fire 0 times (e.g. because they're code-page-protected mutable) → write blocker, propose patching the orchestrator to log the fn-ptr value AND its target's first ~50 instructions on-the-fly.
