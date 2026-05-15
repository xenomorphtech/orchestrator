# xor-chained-state — Recover per-stream initial state for the chained XOR cipher

## Role & workdir
Pure-Python cryptanalysis worker. Workdir: `/home/sdancer/dark-december-xor-chained-state`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `solve-chained-xor-initial-state-from-known-plaintext`

## Why this turn exists
- Cycle 346 `move-synthesize`: recovered exact FRzMove/Stand wire layout (37/33 bytes, packet IDs `0x0385`/`0x0386`). Outcome (b): decode implausible.
- Cycle 347 `xor-key-recover`: tested simple cyclic XOR `p[i]=c[i]^k[i mod N]` for N∈{2,4,8,16}. **FALSIFIED**: packet-id known-plaintext yields no common (k[0],k[1]) pair across client frames. Outcome (c): `dark_december_wire_xor_not_simple_cyclic`.
- **But the same artifact (`xor_key_recover_2026-05-15.md` line 91) noted that the server families `c0d2`/`c3d1` differ by exactly `0x03 0x03`, matching the chained recurrence's propagation pattern: `c[1] = p[1] ^ p[0] ^ k[1]` — when `p[0]` changes by 3, `c[1]` also changes by 3.** This is strong evidence that the cycle-335 chained algorithm IS the correct cipher, and what's missing is just the initial state.

## Hypothesis
The cycle-335 chained algorithm is correct:
```
p[0] = c[0] ^ k[0]
p[i] = c[i] ^ p[i-1] ^ k[i & 7]   for i = 1..N-1
```
where `k[0..7]` is an 8-byte per-stream key. The **unknown** is `k[0..7]` (or equivalently the 8-byte state at `this+0xb8` of the connection's IRzBuffer instance at the moment each frame starts encoding).

The 9 identical-prefix server frames (`c0d2 ... ` constant through byte 9) are repeated broadcasts of the same FRzStandBr about the same actor — proving the cipher state is the same for those 9 frames. So those 9 frames share a single k[0..7] and known plaintext bytes [0..1] = `0x85 0x03` (FRzStandBr packet ID 0x0385, little-endian).

## Falsification (3 outcomes)
- (a) **Recovered k[0..7] decodes ≥80% of long frames to plausible plaintext** (msg_type in {0x0385, 0x0386}, actor_handle non-degenerate, coord floats in [-1e6, 1e6]) → SUCCESS. Fact: `dark_december_wire_xor_state_recovered_<k_hex>_decoded_<n>`.
- (b) **k[0..1] solvable but later bytes have multiple consistent solutions** → state depends on more than packet-id known-plaintext. Document partial recovery. Fact: `dark_december_wire_xor_state_partial`.
- (c) **Recovered k[0..7] decodes <50%** → another cipher dimension is missing (e.g. nonce, per-packet state advance with non-zero stride). Document the gap. Fact: `dark_december_wire_xor_state_insufficient`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-xor-chained-state/analysis/chained_state_recover_2026-05-15.md` with:
1. **Derived k[0..1]** from the 9-frame server prefix: `k[0] = 0xc0 XOR 0x85 = 0x45`. `k[1] = c[1] XOR p[0] XOR p[1] = 0xd2 XOR 0x85 XOR 0x03 = 0x54`. State this explicitly with byte-by-byte arithmetic.
2. **Derive k[2..7] candidates** under two assumptions:
   - (i) actor_or_move_handle at bytes 2..9 is a typical Unreal u64 ID with high bytes zero or small (`0x0000_xxxx_xxxx_xxxx`)
   - (ii) repeating bytes in actor handle (top-byte=top-byte common in entity IDs)
   For each candidate k[2..7], decode the rest of one 9-frame-family frame and check field plausibility.
3. **Cross-validate**: with the candidate k[0..7], decode the FRzMoveBr (c3d1 family, packet ID 0x0386) frames. They should yield the SAME actor_or_move_handle (same player issuing both Stand and Move) — that's the consistency check.
4. **Decode all 22 long frames**: full plaintext + field interpretation (msg_type, actor_handle, 6 coord/param floats, flag).
5. **Sanity table**:
   | frame | direction | msg_type | actor_handle | coord0..5 (f32) | flag |
   should show same actor for c0d2/c3d1 server frames, plausible float coords (not e+38 or e-30).
6. Verdict (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c). Set via `harness fact-set`.

Print `XOR_CHAINED_STATE_DONE` on the final line.

## Execution flow — atomic single Codex turn (≤20 min wall time, 500 MB)

**Step 1** — Load `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`. Split client_39 vs server_35. Group server by cipher[0:2] family.

**Step 2** — Derive k[0], k[1] from known plaintext (server family `c0d2` → packet ID 0x0385 → p[0]=0x85, p[1]=0x03):
```python
k0 = 0xc0 ^ 0x85   # 0x45
k1 = 0xd2 ^ 0x85 ^ 0x03   # 0x54
```

**Step 3** — Set up the recurrence solver. For each candidate (k[2], k[3], ..., k[7]) — 6 unknown bytes, 2^48 search space, too large for brute force. Instead:
- Use the **actor_or_move_handle** at bytes 2..9 (8 bytes). UE4 entity IDs typically have low ~24-32 bits used and high bytes zero. Try the most common pattern: `handle & 0xFFFF_FFFF_FFFF_FFFF` with `handle >> 56 == 0`.
- Decode byte 2: `p[2] = c[2] ^ p[1] ^ k[2]`. For p[2] to be the LSB of a UE4 u64 handle (any value 0-255), k[2] = c[2] ^ p[1] ^ p[2] for any guess of p[2]. So k[2] is not constrained by this alone.
- **Use cross-frame consistency on the 9 identical-prefix frames** + the c3d1 family (which must have the same handle for a different message type from the same actor). Specifically:
  - Frame A (c0d2, FRzStandBr): bytes 0..9 = packet_id_le(2B) + handle(8B). cipher = `c0 d2 ...`
  - Frame B (c3d1, FRzMoveBr): bytes 0..9 = packet_id_le(2B) + handle(8B). cipher = `c3 d1 ...`
  - Same handle, different packet ID (`0x0385` vs `0x0386`). The byte-2 plaintext is the same; the byte-2 ciphertext differs by p[1]_delta (since `c[2] = p[2] ^ p[1] ^ k[2]`, and only p[1] differs between the frames). p[1] is identical (`0x03` in both LE encodings)! So c[2] should be IDENTICAL between c0d2 and c3d1 families. **CHECK this prediction** — if true, strong evidence for the model.

**Step 4** — Use the c3d1↔c0d2 byte-by-byte comparison to constrain k further. If the prediction in Step 3 holds, then any cipher-byte-difference between the two families at position i must equal the plaintext-byte-difference (key cancels). For bytes 0..1 the difference is `0x03 0x03` (packet ID delta). For bytes 2..9 (the shared handle), there should be ZERO difference. For bytes 10+ (different message body), differences are unconstrained.

**Step 5** — Solve k[2..7] using a smarter approach:
- Decode all 9 c0d2 frames with k[0..1]=0x45,0x54 and a CANDIDATE k[2..7]. For each candidate, compute the resulting actor_handle u64. If the candidate produces the SAME handle across all 9 frames (trivially true since cipher is the same) AND produces a sensible u64, accept.
- 6 unknown bytes = 2^48. Too big. BUT we can constrain:
  - Bytes 26..33 in client_39 frames are the actor_handle at position 6..14 (per move-decode layout offset 0x06 size 8). Use those.
  - Actually the simplest path is to GUESS the high byte of the handle is zero. Then p[9] = 0, so k[7] = c[9] ^ p[8] ^ 0. We need p[8] = c[8] ^ p[7] ^ k[6]. Cascading: assume top bytes of handle are zero (p[9]=p[8]=0), then we can solve k[6], k[7] under those assumptions.
  - For the lower 6 bytes of the handle (bytes 2..7), they're truly unknown. BUT: same-direction frames of DIFFERENT actors give us cross-frame plaintext differences = cross-frame cipher differences (key cancels). If two server frames differ at byte 2, the plaintext differs at byte 2 by the same XOR amount.

**Step 6** — Best practical algorithm:
1. Take the 9 identical-prefix c0d2 frames (same plaintext bytes 0..9).
2. Compare each c0d2 frame's bytes 10..32 to others — diffs show where bodies differ. Identify constant-plaintext positions (diff==0 across many frames).
3. For constant-plaintext positions where the plaintext is small (likely a zero high byte or a low-int flag), guess the plaintext byte value and derive the key byte for that position-mod-8.
4. Multiple positions mod 8 → consistency check; pick the candidate k[i] that's consistent.
5. Repeat until k[0..7] is fully determined.

**Step 7** — Decode + validate + write artifact + fact-set + DONE.

## Constraints & gotchas
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO new memdump. NO pyelftools. NO Frida. NO live device. NO MCP aeon.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤20 min wall time.**
- The cycle-347 `xor-key-recover` worker artifact at `/home/sdancer/dark-december-xor-key-recover/analysis/xor_key_recover_2026-05-15.md` (lines 71-91) has the table of server-family prefix-constancy — READ IT for prior cipher-pattern observations.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-xor-chained-state/`
- Frame corpus: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` (47 frames; 11 client_39 + 11 server_35)
- Move/Stand layout: `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`
- Falsified-cyclic artifact (READ the family analysis): `/home/sdancer/dark-december-xor-key-recover/analysis/xor_key_recover_2026-05-15.md`
- Original chained-XOR spec: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- success-fact key: `dark_december_wire_xor_state_recovered_<k_hex>_decoded_<n>` (a)
- block-fact keys: `dark_december_wire_xor_state_partial` (b), `dark_december_wire_xor_state_insufficient` (c)
