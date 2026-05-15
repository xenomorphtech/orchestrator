# xor-mask-layer — Test the packet-id byte-mask hypothesis on top of the chained XOR

## Role & workdir
Pure-Python cryptanalysis worker. Workdir: `/home/sdancer/dark-december-xor-mask-layer`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `decode-with-packet-id-byte-mask-on-chained-xor`

## Why this turn exists
- Cycle 346 `move-synthesize`: wire layout 100% recovered (37/33 bytes; packet IDs 0x0385/0x0386).
- Cycle 347 `xor-key-recover`: simple cyclic XOR falsified. **Preserved positive signal**: c0d2/c3d1 server families differ by `0x03 0x03`.
- Cycle 349 `xor-chained-state`: chained XOR with handle assumptions also FALSIFIED. **Critical new finding**: server-family ciphertext XOR is `03 03 03 03 03 03 03 03 03 03` across ALL 10 prefix bytes (not just bytes 0..1). This is **impossible under any pure-XOR cipher** if the two families share the same actor handle plaintext — because XOR cancels and would give all-zeros, not `0x03`-everywhere.

## Hypothesis
There's an **additional byte-level XOR mask = `packet_id_low_byte`** applied to the wire ciphertext (above the chained XOR). Decryption:

```
c_chained[i] = c_wire[i] XOR packet_id_low_byte        # outer mask, applied PER BYTE
p[0]         = c_chained[0] XOR k[0]
p[i]         = c_chained[i] XOR p[i-1] XOR k[i & 7]    # chained recurrence
```

For Stand (pid 0x0385): `mask = 0x85`. For Move (pid 0x0386): `mask = 0x86`.

Cross-check prediction (decisive): under this model, Stand and Move broadcasts of the SAME actor must produce identical plaintexts after byte 2 (the handle is the same). The c0d2/c3d1 ciphertext XOR of `03 03 03 03 03 03 03 03` is exactly `mask_stand XOR mask_move = 0x85 XOR 0x86 = 0x03` × 10 bytes — matches.

The `0x8a/0x8b/0x11` markers from prior pcap-side affine-layer analysis (cycles 130-150) MIGHT be the per-direction (client/server) byte mask values for a different protocol family — worth a quick check that 0x8a XOR 0x8b = 0x01 (one-bit difference between request/response). Or the mask might be `packet_id_high_byte` (`0x03`) — try both.

## Falsification (3 outcomes)
- (a) **Mask = packet_id_low_byte yields plausible plaintext** (msg_type matches expected, handle non-degenerate u64, ≥3 of 6 coord floats in [-1e6, 1e6]) for ≥50% of 22 long frames → SUCCESS. Fact: `dark_december_wire_xor_mask_recovered_packetid_decoded_<n>`.
- (b) **Mask = packet_id_low_byte solves the prefix consistency but coords still wrong** → mask is correct concept but applied at a different layer or with different value. Document the gap. Fact: `dark_december_wire_xor_mask_partial`.
- (c) **No simple per-byte mask makes coords plausible** → wire cipher has a non-XOR component (e.g. nibble shuffle, rotate, modular add). Fact: `dark_december_wire_xor_mask_non_xor_component`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-xor-mask-layer/analysis/mask_layer_2026-05-15.md` with:
1. **The hypothesis derivation**: explicit derivation showing why `c0d2 XOR c3d1 = 03 03 03 03 03 03 03 03` IS the signature of a per-byte packet-id-low-byte mask (and only that).
2. **Re-solve k[0..1] under the masked model**:
   - For Stand frame `c0d2 ...`: `c_chained[0] = 0xc0 ^ 0x85 = 0x45`. `p[0] = 0x85` (known). `k[0] = 0x45 ^ 0x85 = 0xc0`. (Or whatever the math comes out to — show work.)
   - `c_chained[1] = 0xd2 ^ 0x85 = 0x57`. `p[1] = 0x03` (known). `k[1] = c_chained[1] ^ p[0] ^ p[1] = 0x57 ^ 0x85 ^ 0x03 = 0xd1`.
3. **Re-derive k[2..7]** using the 9-frame c0d2-family constant prefix + the actor handle assumption (e.g. high byte zero). The handle equations now use `c_chained` not `c_wire`.
4. **Cross-validate with c3d1 family**: under the model, both families should yield the SAME plaintext for bytes 2..9 (same actor handle). Compute and check.
5. **Decode all 22 long frames** and report:
   - msg_type per frame (must be 0x0385 or 0x0386)
   - actor_handle u64 (small set of distinct values across the corpus)
   - 6 × float32 coord/param fields (plausibility per the success criterion)
6. **Try alternative masks** if primary fails: `packet_id_high_byte`, `XOR with cycle-counter`, `0x8a/0x8b/0x11` legacy affine markers, etc.
7. Verdict matched to (a)/(b)/(c) and set the closing fact via `harness fact-set`.

Print `XOR_MASK_LAYER_DONE` on the final line.

## Execution flow — atomic, ≤20 min wall time, 500 MB cap

**Step 1** — Load frames.jsonl; isolate the 22 long frames. Re-derive the c0d2 vs c3d1 byte-XOR (should be `03 03 03 03 03 03 03 03 03 03` confirming prior worker's observation).

**Step 2** — Implement the masked-chained-XOR decoder. Solve `k[0..1]` under the new model.

**Step 3** — Use the 9 identical-prefix c0d2 frames as the SAME plaintext anchor. Their `c_chained` bytes are identical → their plaintext bytes are identical → solving for `k[2..7]` reduces to solving for 6 unknown handle bytes (`h0..h5`) under the chained recurrence. Try the handle-high-byte-zero assumption again — this time it might succeed because the model accounts for the mask.

**Step 4** — Cross-validate: decode the 2 c3d1 frames; their plaintext (after mask removal with mask=0x86) should give the SAME handle as the c0d2 family.

**Step 5** — Validate float slot plausibility against the prior worker's invariant analysis. The prior worker proved that under the UNMASKED model, all server frames have at least one invariant float slot stuck in implausible regimes. After applying the mask, those slots should become plausible (or the hypothesis is wrong).

**Step 6** — If primary fails, run the alternative-mask sweep: `0x86, 0x85 ^ 0x86 = 0x03, 0x8a, 0x8b, 0x11, 0x00`, and any byte derivable from the frame's TCP-layer metadata.

**Step 7** — Write artifact + fact-set + DONE.

## Constraints & gotchas
- **HARD memory budget: 500 MB.**
- **NO new disasm. NO new memdump. NO pyelftools. NO Frida. NO live device. NO MCP aeon.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤20 min wall time.**
- The prior worker's invariant float analysis at `/home/sdancer/dark-december-xor-chained-state/analysis/chained_state_recover_2026-05-15.md` (lines 36-55) is the GROUND TRUTH against which to test the new model — if your model produces plausible floats for those exact slots, you've succeeded.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-xor-mask-layer/`
- Frame corpus: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` (47 frames; 11 client_39 + 11 server_35)
- Prior chained-state analysis (FALSIFIED but with rigorous invariant slot table — READ IT): `/home/sdancer/dark-december-xor-chained-state/analysis/chained_state_recover_2026-05-15.md`
- Move/Stand layout: `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`
- Original chained-XOR spec from cycle 335: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Prior pcap-side affine layer (0x8a/0x8b/0x11 markers) reference: search `harness facts | grep affine` if needed.
- success-fact key: `dark_december_wire_xor_mask_recovered_packetid_decoded_<n>` (a)
- block-fact keys: `dark_december_wire_xor_mask_partial` (b), `dark_december_wire_xor_mask_non_xor_component` (c)
