## frzmove-br-decode — Decode S2C FRzMoveBr / FRzStandBr / FRzForceMoveBr field-by-field

## Role & workdir
Pure-analysis worker. Workdir: `/home/sdancer/dark-december-frzmove-br-decode` (worktree of `/home/sdancer/darkdecember/`, branch `frzmove-br-decode`).

## Current goal / sub-goal
- **goal_key**: `dark_december_frzmove_br_field_decode` (new)
- **sub_goal_key**: `name-all-33-serialized-bytes-of-frzmove-br`

## Why this turn exists
Cycle 412 bound the dominant S2C length=41 class (23,570 frames) to **`FRzMoveBr` / `FRzStandBr` / `FRzForceMoveBr`** (33-byte Serialize + 8B outer frame). This is the authoritative C++ source of all player + entity position updates. Cycle 403's Rust decoder already extracts `(x, z, rot)` at decoded offsets 9 / 17 / 25 in observed frames — but those offsets were inferred from data, not derived from C++ struct layout. Disassembling `FRzMoveBr::Serialize` yields the **canonical field offsets and types**, settles whether `Br` is a single C++ struct or three siblings, and connects the 23,570 wire frames to UE4 reflection / RzPktAction callsites.

## Hypothesis
`FRzMoveBr::Serialize` writes 33 bytes in a sequence of `IRzBuffer` raw-scalar calls (slot +0x60). The byte order is `[playerOrEntityID:u64=8B][x:f32=4B][z:f32=4B][rot:f32=4B][... small fields:13B]` totaling 33B. The "small fields" tail carries the cycle-403 entity-id byte at decoded offset 1 (`12 <id> 86 01 00 00 00 46 11`) plus state/status bytes.

## Falsification (3 outcomes)
- (a) **All 33 Serialize bytes named with type + field role**, with the cycle-403 player_track CSV's (x, z, rot) values reproducible by re-decoding the same frames using the new field map → SUCCESS. Fact: `dark_december_frzmove_br_33_bytes_named`.
- (b) **`FRzMoveBr`, `FRzStandBr`, `FRzForceMoveBr` have DIFFERENT Serialize layouts** despite all being 33B → name each separately. Fact: `dark_december_frzmove_br_3_variants_named`.
- (c) **Some bytes are computed at Serialize time** (delta-encoded, bit-packed, or quantized) so they don't map 1:1 to source struct fields → name what's decodable and document the encoding. Fact: `dark_december_frzmove_br_encoded_fields_<n>_named`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-frzmove-br-decode/analysis/frzmove_br_decode_2026-05-15.md` with:

1. **Resolve VAs** for `FRzMoveBr::Serialize`, `FRzStandBr::Serialize`, `FRzForceMoveBr::Serialize` from the dynsym strings in `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin`. Live VA = `st_value + 0x6cdd243000`; .text shard offset = `live_va - 0x6ce4bd4000`.
2. **Disassemble each method** from the .text shard at `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` using `capstone` aarch64 mode. Bound disasm to ≤500 instructions per method.
3. **Field table**: for each `IRzBuffer::append` vtable call (slot +0x60), record the source field address relative to `this` and the `w2` byte count. Build a 33-byte ordered field list.
4. **Field naming**: cross-reference with:
   - `RzPktAction::RQMove(ARzCharacter*, FRzActionInfo&)` — extracts `usingSkill`, `invokeSkill`, `usingSkillKey` from `FRzActionInfo`; check if these flow into the broadcast.
   - UE4 reflection field names for nested structs (e.g., if a slot writes a nested object via +0x68, the nested struct may have `NewProp_*` reflection — look up `Z_Construct_UScriptStruct_FRz<NestedName>_Statics*NewProp_*` in the same dump).
   - `URzUIDevDamageInfo::GetValue_*` and `URzUIDevAttributeInfo::GetValue_*` helper symbols may name struct fields by accessor.
5. **Validation**: re-decode the 6013 `12 02 60 6d` player frames from `streams/first_quest/first_quest_s2c.tcpstream.bin` using the new field map and compare against `/home/sdancer/orchestrator/darkdec_output_streams/player_track.csv` — (x, z, rot) values must match exactly. Same for entity_tracks.csv on the 17555 entity frames.
6. **Reconcile cycle-403 inferred offsets** (x@9, z@17, rot@25 in the decoded view) against the Serialize-derived offsets. The +1 shift from adjacent-XOR collapse should explain the relationship: decoded[k] = raw[6+k] ^ raw[6+k+1] for k in [0, 32], so the field at Serialize offset N appears at decoded offset N-2 (after the 2B packet_id at raw 6..7 is consumed).
7. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `FRZMOVE_BR_DECODE_DONE` on the final line.

## Execution flow

**Step 1 — Resolve VAs:**
```bash
grep -aoE '_ZN[0-9]+(GS_CL|FRz)[0-9]*F?Rz(Move|Stand|ForceMove)Br[0-9]+SerializeE[^[:space:]]*' \
  /home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin | head -5
# Use the prior rz-typename-binding worker's helper at
# /home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_build.py
# to extract st_value for these symbols.
```

**Step 2 — Per-method disasm + IRzBuffer call extraction.** Reuse the bounded-disasm framework from `rz_binding_build.py` if it exposes one. Otherwise: read 500 insns from each method's offset, identify `bl` + `ldr` patterns where x0 is buffer, x1 is field-source, w2 is byte count.

**Step 3 — Build 33-byte field table** (offset, source_in_this, byte_count, source_field_name_guess).

**Step 4 — Cross-reference with FRzActionInfo / UE4 reflection** to name fields.

**Step 5 — Validate against player_track.csv + entity_tracks.csv.**

**Step 6 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 300 MB.** Disasm of 3 methods × ≤500 insns is bounded.
- **NO new memdump. NO Frida. NO live device. NO new shards needed.**
- **HARD output cap**: artifact ≤400 lines.
- **One Codex turn budget: ≤25 min wall time.** Single-turn completion (write all of tasks 1-7 in one turn, then set fact + DONE).
- The rz-typename-binding worker already disassembled `FRzMoveRq::Serialize` (37B) and `FRzMoveBr::Serialize` (33B) for the size match. Reuse its `rz_binding_build.py` extraction logic if it exposes a re-runnable function.
- Cross-pollination memory: [[project_dark_december_wire_framing_plus8]] documents the +8 outer frame model; [[project_dark_december_wire_decoder]] documents the adjacent-XOR cipher.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-frzmove-br-decode/` (branch `frzmove-br-decode`).
- Memdump dynsym shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin`
- Memdump .text shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (base VA `0x6ce4bd4000`)
- RZ binding artifact (PRIMARY REFERENCE): `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md`
- RZ binding extraction helper: `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_build.py`
- RZ wire format (vtable model): `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md`
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_s2c.tcpstream.bin`
- Baseline (validation target): `/home/sdancer/orchestrator/darkdec_output_streams/{player_track.csv, entity_tracks.csv}`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py` (Python) and `/home/sdancer/dark-december-rust-decoder/src/lib.rs` (Rust)
- success-fact key: `dark_december_frzmove_br_33_bytes_named` (a)
- block-fact keys: `dark_december_frzmove_br_3_variants_named` (b), `dark_december_frzmove_br_encoded_fields_<n>_named` (c)
