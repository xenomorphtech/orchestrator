## protocol-wiki — Distill 19 cycle closures into a consolidated Dark December protocol reference

## Role & workdir
Documentation worker. Workdir: `/home/sdancer/dark-december-protocol-wiki` (worktree of `/home/sdancer/darkdecember/`, branch `protocol-wiki`).

## Current goal / sub-goal
- **goal_key**: `dark_december_protocol_wiki` (new)
- **sub_goal_key**: `consolidate-19-closures-into-state-of-understanding-doc`

## Why this turn exists
This autonomous /orchestrate run has closed **19 dark-december goals** producing a comprehensive protocol decode (wire cipher, framing, 822-typename catalog, 7 high-confidence Rz bindings, FRzMoveBr field layout including the cycle-435 framing correction, mirror-block pattern verification on len=61 + partial on len=53). The findings are spread across 19 worker artifacts in 19 separate worktrees and a memory note. There is no single consolidated reference doc.

A wiki-style state-of-understanding doc accelerates every future cycle by giving operators / workers a single jump-off point instead of the artifact-spelunking pattern that has driven the last 5 cycles.

## Hypothesis
A focused 250-350 line reference at `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md` consolidates the 19 closures into 6 sections (Wire framing / Cipher / Catalog / Typed decoder / Per-class field maps / Open questions) without information loss, suitable as the canonical entry point for downstream protocol consumers.

## Falsification (3 outcomes)
- (a) **Wiki delivers all 6 sections + correctly cites all 19 closing facts** → SUCCESS. Fact: `dark_december_protocol_wiki_19_closures_consolidated`.
- (b) **Wiki contains contradictions vs the artifact facts** → editorial error; needs revision. Fact: `dark_december_protocol_wiki_contradiction`.
- (c) **Wiki delivers 4-5 of 6 sections** with the rest as TODOs → partial; document gaps. Fact: `dark_december_protocol_wiki_partial`.

## Success criteria — SINGLE TURN

**Primary**: write `/home/sdancer/dark-december-protocol-wiki/PROTOCOL_WIKI.md` with these sections:

### 1. Wire framing (cycle 435 corrected)
- `[4B total_len LE][1B flag=0x01][1B channel2][raw_body]` — 6-byte outer header.
- `decoded[i] = raw_body[i] ^ raw_body[i+1]` (cycle-383 keyless adjacent-XOR).
- `wire_len = Serialize_size + 7` (NOT +8 from cycle-412 — the cycle-415 layout had off-by-one).
- The decoded view IS the Serialize body. No additional packet_id at raw_body[0..1].

### 2. Cipher
- Adjacent-XOR is the protocol cipher. Keyless. dec is 1 byte shorter than raw_body.
- For per-frame reconstruction: `raw_body[i+1] = raw_body[i] ^ decoded[i]`. No reliable seed for raw_body[0] (cycle 435 falsified `0x86` seed).

### 3. Rz typename catalog (cycle 406)
- 822 typenames: 421 CL_GS::F*Rq, 400 GS_CL::F*Rp, 1 Notify.
- 23 fire-and-forget gameplay Rqs with no Rp (Move, Stand, HeartBeat, HitSpawn, ControlProjectile, ApplyProjectile, SightEnter, etc.).
- 223 reflected (USTRUCT) structs with NewProp_* field names extractable from dump.
- 61 RzPktAction/RzPktSystem RQ/RP handler signatures revealing payload struct types.
- Source: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`

### 4. Typed Rust decoder (cycles 403/417/422/429)
- bit-exact Python parity on the first_quest corpus (47,987 frames).
- Typed Packet enum with 7 high-confidence bindings: FRzMoveRq, FRzMoveBr, FRzSkillMoveSyncRq, FRzMoveDuringSkillRq, FRzControlProjectileRq, FRzTriggerActiveBr, FRzCoolTimeResetNoti.
- Coverage 64.63%. Pushed to `darkdecember/rust-decoder-typed-v2` + minimap-sniffer integration at `darkdecember/minimap-sniffer-typed`.

### 5. Per-class field maps

**FRzMoveBr (S2C len=41, 33B Serialize, 23,568 frames)** — corrected per cycle 435:
```
dec[0]      msg_subtype u8 (= 0x12)
dec[1..8]   actor_handle u64
dec[9..12]  coord_param_0 f32 = pos_x   (cycle-403 anchor)
dec[13..16] coord_param_1 NON-float (range 1e+14)
dec[17..20] coord_param_2 f32 = pos_z   (cycle-403 anchor)
dec[21..24] coord_param_3 NON-float (range 1e-10)
dec[25..28] coord_param_4 f32 = heading_x   (normalized -1..1, cycle-433)
dec[29..32] coord_param_5 NON-float (range 1e-28 padding)
dec[33]     move_flag u8
```

**FRzMoveRq (C2S len=45, 37B Serialize, 6,022 frames)** — cycle 395:
8/8 named positions: token@0, subtype@1, marker@6, class_b@9..12, coord@17, heading@25, subtype@33, tail@37. 3 opaque slots @13/21/29 (cycle 433 falsified all structural interpretations).

**FRzSkillMoveSyncRq (C2S len=61, 952 frames)** — cycle 407 + cycle 438:
Len=45-family + 16-byte mirror block at dec[17..32]:
```
dec[17..20] = mirror_subtype_u32_le  (mirrors dec[49])
dec[21..24] = mirror_header_quad_5_8  (copies dec[5..9])
dec[25..28] = mirror_class_b_signature  (copies dec[9..13])
dec[29..32] = opaque_u32
```

**FRzMoveDuringSkillRq (C2S len=53, 104 frames)** — cycle 441:
Len=45-family + 8-byte mirror block at dec[17..24]:
```
dec[17..20] = mirror_subtype_u32_le  (104/104 matches dec[41]; same pattern as len=61)
dec[21..24] = opaque_u32  (NOT a mirror of any len=45 field)
```
NOTE: cycle-395's heading@25 -> dec[33] shift under +8 model is FALSIFIED for len=53; dec[33..36] is another zero-extended subtype lane.

**FRzControlProjectileRq (C2S len=42, 16 frames)** — bound but field offsets not individually analyzed.

**S2C secondary bindings (cycle 419)**:
- FRzTriggerActiveBr (len=27 / 192 frames, prefix `1b0301`, temporal lift 31.7)
- FRzCoolTimeResetNoti (len=32 / 158 frames, prefix `9b000160` + `ed030000`, temporal lift 44.8)

**C2S len=10 (3607 frames)** — best candidate is `FRzCharacterLookVisibleChangeRq` (cycle 424, (c) partial); not high-confidence.

### 6. Open questions / TODOs
- FRzMoveBr coord_param_1/3/5 are integer/bitfield/padding, NOT f32. Bit-pattern reading requires per-slot Serialize disasm with controlled-input live capture.
- 36% of the corpus (~16,975 frames) remains in `Packet::Unknown` — falls back to legacy heuristic decoder.
- Wire framing for non-RzPktAction message families (login, account, lobby) is not validated.
- Live-loopback typed consumer (cycle 417 Rank 3) was deferred.

### Cross-references
List the 19 closure artifact paths + facts as a footnote table.

## Constraints & gotchas
- **HARD output cap**: PROTOCOL_WIKI.md ≤350 lines.
- **NO new RE work.** This is consolidation only — quote existing facts/artifacts; do not run new analyses.
- **ONE Codex turn budget: ≤15 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- All 19 closing facts should be referenced; the worker should `harness facts | grep dark_december_` to enumerate them.

## Relevant files / references
- 19 prior worktrees at `/home/sdancer/dark-december-*/analysis/*.md`
- 822-symbol catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- Memory: `/home/sdancer/.claude/projects/-home-sdancer-orchestrator/memory/project_dark_december_wire_framing_plus8.md` (REVISED), `project_dark_december_wire_decoder.md`
- success-fact key: `dark_december_protocol_wiki_19_closures_consolidated` (a)
- block-fact keys: `dark_december_protocol_wiki_contradiction` (b), `dark_december_protocol_wiki_partial` (c)
