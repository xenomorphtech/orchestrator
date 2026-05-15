# cert-re — NMSS cert algorithmic completion via static disasm

## Role & workdir
Static dataflow + CFG analysis from `/home/sdancer/nmss-emu/`. Use `aarch64-linux-gnu-objdump --adjust-vma=0x78c6896000 -D -b binary -m aarch64 trampoline_proc_memdump_5558/memdump/78c6896000.bin --start-address=0x... --stop-address=0x...` for static disasm. Aeon MCP `aeon.get_reduced_il`/`get_ssa` time out at 120s on large functions like `0x78c686b068` — fall back to objdump windowing immediately.

## Current goal / sub-goal
- Goal: `nmss_cert_re_algorithmic` — pure-algorithmic cert reproduction (no `--ctx-seed-240-text`, no skip flags).
- Sub-goal: `cert_commit_chain_disasm` — pin the exact PC + value-flow that writes the 32-hex frag1 string into cert+0x68.

## Status — original goal already operationally satisfied
`nmss_cert_re` was achieved via state-injection at cycle 591 (chained-bypass + `--ctx-seed-240-text`). Both donor challenges produce `replay_verified=true`. This worker now pursues the *algorithmic* track.

## Already mapped (don't redo)
- 7-slot primitive dispatch table at `0x78c686b7b8..0x78c686b80c` with 4 known slots: SHA-256 (`0x78c6879938`), 8-char reducer (`0x78c687dbcc`), xxHash64 (`0x78c68810c0`), MD5 (`0x78c686ccd4`).
- Algorithm chain: `0x78c689528c → 0x78c689575c → 0x78c6895b40 (bridge to 0x78c686a9a8) → 0x78c686aaa0 launcher → 0x78c686b068 mux → primitive → fragment object → formatter subtree at 0x78c6903ed4 → cert+0x68 commit at 0x78c6927f88`.
- 0x78c686a9a8 = formatting adapter (NOT the launcher — common confusion).
- Per-library digests captured in `analysis/checkpoints/donor_md5_input_capture_per_lib_7BDA_2026-04-27.json`.
- Manifest schema partially mapped in `analysis/checkpoints/manifest_md5_aggregation_logic_2026-04-27.json`.

## Success criteria
- `analysis/checkpoints/formatter_subtree_0x78c6903ed4_disasm_2026-04-27.json` exists with: prologue, 24-byte object commit ABI at 0x78c6903ed4/0x78c6903ed8, stack-slot provenance (where the 24-byte object came from), and the head-qword vs backing-bytes split for the libc++ long-string (cap, size, data_ptr).
- `analysis/checkpoints/cert_commit_0x78c6927f88_disasm_2026-04-27.json` exists with: the q0 store to [cert+0x70] context, where x22+0x70 is loaded from, and the value flow back to the 24-byte string object.
- `nmss_cert_replay_correct_pure_algo` fact eligible to set when the fragment-to-cert+0x68 transformation is fully understood.

## Next 2–3 concrete tasks

1. **Disasm formatter subtree at 0x78c6903ed4**: window like `--start-address=0x78c6903e80 --stop-address=0x78c6903fc0` (or wider). Identify the prologue, the 24-byte object commit pattern (this is the libc++ long-string materialization), and the input source that produces the 32-hex characters. Save `analysis/checkpoints/formatter_subtree_0x78c6903ed4_disasm_2026-04-27.json`.

2. **Disasm commit site 0x78c6927f88**: window like `--start-address=0x78c6927f00 --stop-address=0x78c6928020`. Trace backward from `str q0, [x22, #0x70]` to find where x22 was set up and where the 16-byte q0 came from (which sp slot, which earlier ldr). Save `analysis/checkpoints/cert_commit_0x78c6927f88_disasm_2026-04-27.json`.

3. **Connect the two**: with both checkpoints in hand, write the value-flow from the 7-slot primitive output (intermediate fragment) → 0x78c6903ed4 (object materialization) → 0x78c6927f88 (cert+0x70 commit). The frag1 string (32 hex lowercase) lives somewhere in this chain — find which fields hold it. Update WIKI.md if the algorithm map needs revision.

## Coordination with sister agents
- `cert-rust-reimpl` (pane `cert-rust-reimpl`) is building a handwritten Rust differential test using your captured digests. They need the formatter+commit disasm to translate post-primitive transformations into Rust code.
- `aeon-jit-perf` (pane `aeon-jit-perf`) is on the JIT-end-to-end track. Independent.
- Cross-pollinate via `harness fact-set <key> <value>`.

## Constraints & gotchas
- Use `python3` not `python`.
- objdump emits `get_sreg_qualifier_from_value` assertion warnings — pipe to sed/rg.
- objdump CANNOT disassemble the FULL 78c6896000.bin (3.18MB) at once — chunk by 0x10000-0x40000 windows.
- Don't redo work already in `analysis/checkpoints/` — read existing files first (e.g. `manifest_md5_aggregation_logic_2026-04-27.json`, `donor_md5_input_capture_per_lib_7BDA_2026-04-27.json`).
- Save partial JSON checkpoints early — pane sessions can crash.

## Relevant files / references
- `/home/sdancer/nmss-emu/WIKI.md` — current understanding (read this first).
- `/home/sdancer/nmss-emu/analysis/checkpoints/cert_writer_chain_0x78c68ef7d8_to_0x78c6927f88_2026-04-26.json` — formatter resume PC + writer chain.
- `/home/sdancer/nmss-emu/analysis/checkpoints/manifest_md5_aggregation_logic_2026-04-27.json` — aggregator schema.
- Donor expected certs: 7BDA → `90237F0E03DF6993A54669AA7CF27E36304273143AD6A030`, AABB → `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`. Frag1 = first 32 hex (lowercase).
- Facts: `harness facts | rg 'seven_slot|hash_primitives_three|aggregator_struct|per_module_digests'`

## Operating mode
Codex agent (gpt-5.4 xhigh). Save checkpoints incrementally. Don't spend more than 30 minutes on objdump tooling before producing first formatter checkpoint.

## Status correction (2026-04-30) — RESUME

A prior cycle declared Phase 1/2 "complete" because the downstream Rust solver self-consistently matched the no-bypass fixture's `641d96f7…`. **That output is not in ground truth.**

Ground truth lives at `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json` — 5 (challenge → expected cert) vectors, challenge-dependent:

- `0000000000000000` → `4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47`
- `0123456789ABCDEF` → `3083EFBA67F78ADBBD44EA727AF751E1F7380A33CC78B030`
- `1111111111111111` → `3933F258F2EDF72D3CDD7E31D075D1F34AE73DF72D60A16B`
- `7BDA93D2F45D36C0` → `90237F0E03DF6993A54669AA7CF27E36304273143AD6A030`
- `AABBCCDDEEFF0011` → `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C`

**Critical observation from `summary.json`:** the JIT/unicorn replay emits the same `90237F0E…` for ALL 5 challenges — challenge-insensitive on that substrate. The static-RE counterpart of this question: **trace where the `--challenge` value actually flows into the cert chain**. If the call graph is mapped fully but the challenge value never enters any of the documented chain (`0x78c689528c → 0x78c686a9a8 → 0x78c686aaa0 → primitives → fragment → 0x78c6903ed4 → 0x78c6927f88`), then the chain you've documented is downstream of where challenge enters — and that upstream is the missing piece.

**Resume objective:** find the challenge-input edge. Either (a) confirm `--challenge` flows into one of the documented PCs (e.g. as q0/q1 source, or as `[ctx+0x210]` derivation), or (b) find a separate code path between argv parsing and the chain root where `--challenge` is mixed into session-bound state. With that edge identified, the replay can be made challenge-sensitive and the 4 unseen vectors become reachable.

## Micro-task 2026-04-30 (post-7BDA late-vector capture) — IDLE TIME

cert-rust-reimpl decoded the late chain as: 0x140-stride 13-record vector folded by `raw32[i+1] = sha256(record_ascii_64[i] || raw32[i])`. Late seed for 7BDA: `6cc4294aef7bc2fb9e2bc515d3f71108561c29671172ac92da3e72c5ff20fd78`. Final raw32 (= sp+0x40c value for 7BDA): `BECA86489D2D6F7E305BEF0116BAEF2FDC4FE34B035E8D1F1D880D6D8C6F42D5`.

The fold loop is at `0x78c6916238..0x78c69163a8` (sp+0x40c writer region). 13 sha256 invocations match the 13-record count. **Open RE question:** what exactly produces the FINAL raw32 emitted at `0x78c69163a8`? Is it just `sha256(record_13_ascii || raw32_12)` directly, or is there a final transform (truncation? swap? pack? extra round)?

**Concrete task while ARM SSH is down:**
1. Window the loop tail with `aarch64-linux-gnu-objdump --adjust-vma=0x78c6896000 -D -b binary -m aarch64 /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78c6896000.bin --start-address=0x78c6916380 --stop-address=0x78c69163e0 | grep -v get_sreg_qualifier_from_value`.
2. Identify the exact sequence of stores that writes the final 32 bytes into the sp+0x408 string object (the writer landed `BECA8648…` for 7BDA per `cert_phase_c_late_fragment_join_2026-04-30.json`).
3. Trace x0/x1/x21 register provenance immediately before the final store. If any non-sha256 transform sits between the last sha256 call and the final write, document it in `analysis/checkpoints/cert_late_chain_iteration_terminator_2026-04-30.json` with: prologue PC, final-store PC, transform sequence, register diffs.
4. Set fact `cert_late_chain_iteration_terminator_disasm_2026_04_30 <path>` once shipped.

Why this matters: cert-rust-reimpl needs the EXACT terminator semantics to close the all-5-challenge port. They have the 7BDA captured vector_span + first record + late seed; the gap is the precise final transform.

Do NOT redo the early-chain RE — that's already mapped. Stay inside `0x78c69162xx..0x78c69163xx` for this task.
