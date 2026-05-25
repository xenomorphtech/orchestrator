# magic32-live-snapshot-replay — Lane Z: symbolic execution of cert body

## Role & workdir
Claude pane worker, workdir `/home/sdancer/nmss-emu-magic32-live-snapshot-replay`. Branch `magic32-live-snapshot-replay`. **Continuing from Lane X commit `bfd8016`** — no /clear needed yet (you have plenty of context).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `lane-z-symbolic-execution-cert-body`

## Why Lane Z (parallel to user resource ask)
Lane X committed `bfd8016` — substrate ✓, cert fn stream captured, BL trace blocked by Netmarble "Mobile Phone Verification" SMS gate. The orchestrator has escalated the **fresh SMS-capable Netmarble account** ask to the user, but **DO NOT WAIT IDLE** — the Lane X verdict's own "Alt:" line names symbolic execution as the goal-close path that needs no device interaction. Run it now in parallel.

## What's in your hand (do NOT re-derive)
- **Live cert fn instruction stream**: 3 KB at `kit+0x197c00..0x198800`, byte-exact match to the Lane V claim. File: `outputs/lane_x_20260518_191300/cert_fn_live.bin` (or whatever path Lane X committed — check git ls-tree).
- **5 BLR vtable dispatches enumerated**: 1 inner-vtable + 4 std::string-style with `w2=0x9881, w3=10`. PCs listed in `outputs/lane_x_20260518_191300/bl_blr_enum.json` (or the equivalent).
- **2 BL targets identified**: `0x197ca0 = C++ guard`, `0x197d48 = CFF dispatcher`.
- **3 atomic (chal, Token) wire pairs** from Lane V: `outputs/lane_v2_20260518_180526/atomic_4tuple.json` (or `lane_u_20260518_172430/`).
- **LIVE MAGIC32**: `87BCB74629734B9BAF2D948A4B8823E7` (install-stable, lane_u atomic capture).
- **Lane W F-1 FALSIFIED by Lane X** — SHA-256 IV in permuted stack layout claim was WRONG. 0/8 IV constants found in any aligned 4-byte LE offset of Lane V's captured 3 KB sp dump. The IV state lives elsewhere (rodata? .bss? not on stack).

## Success criteria
- **Done = at least 1 of the 3 captured wire pairs reproduced from pure-Rust given the symbolically-lifted cert body**. Goal closure threshold: 1/3 byte-match.
- Fact `lane_z_symbolic_lift_yields_algorithm_2026_05_18` set with the lifted algorithm description.
- Single commit `lane-z: ...` on branch `magic32-live-snapshot-replay`. Verdict at `analysis/lane_z_verdict.md` (≤120 lines). Final line `LANE_Z_DONE`.

## Concrete tasks (do in order)

1. **Inventory artifacts** — `git ls-tree -r HEAD outputs/lane_x_*` and `outputs/lane_v2_*` to confirm the cert fn binary, BL/BLR enum JSON, and atomic 4-tuple are on disk. If not, dump them from the running thered (PID may have changed — `pgrep -af thered` first).

2. **Choose symbolic engine** — angr (Python, fast iteration) or Triton (C++, deeper bitvector support). For a 466-insn body with 5 BLRs treated as opaque oracles, **angr is the right starting choice** (Python ergonomics + the same `cle.Loader` we've used before). Install if needed: `pip install --user angr` inside the worktree.

3. **Lift to symbolic IR**:
   - `angr.Project('outputs/lane_x_20260518_191300/cert_fn_live.bin', main_opts={'backend':'blob','arch':'aarch64','base_addr':0x197f5c})`.
   - Mark the 5 BLR target slots and 2 BL targets as **opaque oracles** — implement them as Python `SimProcedure`s that read arguments from `state.regs.x0..x7`, then either (a) **replay** the captured args/returns from the Lane X enumeration JSON if it includes return values, OR (b) **assume** the BLR is a memcpy/strlen/std::string op and emulate accordingly (the 4 std::string-style ones with `w2=0x9881, w3=10` strongly suggest std::string::resize / std::vector::reserve).
   - Constrain the input: `state.memory.store(challenge_addr, claripy.BVV(chal_bytes, 64*8))`. MAGIC32 too.
   - Run `simgr.explore(find=lambda s: s.addr == cert_fn_return_pc)`.

4. **Concretize output** — at the return state, read the Token buffer (24 bytes per F-5: 24-byte truncation of 32-byte state) and compare against the captured wire Token.

5. **Iterate**: if no match, the BLR oracle model is wrong. Plug in different oracle shapes:
   - Try `std::string::assign` → memcpy
   - Try `std::vector::push_back` → memcpy + length
   - Try the inner-vtable as `SHA-256-compress(block)` (treat the 1 non-stdlib BLR as a black-box SHA round)
   Each iteration is a single `simgr.run()` + memory comparison.

6. **Multi-pair validation**: once any 1 pair byte-matches, validate against the other 2. If 2/3 match → goal CLOSED.

7. **Commit + verdict.** `lane-z: ...` single commit. Verdict at `analysis/lane_z_verdict.md` (≤120 lines). Final line `LANE_Z_DONE`.

## Falsification criteria

- **State-space explosion** after >2 hours of `simgr.explore()` with no concrete return state → angr can't path-explore through this body (likely due to MBA at the round level). Pivot: Triton with its bit-precise concolic execution, OR direct manual lift to Rust from the disasm (treating BLRs as Rust trait objects with N candidate impls).
- **All 4 std::string oracle models produce 0/3 matches** → the BLRs are NOT what they appear to be (the `w2=0x9881, w3=10` pattern might be encoding something else entirely). Escalate to "user resource gate is the only remaining path" — close as `stalled-meta`.
- **angr crashes / OOMs >32GB** on a 466-insn body → use Triton's interactive `setSymbolicMemory()` and step manually through ~100 insns at a time.

## Constraints & gotchas

- **RAM budget: 32 GB hard cap** for any single solver process. If angr/Triton grows beyond that, kill and bisect.
- **NO live thered probing this lane** — substrate is blocked by SMS gate. All work is offline against captured artifacts.
- **The cert fn is in rwxp-decrypted code per Lane W** — but Lane X already CAPTURED the decrypted bytes to disk. Use the file, not the live mem.
- **Don't conflate Lane V's structural claims with ground truth** — Lane X falsified F-1. F-3 (custom round constants) and F-5 (24-byte trunc) are still uncontested but treat as hypotheses, not facts.

## Relevant files / references

- `outputs/lane_x_20260518_191300/` — live cert fn bytes + BL/BLR enum.
- `outputs/lane_v2_20260518_180526/atomic_4tuple.json` — 3 challenge/Token pairs for validation.
- `cert-rust-repro/src/lane_x.rs` — Lane X's documented model (uncommitted goal-close test).
- `analysis/lane_x_verdict.md` (commit bfd8016) — Lane X full findings.
- `analysis/lane_w_verdict.md` (commit e33a6b6) — Lane W (note F-1 has since been falsified).
- Facts: `lane_x_committed_bfd8016_2026_05_18`, `lane_v_done_2026_05_18`, `magic32_value_extracted_2026_05_18` (the 87BCB7… LIVE value).
