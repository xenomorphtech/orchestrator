# libue4-xor-disasm — Recover the RzPktSystem rolling-XOR obfuscation algorithm

## Role & workdir
Static-disasm worker. Workdir: `/home/sdancer/dark-december-libue4-xor-disasm`.

## Current goal / sub-goal
- **goal_key**: `dark_december_rz_xor_obfuscation_decoded` (new)
- **sub_goal_key**: `xor-helper-algorithm-recovery`

## Why this turn exists
Prior path `libue4-pktsystem-disasm` (cycle 330) closed at outcome (b) and identified the **rolling XOR helper at `0x6ce57c1904`** as the obfuscation layer between IRzBuffer's plaintext output and the actual socket send. With:
- 8-byte RzPktSystem packet header (recovered)
- IRzBuffer wire format (positional fixed-width LE bytes; recovered in `rz_protocol_2026-05-15.md`)
- Per-message Serialize/Deserialize pairs (recovered for FRz{StatInfo, DamageInfo, DamageCalcInfo, DamageAttributeCalcInfo, ServerInfo} + CL_GS::FRzAppGuardRq + GS_CL::FRzAppGuardRp)
- **THE XOR ALGORITHM** (this path's deliverable)

…we can fully decode any captured frame offline. This is the final missing piece for static-side ingame protocol decoding.

Cross-reference: prior pcap-side campaign (`dark_december_protocol_dump_extended_to_minimap_2026_05_14`) identified an "outer affine layer" with bootstrap variant (0x8a/0x8b client, 0x11 server). This path's recovered XOR may be the **producer side** of that exact same affine layer — comparing the two will validate both.

## Hypothesis
The function at `0x6ce57c1904` is a self-contained byte-XOR primitive (likely ≤256 bytes of code) that takes a (buffer, length, state-or-key) and either:
(a) XORs the buffer in place against a key derived from a counter / position / fixed table (rolling XOR), OR
(b) Applies an affine transform: `out[i] = mul[i % cycle] * in[i] + add[i % cycle]` (mod 256), which is what the pcap campaign called the "affine layer".

Falsification (3 outcomes):
- (a) **Algorithm fully recovered**: pseudocode + key/table contents + position-state semantics → SUCCESS. Fact: `dark_december_rz_xor_algorithm_decoded_<sha_of_summary>`.
- (b) **Algorithm recovered but key/table content lives in the 0x22310-byte-past-shard region** (same wall as pktsystem path) → document mechanism, mark key-recovery-blocked-on-data. Fact: `dark_december_rz_xor_algorithm_known_key_blocked`.
- (c) **Function at 0x6ce57c1904 is NOT a pure XOR helper** — e.g. it's just a vtable dispatcher to something else → falsified anchor; document what it actually is. Fact: `dark_december_rz_xor_anchor_misidentified`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md` with:
1. Function bounds (start VA, end VA, size).
2. Full disasm (cap ≤300 lines — function should be small).
3. **Pseudocode** of the algorithm in Python or C-like form, decodable by a human.
4. **Key/table contents** if they live in rodata (offset + bytes).
5. **State machine** description: is it stateless, position-keyed, or carries state across calls?
6. **Cross-check** against the prior pcap-side affine-layer findings (the `0x8a/0x8b/0x11` markers) — match or distinct?
7. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c).

Print `XOR_DISASM_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Compute file offset, dump function bytes.**
```bash
mkdir -p analysis
SHARD=/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin
# Function VA: 0x6ce57c1904 ; .text shard base 0x6ce4bd4000
# File offset = 0x6ce57c1904 - 0x6ce4bd4000 = 0xbed904
dd if=$SHARD bs=1 skip=$((0xbed904)) count=4096 2>/dev/null > analysis/xor.bin
ls -lah analysis/xor.bin
```

**Step 2 — Disasm.**
```bash
aarch64-linux-gnu-objdump -D -b binary -m aarch64 --adjust-vma=0x6ce57c1904 analysis/xor.bin > analysis/xor.disasm
wc -l analysis/xor.disasm
```

**Step 3 — Locate function bounds.**
First `stp x29, x30, ...` is entry; first `ret` past a properly balanced prologue is exit. Cap at 1024 bytes. If function is larger, sample first 512 bytes and document continuation.

**Step 4 — Identify pattern.**
Common AArch64 XOR/affine patterns:
- **Simple XOR loop**: `ldrb wA,[src],#1; eor wA,wA,wKey; strb wA,[dst],#1; subs wN,wN,#1; b.ne loop`.
- **Position-XOR with key table**: load key table via `adrp+add`, index by counter & 0xF (or similar mask), XOR.
- **Affine `mul*x + add`**: contains `mul` or `madd` instructions in the inner loop (rarer for byte cipher).
- **LCG-keyed XOR**: contains `mul wN,wN,#<prime>; add wN,wN,#<offset>` updating internal state, XOR with byte.

**Step 5 — Resolve key/table location.**
For every `adrp xN, #imm; add xN, xN, #imm2`: target VA = (PC & ~0xfff) + imm + imm2. Check if target ∈ rodata range [0x6cdd243000, 0x6ce4bd4000). Dump 64 bytes via `dd` from the rodata shard. If table is in data shard [0x6ceac63000, 0x6ceba18000), dump from there. If beyond 0x6ceba18000 → outcome (b).

**Step 6 — Translate to Python pseudocode.**
The output of this step is the algorithm spec, reusable to decode captured frames.

**Step 7 — Cross-check against pcap-side affine.**
The prior `dark_december_bootstrap_decoded` fact mentioned the affine layer markers `0x8a/0x8b` (client) and `0x11` (server). Look for those constants in the rodata key table — if present, the static XOR IS the pcap affine layer. If absent, they're different layers.

**Step 8 — Fact-set + print DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set <fact_key> "<summary>"
echo XOR_DISASM_DONE
```

## Constraints & gotchas
- **HARD memory budget: 1 GB.** Single 4 KB function, trivial — but stay disciplined. DO NOT load reconstructed ELF into pyelftools. If you need symbol lookups, grep the existing dynsym dump at `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_symbols.txt`.
- **HARD output cap**: ≤300 lines disasm in artifact + ≤200 lines pseudocode.
- **No Frida / no device interaction.** Static disasm only.
- **No call-graph expansion beyond 1 hop.** Only resolve direct `bl` targets. If the XOR helper calls helpers, look up symbols by VA — don't disassemble those callees in this path.
- **VA resolution rule**: always use dynsym for function entries (carried forward).
- **One Codex turn budget: ≤45 minutes wall time.** This is a SMALL job; don't drag.
- Honor `[[bulk-enumeration-needs-explicit-memory-budget]]`.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-xor-disasm/`
- .text shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (97 MB, base VA 0x6ce4bd4000)
- Rodata shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin` (122 MB, base VA 0x6cdd243000)
- Data shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ceac63000.bin` (14 MB, base VA 0x6ceac63000)
- Dynsym dump (for symbol VA lookups): `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_symbols.txt`
- Prior pktsystem closure (read first): `/home/sdancer/dark-december-libue4-pktsystem-disasm/analysis/pktsystem_framing_2026-05-15.md`
- Prior protocol closure (for IRzBuffer model): `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md`
- Prior pcap-side affine layer: `dark_december_bootstrap_decoded` fact (referencing 0x8a/0x8b/0x11 markers)
- Target VA: **`0x6ce57c1904`** (the XOR helper)
- success-fact key: `dark_december_rz_xor_algorithm_decoded_<sha>` (a)
- block-fact keys: `dark_december_rz_xor_algorithm_known_key_blocked` (b), `dark_december_rz_xor_anchor_misidentified` (c)
