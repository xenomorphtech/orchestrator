# rz-typename-binding — Bind decoded length classes to specific RZ typenames

## Role & workdir
Pure-analysis worker. Workdir: `/home/sdancer/dark-december-rz-typename-binding` (worktree of `/home/sdancer/darkdecember/`, branch `rz-typename-binding`).

## Current goal / sub-goal
- **goal_key**: `dark_december_rz_typename_binding` (new — enabled by cycle 406 symbol catalog)
- **sub_goal_key**: `bind-length-classes-to-rz-cppclass-symbols`

## Why this turn exists
Cycle 406 extracted **822 distinct `CL_GS::F*Rq` / `GS_CL::F*Rp` packet typenames** from the libUnreal.so dump. Cycle 407 + 408 named ~17 wire-format length classes across both directions. The missing bridge is: **for each wire class, which RZ typename produces frames of exactly that length, with exactly that decoded prefix?**

The `dark-december-libue4-rz-protocol` cycle proved the IRzBuffer vtable model (FString=+0x50/+0x58, raw=+0x60/+0x70, nested=+0x68/+0x78, TArray/TMap with `int16` count prefix, all little-endian). For fixed-size messages this lets us **predict the wire length** from the C++ struct field types — and for variable-size messages, predict the lower bound.

## Hypothesis
Three predictions, in increasing strength:

1. The 23 "fire-and-forget" CL_GS Rqs (FRzMove, FRzHeartBeat, FRzCheckLatency, FRzMoveDuringSkill, FRzControlProjectile, FRzStand, FRzSyncLocation, FRzApplyProjectile, FRzHitSpawn[Ex], FRzInteraction[Cancel], FRzDeath, FRzSightEnter, FRzSkillLoop, FRzSkillMoveSync, FRzSummonCancel, FRzTransferBuff, FRzChannelSkillReserve, FRzCharacterLookVisibleChange, FRzCoContentGiveUpAccept, FRzBattlePassLevelBuy, FRzZodiacWarInfo) cover the high-frequency C2S length classes (45, 61, 10, plus the larger minor-class buckets) one-to-one, with the most-frequent CL_GS class binding to FRzMove.
2. The S2C `player_position_update_family` (len=41, 23,570 frames) is a UE4 NetGUID-prefixed multicast (NOT in `GS_CL::F*Rp`), explaining why no `Rp` symbol matches; the leading `12 02 60 6d` is a stable NetGUID + replication header.
3. The matching response classes for non-broadcast actions (`GS_CL::FRzAcceptQuestRp`, `FRzAvatarLevelUpRp`, `FRzInventoryItemMoveRp`, etc.) can be identified by their **expected serialized length** computed from disassembling the corresponding `Serialize` method at the VA listed in `rz_protocol_2026-05-15.md`'s symbol table.

## Falsification (3 outcomes)
- (a) **≥5 length classes are bound to specific RZ typenames** with a stated mechanism (predicted Serialize size + signature byte match) → SUCCESS. Fact: `dark_december_rz_binding_<n>_classes_bound`.
- (b) **No length class binds cleanly to any RZ Rq/Rp** (because the wire framing prepends an opcode/header that disrupts the predicted size) → frame format is opcode-prefixed, not direct Serialize output. Fact: `dark_december_rz_wire_has_opcode_prefix`.
- (c) **Partial binding** (1-4 classes bound) → name what binds and why the rest don't; characterize the wire framing model. Fact: `dark_december_rz_partial_binding_<n>`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md` with:

1. **Predicted Serialize size table**: for each of the 23 fire-and-forget CL_GS Rqs (and the top ~30 other Rqs by name frequency in dynsym), compute or estimate the serialized wire size from:
   - The IRzBuffer vtable model in `rz_protocol_2026-05-15.md`
   - The struct layout inferred from disassembly OR from analogous structs in that cycle
   - For unknown structs, mark "unknown" and skip
2. **Wire-format prefix budget**: the actual wire frame is `[4B LE length][1B channel=01][1B channel2][raw_body]` with adjacent-XOR collapse on raw_body. So `raw_body_len = wire_len - 6` and `decoded_dec_len = wire_len - 7`. Account for this overhead when comparing predicted-Serialize-size to observed-length-class.
3. **Bindings table**: for each named length class (from `s2c_inventory_2026-05-15.md` + `c2s_decode_2026-05-15.md` + `c2s_minor_classes_2026-05-15.md`):
   - Wire length class
   - Decoded prefix
   - Best-fit RZ typename(s) with predicted Serialize size match
   - Confidence (high if size matches exactly + signature prefix matches a unique typename; medium if size matches but multiple candidates; low if only the suffix `Rq` direction matches)
4. **Specifically resolve**: `len=10 ack` → `FRzHeartBeatRq` or `FRzCheckLatencyRq` (try to disambiguate by **disassembling both Serialize methods** and computing exact sizes); `len=45` → `FRzMoveRq`; `len=61` → `FRzMoveDuringSkillRq` or `FRzControlProjectileRq` (cycle-407 found a 16-byte insertion at dec[17:33] — that's the skill_id + targetref payload — disambiguate).
5. **Unbound list**: any RZ typename in the catalog whose predicted size doesn't match any observed length class — these are messages that don't appear in this capture (lobby/login/quest-specific).
6. Verdict (a)/(b)/(c) + closing fact via `harness fact-set`.

Print `RZ_BINDING_DONE` on the final line.

## Execution flow

**Step 1 — Load typename catalog + length classes:**
```python
from pathlib import Path
typenames = Path('/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt').read_text().splitlines()
# Build maps: cl_gs_rqs (421), gs_cl_rps (400)
```

**Step 2 — Compute observed length classes** by re-running darkdec_decoder.py on both streams and bucketing.

**Step 3 — For each of the 23 fire-and-forget Rqs**, find the Serialize method in the dump by symbol search:
```bash
strings -n 12 /home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin | \
  grep -oE '_ZN5CL_GS[0-9]+F[A-Z][a-zA-Z]+(Rq|Rp)[0-9]+SerializeE[^[:space:]]*'
# Find the st_value of Serialize for each typename, then disassemble it from the .text shard.
```
For each Serialize method's disassembly, count IRzBuffer vtable slot calls and the immediate `w2` values to derive total serialized byte count.

**Step 4 — Wire-overhead model check**: predicted_wire_len(typename) = serialized_size + 7 (4B length + 2B channel + 1B encoding-collapse) → compare against observed length classes.

**Step 5 — Bind matches**, record confidence.

**Step 6 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 500 MB.** Disasm work for ~50 small methods is bounded; do NOT do full-shard disasm.
- **NO new disasm beyond the named Serialize methods**. Do not re-disassemble the entire .text.
- **NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget: ≤25 min wall time.**
- Use the `aarch64-linux-gnu-objdump -d` or `capstone`-based per-region disasm. The .text shard is at `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin`, base VA `0x6ce4bd4000`.
- For each Serialize method, the st_value is in dynsym. The mapping: `st_value` is **image-relative**; live VA = `st_value + 0x6cdd243000`; offset into the .text shard = `live_va - 0x6ce4bd4000`.
- **Adjacent-XOR collapse subtlety**: `decoded_dec_len = raw_body_len - 1` because `dec[i] = body[i] ^ body[i+1]` for i in [0, body_len-2]. So if Serialize writes N bytes, those N bytes appear at `raw_body[0:N]`, and the decoded buffer is N-1 bytes long. Account for this.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-rz-typename-binding/` (branch `rz-typename-binding`).
- RZ symbol catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt` (822 typenames)
- RZ wire format (vtable model): `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md`
- Memdump shards: `/home/sdancer/dark-december-libue4-memdump/memdump/{6cdd243000.bin,6ce4bd4000.bin,6ceac63000.bin}`
- Memdump model: `/home/sdancer/dark-december-libue4-memdump/analysis/libue4_memdump_2026-05-15.md`
- Length classes from sibling cycles: C2S (decoder + len45 fields + len61 + minor-classes) and S2C (inventory) — paths listed in `c2s-s2c-join.md` briefing.
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- success-fact key: `dark_december_rz_binding_<n>_classes_bound` (a)
- block-fact keys: `dark_december_rz_wire_has_opcode_prefix` (b), `dark_december_rz_partial_binding_<n>` (c)
