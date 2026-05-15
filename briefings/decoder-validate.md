# decoder-validate — End-to-end validation of the static-recovered ingame decoder stack

## Role & workdir
Pure-Python validation worker. Workdir: `/home/sdancer/dark-december-decoder-validate`.

## Current goal / sub-goal
- **goal_key**: `dark_december_ingame_decoder_end_to_end_validated` (new)
- **sub_goal_key**: `decode-captured-frame-stack`

## Why this turn exists
The static-side ingame protocol decoder stack is now fully recovered across 3 paths:
1. **Body wire format** (`libue4-rz-protocol`): IRzBuffer vtable model, FRz*Info Serialize/Deserialize pairs.
2. **Packet framing** (`libue4-pktsystem-disasm`): 6-byte header layout (`length:u16 | ?:u16 | channel:u16 | body`), RzWriterBuffer model.
3. **Obfuscation** (`libue4-xor-synthesize`): rolling XOR `p0=c0^k0, pi=ci^p(i-1)^k[i&7]`, 8-byte LE key at `this+0xb8`, applied to payload starting at offset +6, state advances by payload_len.

Prior pcap-side campaign has 167-frame captures available. Combining them = end-to-end validation: can we decode a real captured frame to semantic content?

## Hypothesis
Applying the recovered XOR algorithm to the `body_hex` portion of a captured frame (after the 6-byte header) will produce plaintext bytes that match the IRzBuffer per-message Serialize format — visible as recognizable integer/string fields matching one of the known `FRz*Info` message types.

## Falsification (3 clean outcomes)
- (a) **At least one frame decodes cleanly** to plaintext matching a known message type → SUCCESS. Fact: `dark_december_ingame_decoder_validated_<msgtype>_<frameidx>`. Output: decoded message fields.
- (b) **XOR produces apparent plaintext but doesn't match any known message type** → wire format mismatch (maybe per-channel multiplexer, maybe extra inner layer). Document patterns. Fact: `dark_december_ingame_decoder_plaintext_unrecognized`.
- (c) **XOR with discovered algorithm produces noise** → key derivation wrong OR algorithm subtly wrong. Document gap. Fact: `dark_december_ingame_decoder_xor_inconsistent`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-decoder-validate/analysis/decoder_validate_2026-05-15.md` with:
1. The Python decoder implementation (≤200 lines, from the recovered XOR spec).
2. Decode results for first 10 client→server frames + first 10 server→client frames from `frames.jsonl`.
3. For each: (a) ciphertext hex, (b) attempted XOR plaintext hex, (c) interpretation as FRz*Info if possible, (d) verdict (decoded / partial / noise).
4. Key derivation discussion: where does the 8-byte key come from for a fresh connection? Handshake frames may reveal it.
5. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c).

Print `DECODER_VALIDATE_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Read frames.**
```bash
mkdir -p analysis
wc -l /home/sdancer/dark-december-body-decode/analysis/frames.jsonl
head -3 /home/sdancer/dark-december-body-decode/analysis/frames.jsonl
```

**Step 2 — Read the XOR spec.**
```bash
cat /home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md
```

**Step 3 — Implement decoder in Python.**
```python
# Recovered algorithm:
#   p0 = c0 ^ k0
#   pi = ci ^ p(i-1) ^ k[i & 7]    -- cipher-feedback chain
# Key: 8-byte LE field at this+0xb8 (per-connection state)
# Payload offset: +6 from frame start
# State advances by payload_len (carries across packets within a connection)
def decode_payload(cipher_bytes, key8):
    out = bytearray(len(cipher_bytes))
    if not cipher_bytes: return bytes(out)
    out[0] = cipher_bytes[0] ^ key8[0]
    for i in range(1, len(cipher_bytes)):
        out[i] = cipher_bytes[i] ^ out[i-1] ^ key8[i & 7]
    return bytes(out)
```

**Step 4 — For unknown key, try brute-force key recovery.**
The 8-byte key is per-connection. Best candidates:
- All-zero key (sometimes used for unauthenticated channels).
- First 8 bytes of handshake response from the server.
- Specific known-plaintext attack: if the first message has a known field (e.g. `FRzAppGuardRq` has known structure), we can recover the first 8 key bytes.

```python
# Known-plaintext key recovery for first packet:
# If we know p[0..7], we can solve for k[0..7].
# p0 = c0 ^ k0 → k0 = c0 ^ p0
# For i in 1..7: pi = ci ^ p(i-1) ^ k[i&7] → k[i] = ci ^ p(i-1) ^ pi
```

**Step 5 — Try each known message type as the known-plaintext for the first server response.**
Server→client first frames often acknowledge with a small fixed-structure message. Look at `frames.jsonl` for `direction=server_to_client` first frames.

**Step 6 — Validate decode candidates.**
A successful decode shows:
- Sensible integer ranges (small positive numbers for counts, IDs).
- ASCII bytes for any embedded string segments.
- Recognizable structure boundaries.
- A noise decode shows high-entropy random bytes throughout.

**Step 7 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Pure Python, small frames, trivial.
- **No new binary disasm.** Use the recovered spec.
- **No Frida / no device interaction.** Pure offline.
- **HARD output cap**: artifact ≤500 lines; decoded frame log ≤200 entries.
- **One Codex turn budget: ≤45 minutes wall time.**
- The XOR is a STREAM cipher with state — applying it correctly requires the state pointer across frames within a connection.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-decoder-validate/`
- Frame data (read-only): `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`, `handshake_frames.jsonl`, `frames_u.jsonl`, `decoded_frames.jsonl`
- Raw PCAPs: `/home/sdancer/dark-december-live-capture/captures/`, `/home/sdancer/dark-december-protocol-live-dump/captures/`
- XOR algorithm spec: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Packet framing spec: `/home/sdancer/dark-december-libue4-pktsystem-disasm/analysis/pktsystem_framing_2026-05-15.md`
- Message body spec: `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md`
- Prior pcap campaign affine layer (alternate hypothesis): `dark_december_bootstrap_decoded` fact mentions `0x8a/0x8b/0x11` markers
- success-fact key: `dark_december_ingame_decoder_validated_<msgtype>_<frameidx>` (a)
- block-fact keys: `dark_december_ingame_decoder_plaintext_unrecognized` (b), `dark_december_ingame_decoder_xor_inconsistent` (c)
