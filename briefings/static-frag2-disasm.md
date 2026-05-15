# static-frag2-disasm

## Role & workdir
You own the **device-independent algorithmic decode** of the late-frag2 family. While Path A (cert-ptrace, Frida live capture) and Path B (oracle-on-device, on-device replay) are blocked on device transport, you derive the late-frag2 algorithm purely from static disassembly of `/tmp/libnmsssa.so` — no captures needed. You operate in worktree `/home/sdancer/nmss-emu-static-frag2/` (branch `static-frag2-disasm`).

## Goal / sub-goal
- Top-level: `nmss_cert_re_algorithmic`. We have 16/24 bytes of the cert decoded by the algorithmic reproducer at `cert-rust-repro/src/bin/cert_rust_repro.rs`. The trailing 8 bytes come from the late-frag2 family.
- Your specific question: **What algorithm is the late-frag2 corridor implementing?** PCs published by cert-re-6: 0x15fc18 (seed sp+0x440), 0x15fc4c (first append helper, STRONGEST candidate for len=0x41/cap=0x50 family emergence), 0x15fcb4 (second family materialization at x29-0xc0). Helper map: 0x162364 (libc++-string copy from ptr+len), 0x1621b0 (splice/insert helper).

## Success criteria
You succeed when ANY of:
1. You identify a recognizable crypto primitive (SHA-1, SHA-256, MD5, blake2, custom Feistel, etc.) operating in this corridor and produce a Rust port hypothesis that matches one of the known cert tail-bytes (e.g. for challenge `7BDA93D2F45D36C0` the cert is `90237F0E03DF6993A54669AA7CF27E36304273143AD6A030` — bytes 16..24 = `304273143AD6A030`). Set fact `static_frag2_algo_identified=<one_line_summary>` and write a Rust hypothesis runner.
2. You produce a structural decode (dataflow + control flow) sufficient to reproduce in Rust without captures, even if you can't name the primitive. Set fact `static_frag2_structural_decode_done=<checkpoint_path>`.
3. You produce a definitive negative — the corridor is too obfuscated/data-dependent for static-only reconstruction. Set fact `static_frag2_static_only_walled_with_<reason>` and document what runtime data would unblock.

## Background
- Algorithmic reproducer (16 bytes, working): `cert-rust-repro/src/bin/cert_rust_repro.rs`. Algorithm: single-block SHA-256 over 64B sp+0x968 with bswap32 per word, take digest[4..28].
- Phase 1 memory note: `~/.claude/projects/-home-sdancer/memory/nmss_cert_phase1_algorithm.md`.
- Live-PC checkpoint (cert-re-6, 2026-05-02 20:40): `analysis/checkpoints/live_libnmsssa_late_frag2_pcs_2026-05-02.json`.
- The 5 ground-truth (challenge → cert) pairs are in `analysis/checkpoints/native_cert_*_clean_session_2026-05-02.json` from cert-ptrace's cycle 730 capture.
- Ground truth tail-bytes for the 5 challenges (cert bytes 16..24, hex):
  - 7BDA93D2F45D36C0 → `304273143AD6A030`
  - 0123456789ABCDEF → `F7380A33CC78B030`
  - 0000000000000000 → `1036F3C3A65E0B47`
  - FFFFFFFFFFFFFFFF → (in checkpoint files)
  - AABBCCDDEEFF0011 → (in checkpoint files; verify)
  Confirm exact bytes from the snapshot tests in `cert-rust-repro/`.

## Progress so far
- New agent (cycle 818). Worktree fresh from main HEAD `c654108`.
- All prerequisites available locally: `/tmp/libnmsssa.so` exists; the published PCs and helper offsets are in the checkpoint above.
- **Predecessor work**: cert-re-6 already mapped these PCs as structural analogues to the snapshot's old corridor. The snapshot corridor used selectors 0x86/0x57; the live corridor uses shifted stack slots (sp+0x440 / x29-0x100 / x29-0xc0). cert-re-6 cautioned this is "structural analogue, not byte-identical translation". Read that checkpoint carefully before trusting any PC equivalence.

## Next 2-3 concrete tasks
1. **Disassemble the corridor**: pick your tool (objdump, radare2, ghidra-headless, capstone) and produce a clean function-level disasm of the basic blocks reachable from 0x15fc18, 0x15fc4c, 0x15fcb4, plus helpers 0x162364 and 0x1621b0. Output to `analysis/checkpoints/static_frag2_disasm_corridor_2026-05-02.txt` (in your worktree). Note: this is the live ELF, so file offsets ≈ PC offsets (PIE; no ASLR baked in).
2. **Trace the dataflow**: identify what bytes go in (the seed copy at 0x15fc18 reads sp+0x440 — what's been written there before the call?). Identify what comes out (where is the produced 0x41-len/0x50-cap object stored?). Look for: SHA primitives (round constants 0x428a2f98 etc.), MD5 constants (0xd76aa478 etc.), bit-rotation patterns, lookup tables. Crypto signatures in disasm are often unmistakable.
3. **Hypothesis-run**: if you find a candidate primitive, write a small Rust hypothesis runner. Compare its output (fed with whatever input you can derive — possibly the cert prefix bytes, the challenge, or sp+0x968 contents) against the known tail-bytes for the 5 challenges. Even a single match across 5 vectors is informative. Save under `cert-rust-repro/src/bin/static_frag2_hypothesis_<n>.rs` in your worktree.

## Constraints & gotchas
- **DO NOT modify files outside your worktree.** Specifically don't touch `/home/sdancer/nmss-emu/`, `/home/sdancer/nmss-emu-ondevice/`, or `/tmp/libnmsssa.so` (read-only — it's the source of truth).
- The live ELF is 5MB+ — full disassembly is expensive. Be surgical: start with the function containing 0x15fc4c (strongest PC), follow callees one level deep before going wider.
- Crypto primitives have **constant fingerprints**: rg/grep the disasm output for `0x428a2f98|0x67452301|0x6a09e667|0x428a2f98|0xc6ef372f|0x510e527f|0x9b05688c|0x1f83d9ab|0x5be0cd19` (SHA-256 IVs) and `0xd76aa478|0xe8c7b756|0x242070db|0xc1bdceee|0xf57c0faf|0x4787c62a` (MD5 round constants). A single hit is decisive.
- The cert algorithm has been described as having anti-debug timing reads that don't feed the algorithm (memory note `nmss_cert_anti_debug_timing.md`). Don't get distracted by gettimeofday/clock_gettime calls inside the corridor.
- If you find the algorithm needs sp+0x440 or sp+0x968 contents that we DON'T have statically (only at runtime), document what you'd need and set the static-only-walled fact. That negative result is also valuable — it tells the orchestrator that the device transport recovery is actually the only path forward.
- Record progress in checkpoints — even partial findings (e.g. "block at 0x15fc18 does X, helper 0x162364 does Y") are valuable cross-pollination for cert-ptrace and oracle-on-device.

## Relevant files / references
- Worktree: `/home/sdancer/nmss-emu-static-frag2/`
- Live ELF: `/tmp/libnmsssa.so` (read-only)
- Live-PC checkpoint: `analysis/checkpoints/live_libnmsssa_late_frag2_pcs_2026-05-02.json`
- Algorithmic reproducer: `cert-rust-repro/src/bin/cert_rust_repro.rs` (your starting point for any Rust hypothesis)
- Known cert vectors: `cert-rust-repro/tests/` and `analysis/checkpoints/native_cert_*_clean_session_2026-05-02.json`
- Phase 1 memory: `~/.claude/projects/-home-sdancer/memory/nmss_cert_phase1_algorithm.md`
- Sibling agent on Path A: `/home/sdancer/orchestrator/briefings/cert-ptrace.md`
- Sibling agent on Path B: `/home/sdancer/orchestrator/briefings/oracle-on-device.md`

## Reporting cadence
After each meaningful step (disasm extracted / candidate primitive identified / hypothesis run), write a checkpoint and update facts. If you go an hour without a finding, write a status checkpoint anyway documenting what you've ruled out.
