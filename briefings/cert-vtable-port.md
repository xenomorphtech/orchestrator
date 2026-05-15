# cert-vtable-port (H-N12) — disasm + port the vtable cert-producer at 0x78cd4ba208

**You ARE allowed and expected to write code.** Rust (cert producer), Python (disasm helpers).

## Role & workdir

Disassemble the vtable target at runtime PC `0x78cd4ba208` (module-rel `0x21a208`) and the helper subcalls (`bl 0x60330`, `bl 0x5f120`, `bl 0x20a13c`), identify what they do to the cert-object at `x21`, and port to pure Rust. Workdir: `/home/sdancer/nmss-emu-cert-vtable-port/` (create with `git worktree add`).

This is the campaign-closing path. H-N11 left us with a **fully bracketed cert producer**: cert-object at x21 carries the per-challenge state; vtable method `0x78cd4ba208` reads it and produces the 48-char cert ASCII.

## Why this path

H-N11 stop case (c) findings (see `/home/sdancer/nmss-emu-neon-primitive-bp/analysis/neon_primitive_bp_blocker.md`):

- **bl 0x20bb48 is NOT the cipher** — q0..q18 identical across all 5 challenges at BP 5 AND BP 6; the call returns a constant libc pointer (0x7c452e97cc).
- **Real cert producer is downstream**: between BP 6 (`0x20ad50`) and `0x20b60c` (~152 instructions).
- **Cert ASCII first observable at `x20` at PC `0x20b60c`** (49-hit strlen loop). x20 = heap `0xb4000079f50a0b60` (same addr across all chals; chal-specific content).
- **Cert-object lives at x21 = `0xb400007a650b9930`** (same struct across all 5 chals). Per-challenge state at:
  - `[x21+0x00]`: cert-object header (?)
  - `[x21+0x10]`: ?
  - `[x21+0x40]`: q0 store — NEON 16-byte block (likely challenge-derived state)
  - `[x21+0x50]`: ?
  - `[x21+0x80]`: challenge ASCII (with leading space byte)
  - `[x21+0x88]`: ?
  - `[x21+0xe0..0xe4]`: ?
- **vtable target at BP 8b** (the `blr x8` at module-rel `0x20b6e4`) = **runtime PC `0x78cd4ba208`** (module-rel `0x21a208`). This is the cert-output method.
- BPs 7/9/10 are dead branches; vtable dispatch is the real cert path.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (0/5 → 5/5).
- **Sub-goal:** Disassemble `0x78cd4ba208` + helper subcalls + cert-object struct → port to Rust → validate against the 5 captured (cert-object, cert) tuples on disk.

## Success criteria

- **Minimum**: disassemble `0x78cd4ba208` (the vtable target) and produce a structural decomp (registers used, sub-calls, branching). Save to `analysis/vtable_0x21a208_disasm.txt` + `vtable_decomp_notes.md`.
- **Stretch**: Identify the algorithm = a single primitive (AES / SHA / printf-composition / Speck / custom). Port to Rust; validate.
- **Campaign close**: 5/5 → set fact `nmss_cert_5_5_pure_rust_reproduced` + escalate.

## Concrete tasks (ordered)

1. **Disassemble vtable target.** Locate the bytes for `0x21a208..0x21a208+0x1000` in the snapshot (or in `/system/lib64/libUnreal.so` if available). Try:
   - `objdump -d --start-address=0x21a208 --stop-address=0x21a808 /path/to/libUnreal.so` (if static binary available)
   - Otherwise: `ssh root@162.244.80.97` and pull the bytes from the snapshot memory at `0x78cd4ba208..0x78cd4ba208+0x1000`; pipe to `aarch64-linux-gnu-objdump -D -b binary -m aarch64`.
   - Snapshot is at `/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/` — look for the shard that contains 0x78cd4ba208.
   - Save to `analysis/vtable_0x21a208_disasm.txt`.

2. **Inspect cert-object struct in captures.** H-N11 saved 5/5 captures at `/home/sdancer/nmss-emu-neon-primitive-bp/analysis/raw_captures/`. The bp8a_8b files have x21 dump. Extract `[x21+0x00..x21+0x100]` for each of the 5 challenges and DIFF. Find which bytes are challenge-varying vs invariant. That + the vtable disasm = full picture of how challenge → cert.

3. **Disassemble helpers** `0x60330`, `0x5f120`, `0x20a13c` (module-rel; add `+0x78cd296000` for runtime). These run BEFORE the vtable — they populate the cert-object's fields. If one of them is a recognizable primitive (HMAC, SHA, AES), the campaign closes here.

4. **Port to Rust.** Implement a `fn produce_cert(challenge: &[u8; 16]) -> [u8; 48]` that:
   - Computes whatever the helpers compute to fill the cert-object's per-challenge fields (challenge ASCII at +0x80, NEON state at +0x40, etc.)
   - Then runs the vtable method's logic (format-string / std::stringstream / direct ASCII-hex emission).
   - Validate against the 5 captured (challenge → cert) pairs.

5. **If 5/5 → CAMPAIGN COMPLETE.** Set fact `nmss_cert_5_5_pure_rust_reproduced` with the algorithm name + spec.

## Constraints & gotchas

- **No git commits.**
- **Disasm is the critical path.** Without seeing 0x78cd4ba208's actual aarch64, all guesses are speculation. Get the disasm first.
- **libUnreal base = `0x78cd296000`.** All runtime PCs = base + module-rel. The snapshot memdump shards in `/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/memdump/` have raw bytes; the maps.txt has the layout.
- **The 5/5 captures are already on disk** — no need to re-run native-replay-rs unless you want fresh data. Just process them locally.
- **Don't trust prior briefings about "the cipher".** H-N11 falsified the bl 0x20bb48 hypothesis. The cipher concept may not apply — the cert may just be ASCII-hex formatting of pre-existing 24 bytes of state.
- **Hexpattern check first**: the cert is 48 chars / 24 bytes hex-encoded. Look at the cert-object struct dumps — IS there a 24-byte region that hex-encodes to the cert exactly? If yes, the "cipher" is just ASCII-hex and the real producer is whatever wrote those 24 bytes upstream (the cipher subcalls 0x60330/0x5f120/0x20a13c).

## Relevant files / references

- **H-N11 blocker**: `/home/sdancer/nmss-emu-neon-primitive-bp/analysis/neon_primitive_bp_blocker.md`
- **5/5 captures**: `/home/sdancer/nmss-emu-neon-primitive-bp/analysis/raw_captures/` (437MB total)
- **30-record BP capture summary**: `/home/sdancer/nmss-emu-neon-primitive-bp/analysis/bp_captures_2026-05-11.jsonl`
- **Ground truth (5 challenge→cert pairs)**: from H-N4 `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl` AND oracle fact `oracle_service_running_2026_05_11` AND the 5 cert strings in this briefing.
- **Snapshot for disasm**: `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/memdump/*.bin`
- **Patched native-replay-rs (for additional BP captures if needed)**: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/cert_vtable_port_progress_2026-05-12.jsonl`. Stages: `disasm_obtained`, `struct_diff_done`, `algorithm_hypothesis`, `rust_port_draft`, `5_of_5_match_CAMPAIGN_COMPLETE`.

## Operating mode

In-process Agent (background). 3h budget. STOP on:
- (a) 5/5 cert match → **CAMPAIGN COMPLETE**. Set facts `cert_algorithm_identified_2026_05_12` (with the algorithm name) + `nmss_cert_5_5_pure_rust_reproduced`. Escalate to user.
- (b) Algorithm identified but Rust port at <5/5 → set partial fact, document residual edge cases.
- (c) Disasm done + struct diff done but no recognizable algorithm → write `analysis/cert_vtable_port_blocker.md` with what was learned and prescribe finer bisection.
- (d) Can't obtain disasm (binary unavailable / snapshot layout opaque) → write blocker; fall back to runtime BP capture of vtable internals (set BPs INSIDE the vtable method).
