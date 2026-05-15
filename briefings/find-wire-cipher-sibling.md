# find-wire-cipher-sibling — Locate the actual wire cipher function via signature scan

## Role & workdir
Targeted static-disasm worker. Workdir: `/home/sdancer/dark-december-find-wire-cipher-sibling`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `locate-true-wire-cipher-via-chained-xor-signature`

## Why this turn exists
Cycle 372 EMPIRICALLY confirmed (via 150-sec HW BP capture, 125/125 attach, 0 samples, no AC reaction with game in foreground) that `0x6cde42c904` is NOT the wire cipher used during ingame gameplay — even heartbeats (which cycle-322 pcap shows exist) didn't trigger it. There MUST be a sibling cipher function in libUE4. We need to find it.

The HW BP capture infrastructure is durable: `perf_event_open(PERF_TYPE_BREAKPOINT, HW_BREAKPOINT_X)` works, attaches to all 125 PID 28722 threads, no AC reaction. Once we find the right address, the capture path is one-shot.

## Hypothesis
The wire cipher used during gameplay is a function with the SAME chained-XOR signature as `0x6ce57c1904` (now `0x6cde42c904`) but located elsewhere in libUE4's .text. Either: (a) sibling of 0x6cde42c904 in same vtable, (b) a separate cipher class for ingame vs lobby, (c) a JIT-generated cipher (less likely).

Specifically: search for the inner-loop signature:
```
ldrb wA, [xB, xC]       ; load cipher byte
ldrb wD, [xE, xF]       ; load key byte
eor  wG, wA, wH         ; cipher_byte ^ prev_plain
eor  wG, wG, wD         ; ^ key_byte
strb wG, [xB, xC]       ; store plaintext byte
```

Plus the post-loop state-advance pattern:
```
ldr xN, [x0, #0xb8]     ; load state (or similar)
add xN, xN, xP          ; advance
str xN, [x0, #0xb8]     ; store back
```

## Falsification (3 outcomes)
- (a) **Find ≥1 sibling chained-XOR function with same recurrence + state-advance pattern at different VA** → SUCCESS. Update HW BP to that VA, document for next capture. Fact: `dark_december_wire_cipher_sibling_found_<short_va>`.
- (b) **No sibling found via signature scan** → cipher is genuinely different (not XOR, or different recurrence). Fact: `dark_december_wire_cipher_no_sibling_in_libue4`. Next: kprobe sendto syscall path.
- (c) **Find multiple candidates but can't disambiguate which is the gameplay cipher** → Set HW BPs on ALL candidates simultaneously, document for next gameplay capture. Fact: `dark_december_wire_cipher_multi_candidate_<count>`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-find-wire-cipher-sibling/analysis/sibling_search_2026-05-15.md` with:
1. **Capstone scan setup**: load .text shard in 4 MB CHUNKS (NOT full shard), disasm each chunk, scan for the 5-instruction signature pattern, drop chunk after scan. HARD MEMORY CAP: 1.5 GB total worker RSS. Trigger systemctl-stop at 2 GB.
2. **Signature definition**: precise instruction-encoding pattern for the chained-XOR loop. Use cycle-361 disasm at `/home/sdancer/dark-december-xor-recurrence-verify/analysis/recurrence_verify_2026-05-15.md` (the reference function bounds: 0x6ce57c1904..0x6ce57c19b8) for ground truth.
3. **Scan output**: list of candidate function VAs (image-relative), each with:
   - 4-instruction signature window match
   - Estimated function bounds (entry + first ret)
   - Whether it has the `[x?+0xb8]` state-advance signature post-loop
4. **Top 3 candidates ranked** by how closely they match the full 0x6ce57c1904 pattern.
5. **For top candidate**: derive its LIVE VA using the cycle-372 ASLR delta `0x7395000` (or recompute from current `/proc/28722/maps`).
6. Verdict matched to (a)/(b)/(c). Set the closing fact via `harness fact-set`.

Print `FIND_WIRE_CIPHER_SIBLING_DONE` on the final line.

## Execution flow

**Step 1 — Reference signature extraction:**
```bash
sed -n '40,100p' /home/sdancer/dark-december-xor-recurrence-verify/analysis/recurrence_verify_2026-05-15.md
# Identify the exact 5-instruction signature at 0x6ce57c1968-0x6ce57c1984
```

**Step 2 — Streaming capstone scan:**
```python
import capstone
md = capstone.Cs(capstone.CS_ARCH_ARM64, capstone.CS_MODE_LITTLE_ENDIAN)
SHARD = '/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin'
SHARD_BASE = 0x6ce4bd4000
CHUNK = 4 * 1024 * 1024   # 4 MB
import os
size = os.path.getsize(SHARD)
hits = []
with open(SHARD, 'rb') as f:
    for offset in range(0, size, CHUNK):
        chunk_size = min(CHUNK, size - offset)
        f.seek(offset)
        data = f.read(chunk_size)
        # Sliding window over instruction stream
        recent = []
        for insn in md.disasm(data, SHARD_BASE + offset):
            recent.append((insn.address, insn.mnemonic, insn.op_str))
            if len(recent) > 8:
                recent.pop(0)
            # Pattern match: 5 instructions matching the signature
            if len(recent) >= 5 and matches_signature(recent[-5:]):
                hits.append(recent[-5:])
        # Discard `data` and recent before next chunk to avoid memory growth
```

**Step 3 — Signature matcher:**
```python
def matches_signature(window):
    # window is list of 5 (address, mnemonic, op_str) tuples
    # Look for: ldrb ... ldrb ... eor ... eor ... strb
    mnemos = [w[1] for w in window]
    if mnemos != ['ldrb', 'ldrb', 'eor', 'eor', 'strb']:
        return False
    # Verify operand patterns (registers similar to reference)
    # ...
    return True
```

**Step 4 — For each candidate, find the function ENTRY** by walking backwards for prologue (stp, mov xN, sp). Then find the EXIT (`ret`) walking forward.

**Step 5 — Check state-advance pattern** within the function bounds. Look for `ldr xN, [x0, #0xb8]` or similar offset, paired with `add` and `str`.

**Step 6 — Rank candidates** by signature match strength + state-pattern presence + function size similarity to reference (~180 bytes).

**Step 7 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 1.5 GB worker RSS. systemctl-stop at 2 GB.**
- **Streaming chunks (4 MB), NOT full-shard load.** This is the SAME failure mode as cycle 209 and cycle 363 — bulk disasm of 97 MB blows RAM. Streaming + discarding is mandatory.
- **NO pyelftools full-load.**
- **NO new memdump. NO Frida. NO live device interaction.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time.**
- Honor `[[bulk-enumeration-needs-explicit-memory-budget]]` — declared 1.5 GB cap, explicit chunked scan.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-find-wire-cipher-sibling/`
- **.text shard** (97 MB): `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (base VA `0x6ce4bd4000`, image base `0x6cdd243000`)
- Reference function 0x6ce57c1904 disasm: `/home/sdancer/dark-december-xor-recurrence-verify/analysis/recurrence_verify_2026-05-15.md`
- Empirical kuprobe artifacts (preserved infra): `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/`
- ASLR delta from cycle 372: `0x7395000` (stale VA 0x6ce57c1904 → live VA 0x6cde42c904)
- Frame corpus (for eventual decode): `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`
- success-fact key: `dark_december_wire_cipher_sibling_found_<short_va>` (a)
- block-fact keys: `dark_december_wire_cipher_no_sibling_in_libue4` (b), `dark_december_wire_cipher_multi_candidate_<count>` (c)
