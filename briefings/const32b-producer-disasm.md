# const32b-producer-disasm — turn 2 RESTART (post-crash, focused)

**Process group crashed mid-turn-2.** Previous thread `019e1c4b-...` dead; fresh thread starts here. Turn-1 artifacts intact; do NOT redo them.

## Role & workdir

Codex worker on goal `nmss_const_32b_origin`. Workdir: `/home/sdancer/nmss-emu-const32b-producer`.

## Preserved findings from turn 1 (don't re-derive — these are fact)

PC `0x78c68b2388` (shard `78c6896000.bin`) is inside an INLINE SHA-256 finalizer:

| Slot | Meaning |
|---|---|
| `x10 = sp+0x940` | SHA-256 state |
| `x9 = sp+0x7b0` | 32-byte digest output |
| `x21 = [sp,#520] = sp+0x968` | raw 64-byte input block |
| `sp+0x810` | byte-swapped 16-word msg schedule |
| `0x78c6b3a354` | K[64] table |

Heap message built earlier in the stage:
```
x20 = malloc(w24)              w24 = w19 + 0x20
memcpy(x20, x26, x19)          # prefix bytes from x26, length w19
append(x20+x19, [sp+0x7b0], 32) # prior_digest from sp+0x7b0
```

So the SHA at this PC consumes `prefix(x26, x19) || prior_digest(sp+0x7b0)`. This is the iterated body_i recurrence pattern from cert. The chain bottoms out at a seed (first iteration), which is the CONST_32B preimage origin.

## Turn-2 GOAL (single artifact target)

Produce `/home/sdancer/nmss-emu-const32b-producer/analysis/const32b_input_provenance.md` (≤200 lines) answering:

1. Where is `x26` (prefix bytes ptr) written immediately before `0x78c68ae660`?
2. Where is `x19` (prefix length) set? Is it constant `0x20` or loaded?
3. Where is `sp+0x7b0` written earlier in the same frame? If by another SHA finalizer with the same pattern, this is iterated. Trace the chain to the first iteration's input.
4. Classify the seed: rodata-baked / ctx-field-load / syscall / runtime-derived.

If you reach a primitive (rodata literal, syscall, or known field), set fact `nmss_const_32b_preimage_recovered_8_of_8` and the campaign closes.

## CRITICAL guardrails (cycle 92 lesson — events.jsonl bloated 83 MB without artifact)

- **NEVER disasm a window > 256 insns in a single objdump invocation.** Use 64–128 insn windows.
- **Write artifact incrementally.** First write a 10-line skeleton of `const32b_input_provenance.md` BEFORE any further disasm; append each finding as you make it.
- **No >100KB raw dumps in your tool outputs.** If you need a big disasm, save it to a file and reference paths; do not let it land in your context.
- **Stop after each disasm batch and update the artifact.** Don't accumulate findings only in memory.
- **If a tool call would dump >50KB, redirect to a file first**: `python3 -c "..." > /tmp/dump.txt && wc -l /tmp/dump.txt`.
- **No git commits.**
- **No Frida on libUnreal.so** (anticheat).

## Concrete tasks (ordered, focused)

1. **First write the skeleton artifact** at `/home/sdancer/nmss-emu-const32b-producer/analysis/const32b_input_provenance.md` — just headers, no findings yet. ~10 lines.

2. **Disasm 96 insns above `0x78c68ae660`** (the `add w24, w19, #0x20` line). Look for the `ldr`/`mov` that sets `x26` and `x19`. The window already shown in turn-1 artifact says one path goes via `ldr x26, [x8, #544]; ldr x19, [x8, #536]` (where `x8 = [sp,#504]`), and another via `add x26, x10, #0x1; lsr x19, x8, #1`. Pin which arm is taken at runtime, and identify what `[sp,#504]` is.

3. **Search for other `add x9, sp, #0x7b0` patterns** in the function (bounded `0x78c68ae5c0..0x78c68b2400`). Each one is a hash output write — that's an iteration boundary. Count them; the EARLIEST one's prior `[sp+0x7b0]` is the seed.

4. **Trace seed origin in ≤2 hops.** When you find the seed-write site:
   - `adrp+add` to rodata → CONST_32B preimage is baked.
   - syscall result → primitive identified.
   - struct field load → trace one more hop.

5. Update checkpoint `/home/sdancer/orchestrator/analysis/checkpoints/const32b_producer_progress_2026-05-12.jsonl` with stage `preimage_64b_class_identified` (or `primitive_terminator_found`).

## Tooling cheat-sheet

Shard load base: `0x78c6896000`. To translate a PC to a shard offset: `pc - 0x78c6896000`.

- `aarch64-linux-gnu-objdump -D -b binary -m aarch64 --start-address=N --stop-address=M /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78c6896000.bin > /tmp/dis.txt`
- Or capstone via Python — pin start/end byte offsets within the shard.
- K-table marker: search for `adrp ..., 0x78c6b3a000 ; add ..., #0x354` to identify SHA finalizer body entries.

## Operating mode

`codex_app_server`. Single long turn via `harness send --wait --timeout 1800`. STOP criteria:
- (a) Seed identified → set close fact.
- (b) Chain enters deleted module without disasm → save blocker note, recommend HW-BP capture path.
- (c) Artifact written but seed not yet pinned after 2 hops → save with class-tag `seed_class_partial`.

## References

- Turn-1 artifact: `/home/sdancer/nmss-emu-const32b-producer/analysis/const32b_producer_disasm.md`
- Turn-1 window dump: `/home/sdancer/nmss-emu-const32b-producer/analysis/const32b_producer_window_0x78c68b2388.txt`
- Checkpoint: `/home/sdancer/orchestrator/analysis/checkpoints/const32b_producer_progress_2026-05-12.jsonl`
- Shard: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78c6896000.bin`
- Cert pattern reference: `nmss-emu/WIKI.md` (body_i recurrence section)
