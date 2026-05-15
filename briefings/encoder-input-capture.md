# encoder-input-capture (H-N4) — capture the encoder's (input, output) pairs via HW-BP at 0x17ee1c

**You ARE allowed and expected to write code.** Python (driver), Rust (extend native-replay-rs if needed).

## Role & workdir

Set hardware breakpoints at `0x17ee1c` (just before `bl 0x11b104` into the encoder) AND at `0x17ee20` (LR, just after the bl returns) — capture caller-saved regs + sp+0x180..sp+0x230 stack window at BOTH points to recover `(encoder_input, encoder_output)` pairs per challenge. With 5 known (input, output) pairs, the 131-insn encoder at `0x1113c0` becomes tractable to algorithmically lift and port. Workdir: `/home/sdancer/nmss-emu-encoder-input-capture/`.

## Why this path

H-N3 (cycle 54) delivered:
- 5/5 distinct certs captured at writepoint `0x17eeec`. native-replay-rs confirmed algorithmic. ground_truth_v2 written.
- The cert ASCII-hex bytes live at heap pointer `*[sp+0x230] = x8 = 0xb4000079f50a0b60` (NOT in q0; q0 is std::string struct header).
- Function returns std::string by value; 0x17eeec is 24-byte struct copy.
- **H-N3's parting recommendation**: HW-BP at `0x17ee1c` (the bl into the encoder at 0x11b104 → eventually 0x1113c0) captures encoder I/O directly, bypassing std::string ABI.

This path delivers the **final missing piece**: encoder inputs. With (input, output) pairs known for 5 challenges, the 131-insn encoder is small enough to symbolic-lift via miasm (cycle-30 framework) OR to brute-force-fit an algorithmic candidate (HKDF / SHA folds / AES-NI bytecode).

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (metric 0 → 5 if encoder is ported and reproduces certs); `nmss_cert_transformation_recovered` (metric 0.97 → 1.0).
- **Sub-goal:** 5 verified `(encoder_input, encoder_output)` pairs at `analysis/encoder_io_2026-05-11.jsonl`.

## Success criteria

- **Minimum**: HW-BP at `0x17ee1c` fires for all 5 challenges; dumps `regs + sp+0x180..sp+0x238 + relevant heap derefs` at hit. Same at `0x17ee20` (post-encoder, captures whatever the encoder wrote). Write `analysis/encoder_io_2026-05-11.jsonl` with 5 rows: `{challenge, input_state: {regs, stack}, output_state: {regs, stack, std_string_data}}`. Set fact `cert_encoder_io_captured_2026_05_11`.
- **Stretch**: Diff encoder inputs vs outputs to identify which bytes/regs are the actual encoder I/O (vs caller-frame noise). Save `analysis/encoder_io_diff.json` and recommend the algorithmic-lift approach (miasm symbolic vs algorithm-fit) based on input/output shape.
- **Hard gate**: HW-BP at 0x17ee1c doesn't fire OR encoder is called multiple times per cert (would require per-iteration capture) → write `analysis/encoder_capture_blocker.md` and propose alternative entry point.

## Inputs you have

- **Patched native-replay-rs with HW-BP infra**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`. Reuse the `--trace-call-hw <hex>` flag — H-N3 confirmed it works at any address.
- **H-N3 deliverables**: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/`:
  - `ground_truth_v2_2026-05-11.json` — 5 verified (challenge, cert) pairs
  - `cert_writepoint_captures.jsonl` — full HW-BP captures with reg/stack dumps at 0x17eeec
  - `cert_writepoint_summary.md` — H-N3's writeup
- **H-N2 disasm**: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/cert_orch_0x17ded0_disasm.txt` — search for "0x17ee1c" to confirm it's the `bl 0x11b104` instruction. Encoder at `0x1113c0` is downstream (H-N2 said 131-insn state machine, 6 memcpy).
- **Cycle-41 HW-BP I/O table**: `/home/sdancer/orchestrator/analysis/stage_drv_io_table.jsonl` (per-stage_drv-call data; cross-reference if needed).
- **5 challenges**: same as H-N3's set (`0x00`, `0123`, `1111`, `7BDA`, `AABB`).
- **Module dump**: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`.

## Next 3 ordered tasks

1. **Verify the HW-BP-pair logic**. Disasm `0x17ee14..0x17ee30` from the module dump. Confirm `0x17ee1c` is a `bl <target>` instruction with target resolving to `0x11b104` per H-N2's claim. Confirm the LR is `0x17ee20`. Save `analysis/encoder_call_site.json` with the verified addresses.

2. **Run double-HW-BP capture for 5 challenges**. The patched binary supports `--trace-call-hw <hex>` per H-N3; check if it accepts multiple addresses. If not, do 2 runs per challenge (one BP at 0x17ee1c, one at 0x17ee20). For each challenge: capture `regs + sp+0x180..sp+0x238 (152 bytes)` at both points. The diff between pre-bl and post-bl captures localizes WHAT the encoder writes. Save 10 JSONs (2 per challenge) and consolidate into `analysis/encoder_io_2026-05-11.jsonl`.

3. **Diff + recommend lift approach**. Compute byte-diff of pre/post stack windows AND reg deltas. Build a table `{challenge, encoder_input_signature, encoder_output_bytes}`. If the encoder reads a tightly bounded input set (e.g. only 16-32 bytes of stack + 2-3 regs) and writes a known-shape output (e.g. 48-ASCII-char hex), recommend miasm symbolic-lift of `0x11b104..0x11b104+131insns` with concretized inputs. If the input is wider/looser, recommend brute-force algorithm-fitting (HKDF, SHA folds, AES) against the 5 (input, output) pairs.

## Constraints & gotchas

- **No git commits.**
- **The encoder bl-target 0x11b104 is per H-N2 — verify it's not a wrapper that calls the actual encoder at 0x1113c0**. H-N2's chain: `bl 0x11b104 → bl 0x1113c0` (the 131-insn state machine). So 0x17ee1c → bl 0x11b104 → bl 0x1113c0. Capturing at 0x11b104 entry / exit might give cleaner I/O than at 0x17ee1c / 0x17ee20.
- **--trace-call-hw multi-address**: if the patched binary only accepts ONE BP at a time, you'll need 2 runs per challenge OR extend the binary (1h work) to support comma-separated BPs.
- **Encoder may be called recursively or in a loop**: if 0x17ee1c fires multiple times per cert, capture all hits and number them (`hit_idx`).
- **Stack window 152 bytes (sp+0x180..0x238) per H-N3 finding** — covers parent frame variables. Adjust if encoder reads outside this range.

## Relevant files / references

- H-N3 deliverables: `/home/sdancer/nmss-emu-cert-writepoint-bp/analysis/`
- H-N2 disasm + port_blockers.md: `/home/sdancer/nmss-emu-native-replay-orch-port/analysis/`
- Patched native-replay-rs binary: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`
- Native-replay-rs source: `/home/sdancer/nmss-emu/native-replay-rs/src/main.rs` (+ H-N3's NT_PRFPREG/NT_ARM_HW_BREAK extensions)
- Module dump: `/home/sdancer/nmss-emu-callback-frida/analysis/callback_capture_2026-05-11/full_deleted_module_8BC022520D197B4C07F1_6237.bin`
- 5 challenges to test: 0000000000000000, 0123456789ABCDEF, 1111111111111111, 7BDA93D2F45D36C0, AABBCCDDEEFF0011

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/encoder_input_capture_progress_2026-05-11.jsonl`. Stages: `bl_site_verified`, `hw_bp_pre_installed`, `hw_bp_post_installed`, `5x_pre_captured`, `5x_post_captured`, `diff_done`, `lift_approach_recommended`.

## Operating mode

In-process Agent (background). 2h budget. STOP on:
- (a) 5/5 (input, output) pairs captured + diff complete + lift approach recommended → set fact, escalate to H-N5 (encoder port).
- (b) HW-BP doesn't fire / encoder is recursive in a way that confounds capture → write blocker doc, propose hooking the encoder entry 0x1113c0 directly.
- (c) Encoder reads more state than captured (input is wider than sp+0x180..0x238) → expand capture window and re-run.
