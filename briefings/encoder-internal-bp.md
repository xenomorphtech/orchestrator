# encoder-internal-bp (H-N6) — minimal 5-BP capture inside the encoder

**You ARE allowed and expected to write code.** Python (HW-BP driver), shell.

## Role & workdir

Install 5 hardware breakpoints inside the encoder function at `0x20aad4..0x20b8e8` to capture intermediate state and determine whether the **cert is already present at the SSO buffer (x29-0x80) by PC `0x20b7a4`** — which would let us bypass the >4000-insn NEON primitive at `0x20bb48` entirely. Workdir: `/home/sdancer/nmss-emu-encoder-internal-bp/`.

## Why this path

H-N5 (cycle 60) hard-gated on the 902-insn encoder because:
- Sub-callee `0x20bb48` is **>4000 NEON-heavy instructions** (no ret found in 4000-insn scan; 52+ SIMD ops). Symbolic lift is infeasible.
- Cert producer is a C++ vtable dispatch `blr x8` at `0x20b6e4` to runtime addr `0x4eac70` — BSS region unresolvable from static dump.
- H-N5 observed cert ends up in x21 (SSO buffer at x29-0x80) by PC `0x20b7a4`.

**Tactical alternative** (per H-N5 blockers doc): if the cert is already populated in the SSO buffer at some HW-BP before the NEON primitive returns, we can shortcut the algorithmic recovery and treat the upstream code as opaque. The 5-BP minimal plan tests this hypothesis.

## Goal / sub-goal

- **Goal:** `nmss_cert_pure_rust` (metric currently 0; if cert is observable at intermediate BP for 5 challenges, "pure-Rust" reduces to a much smaller scope).
- **Sub-goal:** Identify the earliest BP where cert hex is observable in SSO buffer or pointer-derefable memory. That BP marks the "cert produced" line. Everything BEFORE it is what needs porting; everything AFTER is plumbing.

## Success criteria

- **Minimum**: All 5 BPs fire for ≥3 challenges; capture sp+0x180..0x238, x19-x28, x29-0x80..x29 (SSO buffer region), and pointer derefs. Save `analysis/internal_bp_captures.jsonl`.
- **Stretch**: At BP6 (return from 0x20bb48 → presumably PC `0x20b7a4` is the LR after `blr x8` at 0x20b6e4), check if x21 / x29-0x80 / x21-stored-pointer holds the 24-byte cert hex. If YES: campaign collapse — set fact `cert_observable_pre_neon_2026_05_11`, escalate (the >4000-insn NEON primitive is NOT needed to model).
- **Full**: Cert hex localized at the EARLIEST possible BP — defines the minimal lift target. Set `cert_minimal_lift_target_2026_05_11`.
- **Hard gate**: Cert only appears AT the writepoint 0x17eeec (already known) and nowhere between — means the entire `0x20aad4..0x20b8e8` is on the cert-producing path including the NEON beast. Then the campaign requires modeling the NEON primitive somehow (Triton, manual, or further-decomposed HW-BP).

## Inputs you have

- **Patched native-replay-rs** with `--trace-call-hw <hex>` (supports per-BP single addr): `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`. Check if it supports multiple BP addresses (H-N3 worked with one; may need extension).
- **H-N5 deliverables**: `/home/sdancer/nmss-emu-encoder-port/analysis/`:
  - `encoder_function_bounds.json` — 902 insns, 109 bls, 24 callees
  - `encoder_disasm.txt` — full 902-insn disasm
  - `encoder_subcallees_disasm.txt` — disasm of the 6 compute callees
  - `encoder_port_blockers.md` — refined 10-BP capture plan with the 5-BP minimal subset
  - `static_inputs.rs` — byte-perfect STATIC_STRUCT_64 + STATIC_HEAP_64 + 5-row ground truth
- **5 BP addresses** (minimal set from H-N5):
  - `0x20abd8` — entry of compute sub-path
  - `0x20ad4c` — mid-encoder state checkpoint
  - `0x20ad50` — immediately after 0x20ad4c (likely catches store)
  - `0x20b6e4` — the vtable `blr x8` dispatch (cert producer)
  - `0x20b7a4` — post-vtable; H-N5 believes cert is in x21 here
- **H-N4 encoder I/O ground truth**: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl` — 5 verified (challenge, cert) pairs.

## Next 3 ordered tasks

1. **Extend HW-BP infra if needed**. Check H-N3's patched binary — does `--trace-call-hw` accept multiple BPs in one run? If yes, install all 5. If not, extend to accept comma-separated list OR do 5 sequential runs per challenge.

2. **Run 5 challenges × all 5 BPs**. Capture at each BP: regs (x0-x30 + sp + pc), 256 bytes at sp+0x180..0x280, the SSO buffer at x29-0x80 (and 64 bytes beyond, since SSO buffer might be smaller than guessed), and any pointer-derefable memory pointed to by x21/x22/x23/x25 (cert candidates per H-N5 + H-N4). Total: 5 challenges × 5 BPs = 25 captures.

3. **Scan for cert presence**. For each capture, search for the known cert hex (from H-N4's ground truth) as a substring in: (a) all reg-pointed memory windows, (b) all stack dumps, (c) any reg value that LOOKS like a heap pointer (0xb4...). Report which BP is the FIRST to contain the cert. Save `analysis/cert_presence_per_bp.json`. If found at BP `0x20b7a4` (or earlier), confirm it's also there at 0x20b6e4 — that's the vtable dispatch return point.

## Constraints & gotchas

- **No git commits.**
- **BP ordering matters**: the 5 BPs are along the encoder's execution path; check execution order matches the JSONL hit order.
- **H-N5's cert location hypothesis is x21 = SSO buffer at x29-0x80 by 0x20b7a4** — but H-N5 didn't verify this empirically; that's task 3's job.
- **>4000-insn NEON primitive at 0x20bb48** between 0x20b6e4 and 0x20b7a4 is what we're hoping to skip. If cert is observable at 0x20b7a4 but NOT BEFORE the call into 0x20bb48 (i.e. at 0x20b6e4 entry), then the NEON primitive IS the cert producer and we have no shortcut.
- **5 ground truth certs available** for verification: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`.

## Relevant files / references

- H-N5 deliverables (especially encoder_port_blockers.md): `/home/sdancer/nmss-emu-encoder-port/analysis/`
- H-N4 ground truth: `/home/sdancer/nmss-emu-encoder-input-capture/analysis/encoder_io_2026-05-11.jsonl`
- Patched binary on remote: `root@162.244.80.97:/root/nmss-emu-trampoline/native-replay-rs/target/release/native-replay-rs`
- H-N3 binary source (NT_ARM_HW_BREAK + NT_PRFPREG): `/home/sdancer/nmss-emu-cert-writepoint-bp/native-replay-rs/src/main.rs`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/encoder_internal_bp_progress_2026-05-11.jsonl`. Stages: `bp_infra_ready`, `5x5_captured`, `cert_presence_localized`, `earliest_cert_bp_found_or_not`.

## Operating mode

In-process Agent (background). 2h budget. STOP on:
- (a) Cert observable at BP `0x20b7a4` OR earlier → set `cert_observable_pre_neon_2026_05_11`, escalate to user/orchestrator: this is the campaign-collapse outcome. Suggest H-N7 = port the small region BEFORE that BP.
- (b) Cert ONLY observable at 0x17eeec writepoint → confirms the >4000-insn NEON primitive IS the cert producer; escalate with H-N7 alternative (Triton symbolic lift OR manual AES/Speck identification of the NEON primitive).
- (c) BPs don't fire / can't extract cert from any window → write blockers, propose ptrace-based wider memory dump.
