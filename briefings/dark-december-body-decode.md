# dark-december-body-decode — Reverse-engineer the :10001 frame body obfuscation

## Role & workdir
Cryptanalyst on captured frames. Workdir: `/home/sdancer/dark-december-body-decode` (worktree of `/home/sdancer/dark-december`, branch `body-decode`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump`.
- Sub-goal: decode the obfuscated bodies in the 47 captured `:10001` frames. Framing is solved; bodies show repeated-byte patterns suggesting XOR-rotating-key or position-substitution.

## Cross-pollination facts
- `dark_december_ingame_socket_captured`: framing closed. Frame format = `u32_le len + u16=0x0001 + N-byte body`.
- Body length classes: 8 (heartbeat), 41 (server→client), 45 (client→server).
- Repeated-byte runs in body: `1b1b1b1b`, `f2f2f2f2`, `dfdfdf`, `373737`, `434343`.

## Success criteria
Concrete deliverable:
- The deobfuscation algorithm (XOR key + scheme, or substitution table) that turns captured body bytes into structured/readable plaintext.
- Decoded body of at least 5 distinct frames — each yielding interpretable fields (timestamps, opcodes, coordinates, IDs).
- Fact `dark_december_protocol_body_decoded` on success.
- Falsification fact `dark_december_protocol_body_undecodable_<reason>` on failure.

## Next concrete tasks (single long turn, 3h budget)

1. **Extract clean frame bodies.**
   - Source: `/home/sdancer/dark-december-live-capture/captures/dd_live_10001_1778769976.pcap` (and the `_u_` variant).
   - Use tshark/pyshark/scapy to extract TCP payloads, then unmarshal the length-prefix framing:
     ```python
     # For each TCP segment payload, walk: u32_le_len + 2-byte channel + (len-6) body
     # Note: one TCP segment can contain MULTIPLE frames (per the live-capture artifact)
     ```
   - Dump each frame as `(timestamp, direction, length, body_hex)` to `analysis/frames.jsonl`.
   - Tally by `(direction, length)`. Expect 8/41/45 patterns.

2. **Pattern analysis on 8-byte heartbeat frames (easiest).**
   - There are 12 client→server and 13 server→client heartbeat frames.
   - Body after stripping 6-byte header is 2 bytes (e.g. `5dc0`, `79e4`, `59c4`).
   - Hypothesis A: it's a 2-byte sequence number XOR'd with rolling key.
   - Hypothesis B: it's a checksum/timestamp-derived value.
   - Test: sort by timestamp, plot the 2-byte values. Look for monotonic XOR drift OR per-packet randomness.
   - The `0800000001004051` repeated multiple times in server→client suggests `4051` is a STABLE value → either a constant ACK OR a session-ID-derived encrypted value. Compare bytes across multiple successive heartbeats — if identical, it's a constant (cipher with deterministic IV or unencrypted).

3. **XOR-key recovery on 41/45-byte frames.**
   - These are activity-triggered (taps/swipes). Bodies have repeated-byte runs.
   - If protocol is `XOR(plaintext, key[pos % keylen])`:
     - Identical bytes at same offsets across frames → same plaintext bytes XOR'd with same key bytes.
     - Compute byte-wise XOR between pairs of same-length frames. If the XOR result is "structured" (mostly zeros with sparse changes), the key cancels out → plaintext-diff visible.
   - Example: 5 client→server 45-byte frames. XOR `f1[i] ^ f2[i]` for each byte position; if many positions are zero → frames have many shared plaintext bytes → key is constant (or position-only).
   - Try keylen 4 / 8 / 16 / 32 — most embedded protocols use 4 or 8.
   - With keylen identified, recover key via crib drag (try known headers — many game protocols have a fixed 4-byte opcode at the front).

4. **Substitution / nibble-scheme test.**
   - Repeated runs like `1b1b1b1b` and `f2f2f2f2` suggest 4-byte alignment. Either:
     - Plaintext has 4-byte same values (e.g., float32 = 0.0 → 4 zero bytes) and key has structure → XOR result shows the repeated pattern.
     - Cipher is per-nibble substitution.
   - Check: do all repeated-run sequences start at multiples of 4 bytes from frame start?

5. **Look for embedded timestamps.**
   - 4-byte little-endian Unix timestamp (~1778769976 ≈ 0x6A05D...). After deobfuscation, expect to see 4 bytes close to that value somewhere in frame bodies.
   - Slide a window across each body, XOR with various candidate keys, check for ts-like values (4 byte LE within ±60s of capture time).
   - When found → that's the key + plaintext position.

6. **Lay out the protocol** based on decoded bodies:
   - Heartbeat shape (probably timestamp/counter only).
   - Activity-frame shape (probably opcode + params).
   - Write `<workdir>/analysis/body_decode_2026-05-14.md` with:
     - The (key, scheme) recovered.
     - Decoded interpretation of ≥5 frames.
     - Python script that, given any captured frame, decodes the body.
     - Set fact `dark_december_protocol_body_decoded`.

## Constraints & gotchas
- Pure offline analysis. No device interaction; just analyze the captured pcaps.
- If body is truly random / strong cipher → cannot decode without keys. Document that as a strong falsification — would then need to extract session keys from the running app (which requires defeating Hercules, currently blocked).
- Frames are SMALL (8/41/45 bytes). XOR/substitution recovery is feasible only because we have many same-length frames.

## Relevant files / references
- pcaps: `/home/sdancer/dark-december-live-capture/captures/dd_live_10001_*.pcap`
- live-capture closing artifact: `/home/sdancer/dark-december-live-capture/analysis/live_protocol_capture_2026-05-14.md` — has the first 5 frames of each direction in hex (use these as test vectors).
- Hive crypto idiom (sibling fact): SHA1/SHA256(timestamp) first 16 hex chars UTF-8 → AES-128. The :10001 protocol might use a similar idiom for stream-cipher keystream initialization.
- Tools: `python3`, `scapy`, `pyshark`, `numpy` (for XOR pattern matrix), `tshark`.

## Falsification
- All 4 keylen hypotheses (4/8/16/32) yield random XOR results across all frame pairs → cipher is not a simple repeating-XOR.
- Substitution analysis yields no consistent table → not a substitution cipher.
- After 2-3 hours of analysis, no decoded plaintext fields → bodies are encrypted with a real stream/block cipher with session keys. Then the goal NEEDS session-key extraction (escalate via the kernel-hide-frida or magisk-frida-gadget routes).
