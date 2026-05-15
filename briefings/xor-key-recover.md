# xor-key-recover — Recover the actual wire-layer XOR key via known-plaintext + cross-frame consensus

## Role & workdir
Pure-Python known-plaintext attack worker. Workdir: `/home/sdancer/dark-december-xor-key-recover`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing — moved from "implausible" to "decoded")
- **sub_goal_key**: `recover-wire-xor-key-via-known-plaintext`

## Why this turn exists
`move-synthesize` (cycle 346) recovered the FRzMove/FRzStand wire layout (37 client, 33 broadcast bytes) AND the packet IDs (0x0385 Stand, 0x0386 Move) — exact size match with the 47-frame capture. BUT applying the chained-XOR algorithm from cycle 335 produces non-physical floats.

**Critical observation in the synthesis report**: server long frames split into two stable ciphertext-prefix families with byte-pair pattern `c0 d2 ...` vs `c3 d1 ...`. The delta is `0x0003 0x0003` — exactly the packet-id XOR delta `0x0385 XOR 0x0386`. This is the **signature of a simple cyclic-key XOR with NO feedback** (`p[i] = c[i] XOR k[i mod N]`), NOT the cycle-335 chained algorithm.

Conclusion: either (i) the wire cipher is a different function than 0x6ce57c1904, or (ii) the chain feedback is short-circuited by per-packet state reset. Either way, the recoverable key is **the cyclic key directly visible in the cipher pattern**. We can recover it via known-plaintext — without any new disasm.

## Hypothesis
The wire-layer cipher is `plain[i] = cipher[i] XOR key[i mod KEYLEN]` where `KEYLEN ∈ {2, 4, 8, 16}`. Given known plaintext bytes (the 2-byte packet ID at position 0, and the inferred constant `actor_or_move_handle` u64 across frames of the same player), we can recover `key[]` directly. Cross-frame consensus disambiguates KEYLEN.

## Falsification (3 clean outcomes)
- (a) **A KEYLEN ∈ {2, 4, 8, 16} yields a consistent key across ALL frames AND decoded bytes pass plausibility checks (small ints near zero, float32 coords in [-1e6, 1e6])** → SUCCESS. Fact: `dark_december_wire_xor_keylen_<N>_key_<hex>_decoded_<frames>`.
- (b) **A KEYLEN recovers consistent first ~8 bytes but rest is garbage** → cipher has additional position-keyed transform beyond pure cyclic XOR. Fact: `dark_december_wire_xor_partial_cyclic_<N>`.
- (c) **No KEYLEN produces self-consistent key across frames** → cipher is genuinely position-dependent (key derived from byte index / frame seq / handshake) → escalate to caller-disasm path. Fact: `dark_december_wire_xor_not_simple_cyclic`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-xor-key-recover/analysis/xor_key_recover_2026-05-15.md` with:
1. **The recovery method**: cross-frame XOR analysis showing the cipher diff pattern matches plaintext-only diff (key cancels).
2. **Recovered key**: hex string + KEYLEN.
3. **Per-frame decoded output**: at least 10 frames (5 client-39 + 5 server-35), with decoded fields (msg_type, actor_handle, coord_or_param0..5, flag).
4. **Plausibility check**: do consecutive same-direction frames show:
   - Same `actor_or_move_handle` (or a small set of player IDs)?
   - Float32 coordinates in plausible game-world ranges (rough sanity: `-1e6 < x < 1e6`)?
   - Integer flag/state bytes in 0..255 with low diversity?
5. Verdict matched to (a)/(b)/(c).
6. **Set the closing fact** via `harness fact-set <fact_key> "<summary>"`.

Print `XOR_KEY_RECOVER_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Load frames.**
```python
import json
from pathlib import Path
frames = [json.loads(l) for l in Path('/home/sdancer/dark-december-body-decode/analysis/frames.jsonl').read_text().splitlines() if l.strip()]
client_39 = [f for f in frames if f['direction']=='client_to_server' and f['body_len']==39]
server_35 = [f for f in frames if f['direction']=='server_to_client' and f['body_len']==35]
print(len(client_39), len(server_35))   # expect 11 + 11
```

**Step 2 — Known-plaintext on first 2 bytes.**
We KNOW the packet ID at the start of payload. After the 6-byte RzPktSystem header? Or no header in the body_hex? Read frames.jsonl first 2 bytes:
- client `5dc0` (from cycle 338) decoded to `1a00` = packet-id `0x001a` = RQAppGuard

For Move/Stand frames, the plaintext first-2 bytes must be in {0x0385, 0x0386}. So `key[0] = cipher[0] XOR plain[0]`, `key[1] = cipher[1] XOR plain[1]`.

```python
# For each KEYLEN candidate {2,4,8,16}, derive key[0..1] from each frame assuming plain[0..1]∈{0x0385,0x0386}.
# Then check: does key[0..1] repeat consistently across frames? If yes, that's our cyclic key.
```

**Step 3 — Cross-frame XOR for full key length.**
For two same-direction same-length frames F1, F2 under a cyclic-key XOR:
```
F1[i] XOR F2[i] = P1[i] XOR P2[i]  (key cancels)
```
So pairs of frames give us plaintext-XOR pairs. Look for STRUCTURE in these diffs:
- Bytes that are CONSTANT across frames (same field same value) → diff is `0x00`
- Bytes that DIFFER → tells us which fields vary

```python
# For all pairs of server_35 frames, compute byte-XOR diff. The ZERO bytes in the diff mark constant-plaintext positions.
# Plot a heatmap (text-based, byte-position vs frame-pair-index) of where diff==0.
```

**Step 4 — Solve for key.**
With KEYLEN=N, and many frames sharing some same-position plaintext bytes (e.g. the high bytes of actor_handle for the same player), we can:
```
For position i with known plain[i] in MOST frames:
  key[i mod N] = cipher[i] XOR plain[i]
```
Try KEYLEN=2, 4, 8, 16. The TRUE KEYLEN is the one where `key[i mod N]` is CONSISTENT across all frames at all observable positions.

**Step 5 — Decode and validate.**
Once key recovered, apply to all 22 long frames. Validate:
- All msg_type bytes match `0x0385` or `0x0386`.
- The 8-byte `actor_or_move_handle` field clusters into a small set (1-3 distinct player IDs across the 47-frame corpus).
- The 6×4-byte coord/param fields, interpreted as f32, produce values in a plausible UE4 world range.
- The trailing 1-byte flag is a small int.

**Step 6 — Write artifact + fact-set + DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set <fact_key> "<one-line summary with KEYLEN, key bytes, frames decoded>"
echo XOR_KEY_RECOVER_DONE
```

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Pure Python on 47 frames × 47 bytes max. Trivial.
- **NO new disasm. NO new memdump. NO pyelftools. NO Frida. NO live device. NO MCP aeon.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤20 min wall time.** This is pure crypto math.
- **HEADER OFFSET NOTE**: `body_hex` in frames.jsonl is the BODY AFTER framing. The 6-byte RzPktSystem header is NOT in body_hex. The 8-byte LE XOR state and packet-id are at the START of body. Don't double-skip.
- **Honor `[[bulk-enumeration-needs-explicit-memory-budget]]`** — declared 500 MB cap.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-xor-key-recover/`
- Frame data (read-only): `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` (47 frames)
- Recovered wire layout: `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`
- Prior XOR algorithm spec (the one that doesn't match): `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Prior partial validation (1a00 packet ID recovery): `/home/sdancer/dark-december-decoder-validate/analysis/decoder_validate_2026-05-15.md`
- Catalog: `/home/sdancer/dark-december-rz-message-catalog/analysis/rz_catalog_2026-05-15.md`
- success-fact key: `dark_december_wire_xor_keylen_<N>_key_<hex>_decoded_<frames>` (a)
- block-fact keys: `dark_december_wire_xor_partial_cyclic_<N>` (b), `dark_december_wire_xor_not_simple_cyclic` (c)
