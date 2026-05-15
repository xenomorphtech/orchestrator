# cert-writepoint-bp (H-N3, refined) — capture the cert at its write-point via HW-BP

**You ARE allowed and expected to write code.** Python (ptrace, patched native-replay-rs invocation), shell.

## Role & workdir

Patch native-replay-rs to install a hardware breakpoint at module-rel `0x17eeec` (the `str q0, [x25]` instruction — H-N2's localized cert write-point). On hit, dump `q0` (first 16 bytes of cert), `x8` (next 8 bytes via `str x8, [x25, #0x10]` or similar), `x25` (destination address), and `sp+0x220..sp+0x238` (the source 24-byte cert buffer). Run for ANY challenge — collect verified `(challenge, cert)` ground-truth pairs without needing to lift the orchestrator function. Workdir: `/home/sdancer/nmss-emu-cert-writepoint-bp/`.

## Why this path

H-N2 (cycle 47-49) **hard-gated** the symbolic-exec port of the cert orchestrator at `0x17ded0..0x180aa8`. Major findings produced:
1. The cert is NOT computed by stage_drv(w1=0xe) — that only returns a success flag. The actual cert comes from `bl 0x11b104` (PC 0x17ee1c) and downstream chain through encoder `0x1113c0`.
2. **Cert write-point identified**: `0x17eeec` (`str q0, [x25]`) is where the first 16 bytes of cert get written to memory. `q0` holds the bytes; `x25` is the dst pointer.
3. NEON instructions limit miasm dis_multiblock — symbolic-exec approach blocked.
4. **🚨 GROUND-TRUTH CORRUPTION**: 4 of 5 `test_vectors_2026-04-24/*.json` are corrupt — all return the 7BDA cert. Only 1 reliable ground truth pair exists.

This path BYPASSES symbolic exec entirely. HW-BP at the write-point lets native-replay-rs do the work — we just observe the cert as it's written. This is the cheapest path to:
- Verified `(challenge, cert)` ground truth for any challenge (re-population of test_vectors)
- Proof native-replay-rs is genuinely producing per-challenge distinct certs (re-validates the algorithmic-not-bypass claim from cycle 17)

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (metric 0 → 5 if we can produce certs for arbitrary challenges via the patched binary; counts as "Rust" because native-replay-rs IS a Rust program, even if it uses the snapshot).
- **Sub-goal:** Verified ground-truth corpus and cert-extraction tool, unblocking any future Rust port.

## Success criteria

- **Minimum**: HW-BP at `0x17eeec` fires for all 5 (originally-tested) challenges; dumps q0+x8+x25+sp+0x220..0x238 into `analysis/cert_writepoint_captures.jsonl`. Cert bytes match (cycle-41's pre-stage_drv-x0 carrier+0x228 → final cert at memory location N). Set fact `cert_writepoint_captured_2026_05_11`.
- **Stretch**: Generate 50+ fresh `(challenge, cert)` pairs and write `analysis/ground_truth_v2_2026-05-11.json` — replaces the corrupt 2026-04-24 vectors. Cross-validate against cycle-41 HW-BP I/O. Set fact `cert_ground_truth_v2_2026_05_11`.
- **Hard gate**: HW-BP at 0x17eeec doesn't fire (means write-point address is wrong, or the function path doesn't reach it in native-replay-rs's call sequence) → write `analysis/writepoint_blocker.md`, escalate.

## Inputs you have

- **Patched native-replay-rs binary** on remote with `--trace-call-hw`: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`. The cycle-41 worker added NT_ARM_HW_BREAK support. **Extend this** to install HW-BP at a configurable address (currently only stage_drv 0xbe324 is hooked). Add `--hw-bp-addr <hex> --hw-bp-dump-spec <spec>` flag.
- **Cycle-41 NT_ARM_HW_BREAK code**: probably in `native-replay-rs/src/main.rs` near the existing `--trace-call-hw` implementation (or maybe in a helper file). Reuse the regset/ptrace infrastructure.
- **H-N2 cert-write-point disasm**: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/cert_orch_0x17ded0_disasm.txt` — search for "0x17eeec" to confirm the instruction and operands. Per H-N2: it's `str q0, [x25]` (first 16 bytes); `x8` likely holds bytes 16-23 via `stur x8, [x25, #0x10]` or similar at nearby PC.
- **Native-replay-rs source** for HW-BP infrastructure: `/home/sdancer/nmss-emu/native-replay-rs/src/main.rs` + on remote at `/root/nmss-emu-trampoline/native-replay-rs/`.
- **5 challenges to test**: the original 5 from `test_vectors_2026-04-24/summary.json` (challenges, not certs — challenges are likely correct, only certs are corrupt) — use them as test challenges.
- **Ground-truth-corruption note**: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/port_blockers.md` (H-N2's deliverable).

## Next 3 ordered tasks

1. **Extend patched native-replay-rs** to support a configurable HW-BP. Add CLI flags `--hw-bp-addr <hex>` and `--hw-bp-dump-spec "<comma-separated dump items, e.g. 'q0,x8,x25,sp+0x220:0x18'>"`. Reuse the cycle-41 NT_ARM_HW_BREAK infra. On hit, write a JSON line to `<out_dir>/hw_bp_hits.jsonl` with `{challenge, hit_pc, regs, mem_dumps}`. Build and push to remote.

2. **Run for 5 original challenges**. Use the existing test_vectors challenges (the challenge ASCII is correct; only the corrupt-cert column is wrong). For each: `native-replay-rs <snapshot> --challenge <hex> --hw-bp-addr 0x17eeec --hw-bp-dump-spec 'q0,x8,x25,sp+0x220:0x18' --out cert_writepoint_<chal>.json`. Confirm 5/5 hit the BP and dump distinct certs.

3. **Validate + write ground_truth_v2**. The 24-byte cert is `sp+0x220..sp+0x238` per H-N2. Confirm `q0[0..16] == sp[0x220..0x230]` (i.e., the str is just writing what was at sp+0x220). Cross-check that 5 DIFFERENT challenges produce 5 DIFFERENT certs (no duplicate-7BDA leakage). Write `analysis/ground_truth_v2_2026-05-11.json` with `{challenge_hex: cert_hex}` for all 5. If 4/5 are still duplicate-7BDA, the corruption is deeper than test_vectors_2026-04-24 — escalate as a campaign-level revelation.

## Constraints & gotchas

- **No git commits.**
- **Native-replay-rs is Rust + ptrace-based**: extending the HW-BP infra means modifying both the parent (ptrace controller) and confirming the BP register `NT_ARM_HW_BREAK` slot is free. Cycle-41 already proved this works.
- **`q0` is a 128-bit NEON register** — dump via `PTRACE_GETREGSET` with `NT_ARM_FPSR` or `NT_PRFPREG`/`NT_FPREGSET` (the q-regs are mapped from v0-v31 in fpsimd state). Cycle-41 worker likely already handles this; check its code.
- **Hit count**: the function path executes 0x17eeec when the cert-success branch is taken. If x0 from stage_drv(w1=0xe) is the success flag and the cmp+b.ne dispatches there, the BP fires ONCE per successful cert call. If it never fires, the function may be early-returning before reaching 0x17eeec — try setting the BP at 0x17ee1c (the bl 0x11b104) instead, which is upstream.
- **DON'T attempt to lift the function** — that path is hard-gated.

## Relevant files / references

- H-N2 disasm: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/cert_orch_0x17ded0_disasm.txt`
- H-N2 port_blockers.md: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/port_blockers.md`
- Cycle-41 HW-BP I/O table: `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl`
- Patched binary location: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`
- Native-replay-rs source: `/home/sdancer/nmss-emu/native-replay-rs/src/main.rs`
- Corrupt test_vectors: `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json` (4 of 5 cert values are duplicate)

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_writepoint_bp_progress_2026-05-11.jsonl`. Stages: `binary_patched`, `hw_bp_installed`, `hit_<challenge>`, `5x_done`, `ground_truth_v2_written`.

## Operating mode

In-process Agent (background). 2h budget. STOP on:
- (a) 5/5 challenges produce verified distinct certs at 0x17eeec → escalate as `cert_ground_truth_v2_recovered`, metric bump.
- (b) HW-BP doesn't fire on any challenge → write blocker doc, suggest moving BP upstream to 0x17ee1c (the bl into encoder).
- (c) All 5 challenges produce the SAME cert (duplicate-7BDA leakage at the writepoint) → MAJOR campaign-level finding: native-replay-rs is NOT actually algorithmic — the snapshot encodes a single cert. Escalate to user.
