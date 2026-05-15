# Dark December packet decoder

Utilities for decoding a Dark December TCP capture on port `10001` and rendering an
interactive minimap of the player and entity/monster position candidates.

## What this decodes

From the first quest capture:

- Traffic is TCP on port `10001`.
- Application frames use a 4-byte little-endian total length prefix.
- Bytes 4 and 5 are a small channel/header pair, commonly `01 00` or `01 01`.
- The useful body decodes with adjacent XOR over bytes after that 2-byte channel:

```text
decoded[i] = raw_body[i] ^ raw_body[i + 1]
```

The current extractor detects:

- Player position candidate: S2C 41-byte frames whose decoded body starts
  `12 02 60 6d`.
- Entity/monster movement candidates: S2C 41-byte frames shaped as
  `12 <id> 86 01 00 00 00 46 11`.
- Coordinates are little-endian float32 values at decoded offsets `9` and `17`.
- An orientation-like float is at decoded offset `25`.

## Usage

Put a `.pcapng` capture next to the script, or pass its full path:

```powershell
python darkdec_decoder.py "CAPTURA DARK DECEMBER PRIMERA QUEST.pcapng" --out darkdec_output
```

The script writes:

- `darkdec_output/minimap.html`
- `darkdec_output/decode_report.md`
- `darkdec_output/player_track.csv`
- `darkdec_output/entity_tracks.csv`
- `darkdec_output/entities_summary.csv`
- `darkdec_output/active_end_entities.csv`
- `darkdec_output/frame_lengths.csv`
- `darkdec_output/decoded_frame_samples.txt`

Open `minimap.html` in a browser and use the timeline slider to inspect movement.

## Live minimap sniffer

`minimap-sniffer/` contains a Rust + egui minimap for live Windows captures
through Npcap/WinPcap-compatible pcap. A browser view remains available for
debugging.

```powershell
cd minimap-sniffer
powershell -ExecutionPolicy Bypass -File .\run-egui.ps1 -ListDevices
powershell -ExecutionPolicy Bypass -File .\run-egui.ps1 -Iface "Realtek" -Port 10001
```

It can also replay the committed TCP stream export:

```powershell
cargo run --release --no-default-features --features egui -- --offline-stream-dir ..\streams\first_quest
```

## Privacy note

Raw captures are intentionally ignored by `.gitignore`. They may contain IPs,
session data, account-adjacent metadata, or gameplay traces. Keep captures local
unless you explicitly want to publish them.
