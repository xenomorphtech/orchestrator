# dark-december-bootstrap-decode — Continue: semantic field labels for the 5 bootstrap frames

## Role & workdir
Offline protocol analyst, second turn. Workdir: `/home/sdancer/dark-december-bootstrap-decode`. **No device interaction, no captures, no aeon/openviking, no Frida.** Pure analysis of already-extracted latent bodies.

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump` (target 1.0, current 0.995).
- Sub-goal: **semantic field labeling** for the 5 bootstrap frames now that latent bodies are extracted. Move 0.995 → 1.0.

## Prior-turn status
Task 1 closed last turn: affine cell-collapse rule recovered (`0x8a`/`0x8b` client family, `0x11` server). Latent bodies extracted for all 5 frames and recorded in `analysis/bootstrap_decode_2026-05-14.md`:

- Frame 0 (c→s, 24B latent): `931f2e00a0053506ad21602ef8216e27f17afa1a57c0c1c1`
- Frame 1 (s→c, 4B latent): `47555555`
- Frame 2 (c→s, 98B latent): `eb4a2946e94c2247e00f7c1bb85e3b48a44b2a58f11e7b18bf593b5eee4b2a44e21b741dbb1e7916bb573b5e972d1d2cd9605063902b1327ef516254a01c281fe9516876d56b0d3d9b26152c883a08389c211170866b5a6398224723876f5d3b9f71`
- Frame 3 (s→c, 4B latent): `6f7d7de5`
- Frame 4 (c→s, 87B latent): `ac2d1c2c9b261424912a1a22901d5c12d008470ecc447547c9315a69da68500ffa75107ed50a595cbf5b7623f67b4a64d0744477c84608678f60642ce33f7a7afc767676f2307b885cd6d6d650db79f99fcad6d751dbdb`

## Success criteria
Append a new section "## Task 2-5 Closure" to `analysis/bootstrap_decode_2026-05-14.md` (same file — append, do not rewrite). Sections:

1. **First-4-bytes parse**: for frames 0/2/4, interpret first 4 latent bytes as (a) LE float32, (b) LE u32, (c) BE u32. Are any of them sensible "opcode | length | sequence | version"?
2. **Frames 1 & 3 are likely ACKs**: 4 latent bytes each. Cross-reference with frames 0/2/4 — do they encode a length/sequence/opcode that pairs with the preceding client frame?
3. **Field-boundary hypotheses** for frames 0 (24B), 2 (98B), 4 (87B). Common Korean MMO socket bootstrap layouts:
   - `u16 opcode | u16 flags | u32 sequence | u32 timestamp | payload`
   - `u8 type | u8 version | u16 length | <body>`
   - `4B magic | 4B version | 4B nonce | <client info>`
4. **Pmang/Hive/Nexon TCP-socket family** comparison: search prior body-decode artifacts (`/home/sdancer/dark-december-body-decode/analysis/`) for known Hive auth-socket headers (HiveAuthV4Network, HiveProtocolHelper conventions). State if the bootstrap shapes are consistent with Hive's TCP socket family OR a Korean MMO custom protocol.
5. **Verdict**: assign a confidence-labeled hypothesis to each of the 5 frames. Set fact `dark_december_bootstrap_decoded` (≥3 confident labels) OR `dark_december_bootstrap_partial` (<3 confident; document what's left).

## Tasks (single-pass, do not loop)

### TASK 2 — First-4-bytes parse for frames 0/2/4 (client)
- Frame 0 first-4 latent: `93 1f 2e 00` → LE u32=0x002e1f93=3022227, BE u32=0x931f2e00, LE f32, BE f32
- Frame 2 first-4 latent: `eb 4a 29 46` → LE u32=0x4629(46)?, BE u32=0xeb4a2946
- Frame 4 first-4 latent: `ac 2d 1c 2c` → similar
Report the four candidate interpretations per frame. Highlight any with clean game-protocol semantics (small ints, recognizable opcodes, timestamps near 1778771203 UNIX time).

### TASK 3 — Frames 1 & 3 as ACK candidates
- Frame 1 `47 55 55 55` (s→c, latent 4B): does `0x47` (=71) match anything in frame 0?
- Frame 3 `6f 7d 7d e5` (s→c, latent 4B): does `0x6f` (=111) match anything in frame 2?
Look for length/sequence echoes. State what frames 1/3 likely encode.

### TASK 4 — Field-boundary hypothesis tables for frames 0, 2, 4
- Per frame: propose a tentative `offset → field` map (e.g., 0..2 = opcode, 2..4 = flags, 4..8 = sequence, 8..end = payload).
- Cite which evidence supports the boundary (repeated bytes, alignment, plausible value range).

### TASK 5 — Verdict + fact-set + exit
- Append "## Task 2-5 Closure" section with sections 1-4 above + a final "## Verdict" line: per-frame confidence (high/medium/low) + 1-line label.
- `harness fact-set dark_december_bootstrap_decoded "<one-paragraph summary>"` if ≥3 confident.
- Otherwise `harness fact-set dark_december_bootstrap_partial "<what was learned + what's left>"`.

## Constraints & gotchas
- No device interaction. No new captures. No aeon. No Frida.
- Single-pass turn: do tasks 2-5, append to artifact, set fact, exit.
- Memory budget: lean. No large file reads beyond targeted greps.
- **Append, do not rewrite** the existing artifact — Task 1 section is canonical.

## Falsification (acceptable outcomes)
- First-4-bytes parse yields no clean opcode/length/timestamp interpretation → mark `dark_december_bootstrap_partial` with what was tried.
- Frames 0/2/4 share an identical 4B prefix but it doesn't match any known Hive/MMO header → still partial.
- Frames 1/3 don't pair with 0/2/4 → note inconsistency, still partial.

## Relevant files / references
- workdir: `/home/sdancer/dark-december-bootstrap-decode/`
- existing artifact: `analysis/bootstrap_decode_2026-05-14.md` (Task 1 closure — APPEND, do not overwrite)
- sibling decoded frames: `/home/sdancer/dark-december-body-decode/analysis/decoded_frames.jsonl`
- sibling capture metadata: `/home/sdancer/dark-december-handshake-capture/captures/dd_handshake_10001_1778771059.pcap` (timestamp ~1778771203 = May 14 2026 ~14:46 UTC = handshake epoch reference)
- fact key (success): `dark_december_bootstrap_decoded`
- fact key (partial): `dark_december_bootstrap_partial`
