# libue4-xor-synthesize — Synthesize XOR algorithm closure from existing disasm

## Role & workdir
Static-analysis closure-only worker. Workdir: `/home/sdancer/dark-december-libue4-xor-disasm`.

## Current goal / sub-goal
- **goal_key**: `dark_december_rz_xor_obfuscation_decoded` (continuing — prior worker died at memory cap)
- **sub_goal_key**: `xor-synthesize-from-existing-disasm`

## Why this turn exists — RECOVERY
Prior worker `libue4-xor-disasm` (cycle 332) extracted the disasm (`analysis/xor.disasm`, 43 KB) but hit MemoryHigh=20G (peak 21.1 GB, 0B available, 3.7 GB swap) and was systemctl-stopped before it could write the closure markdown. The disasm IS on disk — your job is to **synthesize the algorithm spec from the existing file**.

**Do NOT load any large binary. Do NOT run pyelftools. Do NOT disasm again.** The .disasm file is enough.

## Hypothesis
The function at `0x6ce57c1904` is a self-contained byte-level obfuscation primitive (XOR / rolling XOR / affine). Reading the existing `analysis/xor.disasm` lets you describe the algorithm, key derivation, and any rodata-key lookups.

## Falsification (3 clean outcomes)
- (a) **Algorithm recoverable from disasm**: write the pseudocode + key/table refs → SUCCESS. Fact: `dark_december_rz_xor_algorithm_decoded_<short_sha>`.
- (b) **Key table refs found but values past the captured shards** → document, mark key-blocked. Fact: `dark_december_rz_xor_algorithm_known_key_blocked`.
- (c) **Function turns out to be NOT a XOR helper** → falsified anchor; explain what it actually does. Fact: `dark_december_rz_xor_anchor_misidentified`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md` with:
1. Function bounds (find prologue + first ret in `analysis/xor.disasm`).
2. **Pseudocode** in Python-like form, ≤100 lines, that a human can paste-and-run to deobfuscate captured bytes.
3. **Key/table location** if `adrp+add` pairs reference rodata. Compute target VA and look up via `dd` on the rodata shard (single 64-byte read, NOT a full-shard load).
4. **Position/state semantics**: stateless? counter-keyed? buffer-position keyed?
5. **Cross-check** against the prior pcap-side affine layer markers `0x8a/0x8b` (client) and `0x11` (server) — look for those constants in the disasm or referenced rodata.
6. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c).

Print `XOR_SYNTHESIZE_DONE` on the final line.

## Execution flow — atomic, single Codex turn — VERY SHORT

**Step 1 — Read disasm:**
```bash
wc -l analysis/xor.disasm
sed -n '1,200p' analysis/xor.disasm
```

**Step 2 — Find function end + key pattern:**
```bash
# First "ret" location (function exit)
grep -n -m 5 ' ret$' analysis/xor.disasm
# adrp+add pairs (rodata refs)
grep -n -A1 -E 'adrp\s+x[0-9]+' analysis/xor.disasm | head -40
# Inner loop signature
grep -n -E '(eor|mul|madd).*w[0-9]+' analysis/xor.disasm | head -20
# Loop control
grep -n -E '(subs|cmp).*w[0-9]+' analysis/xor.disasm | head -10
```

**Step 3 — Resolve any adrp+add target.**
For each pair:
- `adrp xN, #IMM` (PC-relative, page-aligned)
- `add xN, xN, #IMM2`
- target_va = (PC & ~0xfff) + IMM + IMM2
- File offset in rodata shard = target_va - 0x6cdd243000

```bash
# Read 64 bytes at target offset from rodata
RODATA=/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin
OFF=$((<target_va> - 0x6cdd243000))
dd if=$RODATA bs=1 skip=$OFF count=64 2>/dev/null | xxd
```

**Step 4 — Translate to Python pseudocode.**

**Step 5 — Cross-check against 0x8a/0x8b/0x11 constants — grep raw disasm for `0x8a`, `0x8b`, `0x11` immediates.**

**Step 6 — Write markdown + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** READ files with grep/sed/head/tail; do NOT slurp into Python. Do NOT load any ELF, do NOT pyelftools.
- **HARD output cap**: ≤300 lines disasm in artifact, ≤100 lines pseudocode.
- **NO new disasm work** — the existing `analysis/xor.disasm` is your only source.
- **NO MCP aeon calls** (memory blew prior session via this).
- **One Codex turn budget: ≤15 minutes wall time.** This is a synthesis-only job.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-xor-disasm/`
- **PRIMARY INPUT** (existing): `analysis/xor.disasm` (43 KB, disasm of function at 0x6ce57c1904)
- Target function VA: `0x6ce57c1904`
- Rodata shard (read SPECIFIC offsets only, never load): `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin` base VA `0x6cdd243000`
- Dynsym dump (grep-only): `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_symbols.txt`
- success-fact key: `dark_december_rz_xor_algorithm_decoded_<sha>` (a)
- block-fact keys: `dark_december_rz_xor_algorithm_known_key_blocked` (b), `dark_december_rz_xor_anchor_misidentified` (c)
