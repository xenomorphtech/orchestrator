# neon-primitive-bp (H-N11) — HW-BP capture around the NEON cipher core at 0x78cd4a1b48

**You ARE allowed and expected to write code.** Rust (patch native-replay-rs HW-BP capture sites), Python (analyze captures, fit algorithm).

## Role & workdir

Capture the **input/output of the actual runtime cert primitive** — the >4000-insn NEON-heavy code at runtime PC `0x78cd4a1b48` (module-rel `0x20bb48`), called from libUnreal's encoder. Workdir: `/home/sdancer/nmss-emu-neon-primitive-bp/` (create with `git worktree add`).

This is THE campaign-closing path — H-N5 already worked out the BP plan; H-N10 just falsified MD5 and pointed us back here.

## Why this path

- **H-N10 (md5-input-capture) FALSIFIED the MD5 hypothesis.** The instrumented libUnreal MD5 sites do anti-tamper integrity hashing; the lib9781e236 MD5 cert_builder is NEVER hit in trampoline replay.
- **The real cert primitive** is at libUnreal `bl 0x20bb48` (= runtime `0x78cd4a1b48`). Per H-N5 encoder-port analysis: ≥4000 insns, NEON-heavy, output is the cert std::string body by BP 9 (module-rel `0x20b7a4`).
- **native-replay-rs IS verified 5/5** — oracle-service smoke test 5/5 against ground truth with this binary. Memory `project_native_replay_rs.md` is correct. The challenge IS applied correctly by native-replay-rs (it patches `[x24+0x10]` heap state, NOT just `chal_addr` — that's why the unicorn script failed and native-replay-rs succeeds).
- Native-replay-rs already supports `--trace-call-hw <hex>` HW-BPs. The 10-BP plan is fully spec'd in `/home/sdancer/nmss-emu-encoder-port/analysis/encoder_port_blockers.md` lines 81–135.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5).
- **Sub-goal:** capture cipher-boundary I/O at runtime BPs 5 + 6 + 7, and state schedule at BP 2, across all 5 challenges. With cipher input/output pairs + format string, the algorithm is identifiable.

## Success criteria

- **Minimum**: BPs 2, 5, 6, 7 captured for at least 2 challenges (5 best). Save `analysis/bp_captures_2026-05-11.jsonl` with `{challenge, bp_id, bp_module_rel, registers, memory_dumps}` records.
- **Stretch**: BP 7 format string identified + BP 5/6 cipher-boundary pair → cipher identified (AES / SipHash / custom). Set fact `cert_cipher_identified_2026_05_11`.
- **Campaign close**: Rust port validates 5/5 against ground truth → set fact `nmss_cert_5_5_pure_rust_reproduced` + escalate to user as CAMPAIGN COMPLETE.

## BP plan (from encoder_port_blockers.md lines 81–135)

Module-rel addresses; add `0x78cd0a0000` (the libUnreal base in the snapshot) → runtime PC. native-replay-rs takes runtime PCs.

| # | Module-rel BP | Runtime PC | Capture | Priority |
|---|---|---|---|---|
| 2 | `0x20abd8` | `0x78cd2abbd8`* | `x0`, dump 256B at `[x0]` | **HIGH** — state schedule reveal |
| 5 | `0x20ad4c` | `0x78cd2abd4c`* | `x0`, `x1`, dump `*x1` 32B | **HIGH** — cipher input |
| 6 | `0x20ad50` | `0x78cd2abd50`* | dump `[sp+0x80..sp+0x100]`, `[x29-0x80..x29-0x60]` | **HIGH** — cipher output |
| 7 | `0x20accc` | `0x78cd2abccc`* | `x8`, `x0` (fmt string addr → dump string @ x0) | **HIGH** — format string |
| 9 | `0x20b7a4` | `0x78cd2ab7a4`* | dump `[x21..x21+0x18]` (cert std::string) | confirm-cert-ground-truth |
| 1 | `0x20abd4` | `0x78cd2abbd4`* | `x0`, `w1`, `w2` | establish state-build input |
| 3 | `0x20abe4` | `0x78cd2abbe4`* | `x0`, `x1` | second-stage input |
| 4 | `0x20abe8` | `0x78cd2abbe8`* | `x0` | branch decision |
| 8 | `0x20b6d4` + `0x20b6e4` | `0x78cd2ab6d4`, `0x78cd2ab6e4` | `x21`+8 bytes, `x8`, post-state | vtable dispatch identity |
| 10 | `0x20b7f8` | `0x78cd2ab7f8` | `x0..x7`, 3 stack std::strings | final composition |

\*VERIFY the libUnreal base first — runtime PC of encoder is `0x78cd4a0ad4` (encoder entry) per H-N4. If encoder entry = base + 0x20aad4, then libUnreal base = `0x78cd4a0ad4 - 0x20aad4 = 0x78cd296000`. Verify before running BPs. (The 0x78cd4a1b48 number from earlier briefings = base + 0x20bb48 ⇒ base = 0x78cd296000. Cross-check.)

## Next 3 ordered tasks

1. **Verify libUnreal base in the snapshot.** Cross-reference encoder entry `0x78cd4a0ad4` (per `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`) with module-rel `0x20aad4` (per `encoder_function_bounds.json`). Confirm base = `0x78cd296000`. Compute all 10 runtime BP PCs. Sanity check: `0x78cd296000 + 0x20bb48 = 0x78cd2a1b48`. (Note: prior briefings used `0x78cd4a1b48`; need to reconcile — could be different module layout. Use H-N4 ground truth.)

2. **Run BP 7 first** (cheapest, single value). On `root@162.244.80.97`:
   ```
   cd /root/nmss-emu-trampoline && ./native-replay-rs/target/release/native-replay-rs \
     --challenge 7BDA93D2F45D36C0 --trace-call-hw 0x<BP7_runtime_PC> \
     --dump-mem-at-x0 64 --json-out /tmp/bp7_7BDA.json
   ```
   If `--dump-mem-at-x0` isn't supported, ADD that flag — see existing patched HW-BP infra in the binary's source at `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/src/`. Format string @ x0 reveals the cert composition recipe.

3. **Run BPs 5, 6, 2 across 5 challenges**. Save all captures to `analysis/bp_captures_2026-05-11.jsonl`. Then identify the cipher from the (input, output) pairs at BPs 5/6. If recognizable primitive → port to Rust → validate 5/5.

## Constraints & gotchas

- **No git commits.**
- **Don't write to the unicorn replay script.** Its `--challenge` flag is broken (per H-N10 blocker): writes wrong heap location. Use **native-replay-rs only**.
- **All HW-BP work happens on the remote ARM box `root@162.244.80.97`**. Use sshpass + scp to deploy patches; results are on remote; rsync back to workdir.
- **Confirm runtime PC base first** before pasting BPs into commands. Off-by-one on base means BP never fires.
- **Format-string reveal at BP 7 may collapse the campaign** — per encoder_port_blockers.md the fmt is likely at `0x4392a5` but may be a different format. If it's e.g. `"%08X%08X%08X..."` over 12 args, the cert is 12 hex words of state, easily extractable from state dump at BP 2.
- **BP 8** (`blr x8` vtable dispatch) — capture `x8` to identify the vtable target. The function name (if symbolic) may name the cipher.
- **Stack offset note**: BPs 5/6 ARE inside the encoder, which sets up its own frame. Use `sp` + the offsets in the table; don't relative-address by `x29` unless the table specifies it.

## Relevant files / references

- **THE plan**: `/home/sdancer/nmss-emu-encoder-port/analysis/encoder_port_blockers.md` lines 81–135
- **Encoder bounds**: `/home/sdancer/nmss-emu-encoder-port/analysis/encoder_function_bounds.json`
- **Encoder disasm**: `/home/sdancer/nmss-emu-encoder-port/analysis/encoder_disasm.txt` (902 insns)
- **H-N4 ground truth (5 challenge→cert pairs)**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- **Patched native-replay-rs**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs` (with `--trace-call-hw` already supported)
- **Native-replay-rs source**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/src/`
- **H-N10 blocker (read before starting)**: `/home/sdancer/nmss-emu-md5-input-capture/analysis/md5_input_blocker.md` — context on why MD5 is falsified and why the unicorn replay can't be used.
- **MD5 paths now FALSIFIED**: md5-sha-fit ran AGAINST the wrong primitive (lib9781e236, never executed); md5-input-capture captured anti-tamper hashes instead of cert primitive input.

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/neon_primitive_bp_progress_2026-05-11.jsonl`. Stages: `base_verified`, `bp7_captured`, `bp_5_6_captured`, `cipher_identified`, `rust_port_5_of_5_CAMPAIGN_COMPLETE`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) 5/5 cert match → **CAMPAIGN COMPLETE**. Set facts: `cert_cipher_identified_2026_05_11` + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.
- (b) Cipher identified but Rust port at <5/5 → set partial fact, write residual blockers.
- (c) BP 7 reveals format string but cipher (BP 5/6) is still mystery → set `cert_format_string_2026_05_11` fact, propose BP-2 (state schedule) follow-up.
- (d) HW-BPs unsupported or `--trace-call-hw` doesn't fire → write blocker, fall back to inline Frida instrumentation on device (oracle-on-device.md style).
