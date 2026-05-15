# magic32-origin-static — chase the 2 remaining caller branches to the MAGIC32 producer

**You ARE allowed and expected to write code.** Python+capstone, shell, small Rust if needed.

## Role & workdir

Static-RE worker, final hop. **Workdir**: `/home/sdancer/nmss-emu-magic32-origin-static/`.

## Current goal / sub-goal

- `goal_key` = **`nmss_magic32_origin`** — full producer chain to primitive input.
- `sub_goal_key` = `magic32_remaining_branches` — chase the 2 remaining live caller branches of `0x78c69caf0c`.

## Success criteria

- **Campaign close (Δmetric → 16):** Identify a primitive input (syscall/JNI/file/network) that produces the MAGIC32 token. Set fact `nmss_magic32_origin_recovered_16_of_16` + escalate as **CAMPAIGN COMPLETE**.
- **Partial (Δmetric +0.5 each):** Per branch — narrow it to a specific upstream PC + classify how x1 is materialized.

## Progress so far (turns 1–5 complete; metric 15/16)

Chain pinned 4 levels deep:
```
device_info+0x210 std::string (MAGIC32 backing at heap 0x79e50eb290)
  ← publisher 0x78c69aa16c (jump-table parser, 179 entries)
  ← varargs wrapper 0x78c69aebec (builds va_list cursor at x29-0x40)
  ← serializer 0x78c69e4cd0 (writes DESTINATION row at slab-embedded x1+0x210)
  ← slab builder 0x78c69e4b88 (allocates 0x488 slab, forwards arg1 as serializer x3)
  ← outer owner-builder 0x78c69caf0c (7 direct callers indexed)
```

7-caller census of `0x78c69caf0c`:
- `0x78c69af4f0`: x1 = x22 (register-carried)
- `0x78c69afff0`: x1 = [sp] (stack-spilled)
- `0x78c69bfa58`: x1 = x24 (register-carried)
- `0x78c69c6ff8`: x1 = `[x21,#0xf0]` or `[x21,#0x130]` (ctx-field)
- `0x78c69c70ec`: x1 = result of `blr x8` at 0x78c69c70cc, cached to `[x21,#0x558]` (indirect-call result)
- `0x78c69c81c4`: **FALSIFIED** (cycle 40) — x1 = sp+0x158 textual socket IP via `getsockname → inet_ntop(0x78c67e1b70=PLT@inet_ntop)`. Not MAGIC32.
- `0x78c69cb30c`: tail-branch wrapper

**Artifacts (read these first):**
- `/home/sdancer/nmss-emu-magic32-origin-static/analysis/parser_0x78c67e1b70.md` (turn 5 — inet_ntop classification + falsified-branch reasoning)
- `/home/sdancer/nmss-emu-magic32-origin-static/analysis/magic32_producer_chain.md` (turn 5 updated chain + falsification)
- `/home/sdancer/nmss-emu-magic32-origin-static/analysis/serializer_block_parent.md`
- `/home/sdancer/nmss-emu-magic32-origin-static/analysis/serializer_block_callers.jsonl`
- `/home/sdancer/nmss-emu-magic32-origin-static/analysis/publisher_caller.md`
- `/home/sdancer/nmss-emu-magic32-origin-static/analysis/writer_pc_family_disasm.md`
- `/home/sdancer/nmss-emu-magic32-origin-static/analysis/device_info_base.md`

## Next 2–3 concrete tasks (ordered)

1. **Chase `0x78c69c70ec` (indirect-call result branch).**
   - Disassemble 64 insns around `0x78c69c70cc` (where `blr x8` materializes x1).
   - Identify how `x8` was set up — usually `adrp+ldr+add` to a vtable slot, or `ldr x8, [x21, #imm]`.
   - If vtable-based: scan `78cd296000.bin` (cert-campaign vtable region) for the 8-byte LE image of the vtable slot's target. The target function is the actual producer of x1.
   - If `[x21, #imm]`-based: the x21 owner caches the producer fn-ptr at `#imm`. Find what writes to that slot.
   - Trace the producer fn's body: does it call `recv@plt` / `read@plt` / `JNI_*` / `getauxval` / `__system_property_get`? If so → primitive identified. Save to `analysis/branch_0x78c69c70ec.md`.

2. **Chase `0x78c69c6ff8` (ctx-field branch).**
   - x1 comes from `[x21, #0xf0]` (or `#0x130` on a flag path). Disasm 64 insns around `0x78c69c6ff8` to confirm exact offset and flag-condition.
   - Search both shards `78c6896000.bin` and `78cd296000.bin` for `str ..., [x?, #0xf0]` (where the str source-register's value lives at MAGIC32 length 0x20 ASCII OR is a pointer chain leading to it).
   - The producer of `[x21, #0xf0]` is the actual MAGIC32 setter for ctx. Document this. Save to `analysis/branch_0x78c69c6ff8.md`.

3. **Synthesize.** Whichever branch terminates at a primitive (syscall/JNI/file/property) — that's the MAGIC32 origin. Write final `analysis/magic32_producer_chain.md` with the full chain. Set fact `nmss_magic32_origin_recovered_16_of_16` + escalate.

## Constraints & gotchas

- **No git commits.**
- **No Frida on libUnreal.so** (user memory).
- `0x78c69c81c4` is **FALSIFIED**, don't re-investigate. Same goes for the `inet_ntop / getsockname` chain.
- If both remaining branches dead-end at indirect calls reaching deleted blobs → write `analysis/magic32_origin_blocker.md` recommending `magic32-netmarble-login-replay` (network HTTPS-handshake capture) or HW-watchpoint pivot.
- Cycle-21 overclaim about "serializer x1 = source_device_info" is corrected. Don't reintroduce.

## Relevant files / references

- **Memdumps**: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/{78c678d000.bin, 78c6896000.bin, 79e50b7000.bin, 7ac5043000.bin, 78cd296000.bin}`
- **Cross-pollination facts**: `nmss_magic32_producer_chain_4_deep_2026_05_12`, `nmss_magic32_parser_is_inet_ntop_2026_05_12`, `nmss_magic32_branch_0x78c69c81c4_falsified_2026_05_12`

## Progress log

Append to `/home/sdancer/orchestrator/analysis/checkpoints/magic32_origin_static_progress_2026-05-12.jsonl`. Stages: `branch_0x78c69c70ec_classified`, `branch_0x78c69c6ff8_classified`, `magic32_producer_primitive_pinned`.

## Operating mode

Codex `codex_app_server` (no pane), long-running turn via `harness send --wait --timeout 10800` in persistent nohup CLI. STOP on:

- (a) Primitive input identified → **CAMPAIGN CLOSE**, fact `nmss_magic32_origin_recovered_16_of_16`.
- (b) Both branches dead-end → `analysis/magic32_origin_blocker.md`; recommend `magic32-netmarble-login-replay`.
- (c) 2 cycles with no new ground → flag falsification of the whole `magic32-origin-static` path.
