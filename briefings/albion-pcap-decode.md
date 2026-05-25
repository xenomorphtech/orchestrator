# albion-pcap-decode — parse Photon pcap using community decoder references

## Role & workdir
Decode the live Albion Online pcap (`/tmp/albion_live.pcap0` on the vast.ai instance, ~34 MB and growing) into structured Photon events using existing community decoders as reference. Output a per-event stream we can feed into game-state tracking + pathfinding telemetry.

**Workdir**: `/home/sdancer/albion-pcap-decode`

## Current goal / sub-goal
- **goal_key**: `vastai_albion_web`
- **sub_goal_key**: `albion-pcap-photon-decode`

## Hypothesis & falsification
**Hypothesis**: Albion uses Photon UDP (server `5055-5057` range) with the standard Photon binary protocol. Existing community decoders (`broderickhyman/albiondata-client` in Go, `albion-online-data`, `aoaddons-decoder`) have already mapped event/operation codes for: player position updates, NPC events, chat, mob events, inventory/equip, gather/resource events. We can port (or directly use) those code tables to decode our pcap into a structured event stream.

**Falsification**: Photon traffic in `/tmp/albion_live.pcap0` is encrypted/obfuscated beyond what public decoders handle, OR none of the community repos cover the current game version's op codes.

## Success criteria
1. **`/home/sdancer/albion-pcap-decode/analysis/op_code_tables.md`** — consolidated map of Photon op codes + event codes + parameter signatures, cross-referenced from at least 2 community decoders (cite repo + file + commit).
2. **`/home/sdancer/albion-pcap-decode/scripts/decode_pcap.py`** (or `.rs`) — reads `albion_live.pcap0`, emits one JSON line per decoded Photon event with fields `{t, dir, op_code, event_code, params}`.
3. **`/home/sdancer/albion-pcap-decode/analysis/events_decoded.jsonl`** — at least the FIRST 1000 Photon events decoded from a portion of the pcap, with at least these event classes represented: login/auth (op 2), new-character-position (event 1 or similar — server tick), NPC dialogue, equip/inventory change, gather event.
4. **`/home/sdancer/albion-pcap-decode/analysis/player_position_stream.csv`** — char_id, t, x, z extracted from the position events alone. This is the deliverable that unblocks pathfinding integration.
5. Fact: `albion_pcap_decoded_<N>_events_<date>` on success, OR `albion_pcap_decode_blocked_<reason>` on falsification.

## Context

The current `vastai-albion-sonnet` worker drives Albion via minimap vision. The pcap `/tmp/albion_live.pcap0` was capturing during: login attempts, character creation (Vyqmzsni), tutorial play, NPC dialogues ("First Steps" + "Information is Key"), first combat, sword/shield equip, stone/wood gathering. So the pcap has a rich variety of event classes — confirms it's not just keep-alive traffic.

A sister worker (`albion-navmesh`) is in parallel extracting navmesh data from Unity assets. The two outputs combine: navmesh = where you CAN walk; pcap-decoded position stream = where you ARE. Together they replace the random-walk loop.

## Next 2-3 concrete tasks (in order)

1. **Pull the pcap locally + survey community decoders.** From the vast.ai box: `scp -P 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai:/tmp/albion_live.pcap0 /home/sdancer/albion-pcap-decode/captures/`. Then `git clone` (read-only): `broderickhyman/albiondata-client`, `tijoko/albion-radar-deatheye-2pc` or similar Photon-decoder repo (search github for "albion online" + "photon"), and `vincentr/albion-online-data` if it exists. Read their op-code tables; consolidate into `analysis/op_code_tables.md`. **Do NOT** copy GPL/AGPL code verbatim — extract op-code constants and packet structure docs, then write fresh code.

2. **Write `decode_pcap.py`**. Layers: PCAP→Ethernet→IPv4→UDP→Photon. Photon framing: per-packet header (peer-id, crc, command count) + per-command (cmd-type, reliability, sequence, payload). Within reliable-payload: opType + opCode/eventCode + params (typed). Use community references for byte layouts. Hard RSS cap 512 MB. Streaming decode (don't buffer the whole pcap). Emit JSONL.

3. **Extract position stream**. Filter `events_decoded.jsonl` to position-update events (whatever the community decoders flag as `Move` or `NewCharacter` or `CharacterEquipmentChanged` with position fields). Output `player_position_stream.csv` with `char_id, t, x, z`. Identify which char_id corresponds to our player Vyqmzsni (cross-reference with login/create-character events at the start of the pcap).

## Constraints & gotchas

- **NO modification of the game client process.** Read-only pcap analysis only. Pure offline work. Memory: `[[albion-client-wedge-class]]`.
- **Hard RSS cap 512 MB** on decode_pcap.py — must stream the pcap, not load it all in memory. The pcap is growing (live tcpdump on remote), so keep your local snapshot fixed.
- **License hygiene**: community Albion decoders are typically GPL/AGPL. Use them as READ-ONLY references for op codes + packet structure. Write your own decoder in clean Apache/MIT-style code. Cite sources in `op_code_tables.md`.
- **Do not commit pcap files**: `/home/sdancer/albion-pcap-decode/captures/*` should be `.gitignore`d — it may contain auth tokens.
- **Encryption check first**: if the first decoded packet looks encrypted (high entropy, no recognizable Photon header), surface that as a critical fact immediately rather than spending cycles forcing it.

## Relevant files / references

- Live pcap on remote: `/tmp/albion_live.pcap0` on `ssh8.vast.ai:14838` (growing, capture still running)
- Photon protocol primer: Photon Engine docs, also reverse-engineered in `broderickhyman/albiondata-client/blob/master/photon/` and `OPCodes/`
- Community starting points (NOT exhaustive — search yourself):
  - `https://github.com/broderickhyman/albiondata-client`
  - `https://github.com/PhotonEnchantments/SharpPcapDecoder` (older)
  - `https://github.com/Triky313/AlbionOnline-StatisticsAnalysis` (C#, has op code tables)
- Player char name to find in pcap: **Vyqmzsni** (account email `albionbo2r5mnh@inboxkitten.com`)
- Memory pointers: `[[albion-substrate]]`, `[[albion-client-wedge-class]]`, `[[check-existing-decoder-before-re]]` (esp. relevant — sweep existing community work FIRST)

## Reporting cadence

Append a status line to `/home/sdancer/orchestrator/analysis/talk_channels/albion.jsonl` every 5 min (`{"ts":"<ISO>","from":"albion-pcap-decode","text":"<short status>"}`).
