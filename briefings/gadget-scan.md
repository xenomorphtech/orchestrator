# gadget-scan — crypto-constant byte-pattern sweep on cert core disasm

**You ARE allowed and expected to write code.** Python (regex/grep), shell.

## Role & workdir

Scan the disasm of cert builder `0x78c689575c` (553+ insns) AND the gated core `0x78c68e2b68` (≥5000 insns) for **byte-patterns of known cryptographic primitive constants** — AES S-box, SHA-256 round constants, MD5 T-table, ChaCha20 magic, Speck rotation immediates, etc. A single hit identifies the primitive and shortcuts both algorithm-fit and HW-BP capture. Workdir: `/home/sdancer/nmss-emu-gadget-scan/`.

## Why this path

K=6 planner cycle 68 recommended this as the **cheapest kicker** (one grep over disasm + memdump shards). Likely to shortcut H-N8 and algorithm-fit if it lands.

## Goal / sub-goal

- **Goal:** `nmss_cert_primitive_identified` (currently 0.0 → 1.0 if a constant hit names the primitive uniquely).
- **Sub-goal:** Identify the cert primitive by finding its load-bearing constants in the disasm / memdump.

## Success criteria

- **Minimum**: Scan completed against all known constants. Save `analysis/gadget_hits.json` with `{primitive, constant_name, hit_addr, context}`.
- **Stretch**: ≥1 unique-to-primitive hit → primitive identified. Set fact `cert_primitive_identified_2026_05_11`.
- **Hard gate**: zero hits in any known primitive's constant table → primitive is custom OR uses runtime-derived constants → escalate.

## Inputs you have

- **H-N7 cert_builder disasm**: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_builder_0x78c689575c_disasm.txt` (553 visible insns).
- **H-N7 cert_core disasm**: `/home/sdancer/nmss-emu-cert-producer-port/analysis/cert_core_0x78c68e2b68_disasm_first20k.txt` (first 20k chars of ≥5000-insn core).
- **Module memdump shards**: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78c6*.bin` — raw bytes including the cert core's full 5000+ insns + any constant tables in adjacent .rodata.
- **Known crypto constants** (Python lists below or fetch from any crypto library):
  - AES S-box (256 bytes, starts `63 7c 77 7b f2 6b 6f c5 30 01 67 2b fe d7 ab 76 ...`).
  - AES Inverse S-box (256 bytes, starts `52 09 6a d5 30 36 a5 38 bf 40 a3 9e 81 f3 d7 fb ...`).
  - AES Rcon (10 bytes: `01 02 04 08 10 20 40 80 1b 36`).
  - SHA-256 round constants K[0..63] (64 × 4 bytes, starts `0x428a2f98 0x71374491 0xb5c0fbcf 0xe9b5dba5 ...`).
  - SHA-256 initial H[0..7] (`0x6a09e667 ...`).
  - SHA-1 constants (`0x5a827999 0x6ed9eba1 0x8f1bbcdc 0xca62c1d6`).
  - SHA-1 initial H[0..4] (`0x67452301 ...`).
  - MD5 T-table (64 entries) + initial state.
  - ChaCha20 magic: `"expand 32-byte k"` (16 bytes) and `"expand 16-byte k"`.
  - BLAKE2 IV.
  - Speck rotation immediates: `lsl #3` and `lsl #8` are signature of Speck128/128.
  - SipHash magic: `"somepseudorandomlygeneratedbytes"` is uncommon; SipHash uses constants `0x736f6d6570736575 0x646f72616e646f6d 0x6c7967656e657261 0x7465646279746573`.

## Next 3 ordered tasks

1. **Build the constant table** as a Python dict `{primitive_name: {constant_name: bytes_literal}}`. Include all common variants.

2. **Scan disasm + memdump**. For each (primitive, constant) pair, grep:
   - In disasm: search for the hex representation of the constant (e.g. `0x428a2f98`) in immediate operands.
   - In memdump shards: byte-search for the constant in .rodata regions. Use `grep -boba` or Python `bytes.find`.
   - Also: look for `lsl #3` and `lsl #8` patterns in the disasm — Speck rotation signature.

3. **Report findings**. Save `analysis/gadget_hits.json`. If a constant hits, also check whether it's a UNIQUE-to-primitive identifier (e.g. SHA-256 K[0]=0x428a2f98 is unique; `0x67452301` could be SHA-1 initial OR a common byte pattern).

## Constraints & gotchas

- **No git commits.**
- **Cycle time: ~10-30 min**. Cheap and fast.
- **AES T-table** (4 × 1KB) is a strong signature — if found, the primitive is AES.
- **SHA-256 K[0..63]** is the strongest signature — if found in any .rodata, almost certainly SHA-256.
- **Speck128** doesn't use constants — only rotation immediates. Check for `lsl #3` AND `lsl #8` in the disasm (signature pair).
- **HMAC** doesn't add new constants beyond its underlying hash; if SHA-256/MD5 K-tables are found, HMAC is the likely use case.

## Relevant files / references

- H-N7 cert_builder/cert_core disasm: `/home/sdancer/nmss-emu-cert-producer-port/analysis/`
- Module memdump shards: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`
- Reference crypto constants: Python's `cryptography.hazmat.primitives.hashes` source OR online tables.

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/gadget_scan_progress_2026-05-11.jsonl`.

## Operating mode

In-process Agent (background). 1h budget. STOP on:
- (a) Unique primitive constant hit → set fact, escalate.
- (b) Multiple primitive hits (suspicious — could be the snapshot containing crypto libraries linked but unused) → list all hits with context for resolution.
- (c) Zero hits → primitive is custom OR uses runtime-derived constants → escalate to H-N9 (more exotic primitive options).
