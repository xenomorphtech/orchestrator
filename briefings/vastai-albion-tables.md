# vastai-albion-tables — V2: live capture → quest-goal events

## Role & workdir
Codex worker, workdir `/home/sdancer/vastai-albion-tables`. **V1 done** (commit `ba9d48d` ALBION_TABLES_DONE): pcap transport-decode + 40 BattlEye-wrapped code=1 messages via SAT Protocol18. **V2 NEW** (this briefing): ingest **live** gameplay traffic from a continuous tcpdump capture, decode, correlate with ao-bin-dumps quest tables, emit **quest-goal events** to feed the minimap.

## User directive (verbatim, 2026-05-19 ~18:13)
> "resume working on log packets with another worker and decoding them, correlate with quest tables, identify quest goals, make a minimap"

## Current substrate
- Sonnet is unpaused (cycle 2333) and is playing the game right now — login → char-select → in-world → quests/move/fight. Each gameplay action emits Photon UDP 5055 packets.
- A rolling tcpdump is running on the vast.ai box (ssh ssh8.vast.ai port 14838, user root): `tcpdump -i any -U -C 100 -W 8 -w /tmp/albion_live.pcap 'udp port 5055'`. Rolls every 100MB, keeps 8 files. PID currently 3163470.
- Your existing `photon-decoder-rs/` decodes Photon transport cleanly (1098/1098 on the old pcap). It has `--jsonl` output mode (added cycle 2277).
- ao-bin-dumps quest data is at https://github.com/ao-data/ao-bin-dumps (use the JSON files: questsnpcs.json, items.json, mobs.json, world.json). Read via WebFetch only — DO NOT clone the repo.

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-live-quest-event-stream`

## Success criteria
- Pull pcap chunks from the remote in a loop (scp every ~30s). Decode via the Rust decoder, emit quest-relevant events.
- **Quest-goal events**: each detected quest interaction emits a JSON line: `{"t":<ts>, "kind":"quest_state", "quest_id":<id>, "quest_name":"<from ao-bin>", "step":"<accepted|in_progress|complete>", "goal":"<kill X mob | deliver Y to Z | explore region R>", "target_pos":[x,y]|null, "target_npc":"<name>|null", "target_mob_type":"<id>|null"}`.
- Goal-stream sink: append events to `/tmp/albion_events.jsonl` (orchestrator + minimap consume from here).
- Fact `albion_quest_events_streaming_<date>` set with: events captured, quests identified, opcode coverage.
- Verdict at `analysis/albion_quest_events_verdict.md` (≤180 lines). Final line `ALBION_QUEST_EVENTS_DONE`.

## Concrete tasks (do in order)

1. **Validate the live capture is producing data**: `ssh root@ssh8.vast.ai -p 14838 "ls -la /tmp/albion_live*.pcap 2>&1"`. Once non-empty, scp the latest rolling file: `scp -i ~/.ssh/id_ed25519 -P 14838 root@ssh8.vast.ai:/tmp/albion_live.pcap /tmp/live_in.pcap`. If sonnet hasn't entered the game yet, wait — gameplay traffic starts after CHAR_SELECT → INTO_WORLD.

2. **Decode the new pcap**: `cargo run --manifest-path photon-decoder-rs/Cargo.toml -- --pcap /tmp/live_in.pcap --jsonl > /tmp/live_decoded.jsonl`. Compare to the old `albion_combat.pcap` decode — gameplay traffic should have MUCH richer code variety than the 40-code=1 BattlEye-only set.

3. **Pull ao-bin-dumps quest tables** via WebFetch (NOT clone):
   - `https://raw.githubusercontent.com/ao-data/ao-bin-dumps/master/formatted/quests.json` (or questsnpcs.json)
   - `https://raw.githubusercontent.com/ao-data/ao-bin-dumps/master/formatted/items.json`
   - `https://raw.githubusercontent.com/ao-data/ao-bin-dumps/master/formatted/mobs.json`
   Cache locally as `analysis/aobin_quests.json`, `analysis/aobin_items.json`, `analysis/aobin_mobs.json`.

4. **Build the quest-event mapper** at `scripts/quest_event_mapper.py` (~100 LoC):
   - Read JSONL from stdin (output of the decoder).
   - For each Photon event with code matching a quest opcode (per SAT EventCodes.cs reference URLs in v1 verdict), look up:
     - quest_id from params → quest_name + objective text from aobin_quests.json
     - target NPC name from params → npc location from aobin world data
     - mob type → human name from aobin_mobs.json
   - Emit `{"t","kind":"quest_state","quest_id","quest_name","step","goal","target_pos","target_npc","target_mob_type"}`.
   - Stream to stdout AND append to `/tmp/albion_events.jsonl`.

5. **Wire to live capture** (loop): write `scripts/live_ingest.sh` that every 30s:
   - scp the rolling pcap from vast.ai
   - re-decode just the NEW portion (track byte offset across runs)
   - pipe through quest_event_mapper.py → append to /tmp/albion_events.jsonl
   Run as a foreground service for now (you don't have to make it durable — orchestrator will set up systemd if it sticks).

6. **Verdict + fact**: `analysis/albion_quest_events_verdict.md` (final line `ALBION_QUEST_EVENTS_DONE`). Write fact key+value to `analysis/fact_to_set.txt`. Single commit on branch `codex-albion-tables` named `Add live-capture quest event stream`.

## Constraints
- Do NOT touch sonnet or the live Albion process.
- Do NOT install Albion-game files locally — only the pcap + ao-bin JSON tables.
- Memory budget: stay under 1 GB RSS (per [[bulk-enumeration-needs-explicit-memory-budget]]). pcaps can be large — stream, don't slurp.
- SSH key: `~/.ssh/id_ed25519`. Port 14838. User root.
- The 18:10 historical pcap (analysis/albion_combat.pcap) is BattlEye-only — your v2 deliverable depends entirely on the new live captures. If sonnet hasn't generated gameplay traffic yet, do prep work (step 3 = fetch ao-bin tables, step 4 = build mapper) and wait.
- ⚠️ Photon RELIABLE message ≠ quest event automatically. You'll need to scan the SAT EventCodes.cs (URL in v1 verdict) to identify which `code` values correspond to QuestProgress / TakeItem / Move / etc. Confirm against in-pcap evidence.

## Memory references
- `[[albion-tables-decoded-2026-05-19]]` — v1 fact.
- `[[albion-minimap-ws-bridge-v1-shipped-2026-05-19]]` — sibling worker's WS endpoint.
- v1 verdict: `/home/sdancer/vastai-albion-tables/analysis/albion_tables_verdict.md`.
- SAT decoder refs (already in v1 verdict): photon_spectator, PhotonPackageParser, AlbionOnline-StatisticsAnalysis Protocol18.
