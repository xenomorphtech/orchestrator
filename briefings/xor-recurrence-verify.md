# xor-recurrence-verify — Re-verify the cipher recurrence from the original disasm

## Role & workdir
Static-analysis verification worker. Workdir: `/home/sdancer/dark-december-xor-recurrence-verify`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `verify-or-correct-cycle335-cipher-recurrence`

## Why this turn exists
Five rounds of cryptanalysis (cycles 347-358) have FALSIFIED every interpretation of the cycle-335 chained-XOR algorithm `p[i] = c[i] ^ p[i-1] ^ k[i & 7]`:
- Cycle 347: simple cyclic XOR (no chain) — falsified by 11 distinct (k[0],k[1]) pairs across client frames
- Cycle 349: chained XOR alone — falsified by invariant float slot impossibility
- Cycle 352: chained XOR + packet-id-byte mask — falsified by same impossibility (mask explained the `03`-signature but not the slot-1 floats)
- Cycle 358: above + u32 field re-interpretation — falsified by multi-billion value spreads with no monotonic structure

BUT each falsification preserved positive structural signals:
- Packet IDs forced: 0x0385 (FRzStandBr) and 0x0386 (FRzMoveBr)
- 9-frame c0d2 family identical at bytes 0..9 → same plaintext bytes 0..9 → same handle
- c0d2 vs c3d1 cipher delta `0x03 0x03 0x03 0x03 0x03 0x03 0x03 0x03 0x03 0x03` → mask=`packet_id_low_byte` is structurally required
- `@14` and `@30` upper 24 bits are FIXED per frame under masked-chained — impossibility for any field type

**The cycle-335 algorithm spec must be WRONG.** That spec was synthesized from disasm at `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm` (43 KB on disk). The disasm itself is correct; the textual spec may have mis-described the recurrence (e.g. wrong operand order, missing rotation, missed AND-mask, missed multiply).

## Hypothesis
The actual cipher recurrence in 0x6ce57c1904's disasm differs from `p[i] = c[i] ^ p[i-1] ^ k[i & 7]`. Candidate corrections to test from the disasm:
1. `p[i] = c[i] ^ p[i-1] ^ k[i & 7] ^ pos_offset` (position-dependent constant)
2. `p[i] = c[i] ^ ROR8(p[i-1], n) ^ k[i & 7]` (rotation on feedback)
3. `p[i] = c[i] ^ p[i-N] ^ k[i & M]` (different feedback depth / key length)
4. The function processes BYTES in NON-LINEAR order (e.g. nibble-swapped, every-other, reverse)
5. The function reads MORE than just c[i] per output (multiple cipher bytes per plaintext byte)
6. There's a NON-XOR operation in the loop body (multiply, add, S-box lookup)

## Falsification (3 outcomes)
- (a) **Re-reading the disasm reveals a corrected recurrence that decodes ≥50% of 22 long frames plausibly** → SUCCESS. Fact: `dark_december_wire_xor_recurrence_corrected_<short_hash>_decoded_<n>`.
- (b) **Disasm reads exactly as cycle-335 specified — recurrence is correct, falsification is elsewhere** → the cipher is NOT 0x6ce57c1904 (wrong function identified). Fact: `dark_december_wire_xor_wrong_function`.
- (c) **Disasm shows a more complex recurrence but it still doesn't fit captures** → cipher has a per-packet IV/nonce we can't recover offline. Fact: `dark_december_wire_xor_needs_runtime_state`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-xor-recurrence-verify/analysis/recurrence_verify_2026-05-15.md` with:
1. **Re-read the disasm body** at `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm`. Find the inner loop (between the prologue and the first `ret`).
2. **Trace the EXACT byte-level operations** from cipher byte load to plaintext byte store. Include: register assignments, all ALU operations (eor, orr, and, ror, mul, etc.), memory accesses with offsets.
3. **Write the recurrence in math form**, byte-by-byte, matching what the assembly actually computes.
4. **Compare** to the cycle-335 spec `p[i] = c[i] ^ p[i-1] ^ k[i & 7]`. State each discrepancy.
5. **Apply the corrected recurrence** to the 9 c0d2 server frames (35 bytes each). Compute whether the @14 upper-24-bits constraint is now ESCAPABLE (i.e. the upper 24 bits depend on h0..h5 not just k[0..1]).
6. **If recurrence is corrected, attempt decode** of at least 3 frames and verify against the move-decode field layout.
7. **Look for sibling cipher functions** — grep the disasm for jumps/calls/branches that exit to other code (might be a sibling cipher).
8. Verdict matched to (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `XOR_RECURRENCE_VERIFY_DONE` on the final line.

## Execution flow

**Step 1 — Read the disasm:**
```bash
wc -l /home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm
# Find function entry/exit
head -100 /home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm
grep -n -m 5 ' ret$\|stp\|ldp' /home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm
```

**Step 2 — Locate the inner loop:**
```bash
# A byte-level XOR loop typically has: ldrb (load byte), eor (xor), strb (store), subs+b.ne or cbnz
grep -n 'ldrb\|strb\|eor\b' /home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm | head -50
grep -n 'b.ne\|cbnz\|cbz' /home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm | head -20
```

**Step 3 — Annotate the loop body line-by-line.** For each instruction, name what register holds what semantic value (cipher byte, plaintext byte, key byte, position counter, accumulator). 

**Step 4 — Write the recurrence in math form.** Be specific:
- `cipher_byte = MEM[buf+i]` (which buffer? what offset?)
- `prev_plain = X[?]` (which register? when was it loaded?)
- `key_byte = X[?]` (which register? was it from rodata or stack or argument?)
- `result = cipher_byte ^ prev_plain ^ key_byte ^ ???` (any extra operations?)
- Where does the result get stored?

**Step 5 — Re-decode c0d2 frame 35.** With the corrected recurrence and h0..h5 as free variables, compute the symbolic plaintext for bytes 14-17. If the upper 24 bits now depend on h0..h5, the slot CAN be plausible for SOME choice — that's a green flag.

**Step 6 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.**
- **NO new disasm of the binary** — the existing 43 KB xor.disasm is enough.
- **NO pyelftools. NO Frida. NO live device. NO MCP aeon.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time** (more than usual — careful disasm reading is the value-add here).
- Prior worker artifacts to cross-reference:
  - `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md` — cycle-335 spec being verified
  - `/home/sdancer/dark-december-xor-field-interp/analysis/field_interp_2026-05-15.md` — cycle-358 demonstration of "upper 24 bits fixed per frame" constraint

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-xor-recurrence-verify/`
- **PRIMARY INPUT**: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor.disasm` (43 KB, disasm of function at 0x6ce57c1904)
- Cycle-335 spec to verify: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Frame corpus (for decode test): `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`
- Move/Stand layout: `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`
- success-fact key: `dark_december_wire_xor_recurrence_corrected_<sha>_decoded_<n>` (a)
- block-fact keys: `dark_december_wire_xor_wrong_function` (b), `dark_december_wire_xor_needs_runtime_state` (c)
