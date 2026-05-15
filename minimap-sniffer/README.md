# Dark December live minimap sniffer

Windows live minimap for Dark December traffic on TCP port `10001`.

The sniffer captures packets through Npcap/WinPcap-compatible pcap, reassembles
the TCP payload streams by sequence number, decodes the known Dark December
movement frames, and serves a local browser minimap over HTTP/SSE.

## Windows setup

1. Install Rust.
2. Install Npcap and enable **WinPcap API-compatible Mode**.
3. If the Rust `pcap` crate cannot find the import libraries, install the Npcap
   SDK and set `LIBPCAP_LIBDIR` to its `Lib\x64` directory.

Example:

```powershell
$env:LIBPCAP_LIBDIR = "C:\Npcap-SDK\Lib\x64"
cargo run --release -- --list-devices
cargo run --release -- --iface "Ethernet" --port 10001
```

Then open:

```text
http://127.0.0.1:17891/
```

## Offline replay

The offline mode reads the committed TCP streams, so it can exercise the decoder
and minimap without a live capture:

```powershell
cargo run --release --no-default-features -- --offline-stream-dir ..\streams\first_quest
```

## Useful flags

- `--list-devices`: print pcap devices and exit.
- `--iface <name>`: choose a capture adapter by exact name or substring match
  against name/description.
- `--port <port>`: game TCP port, default `10001`.
- `--bind <addr:port>`: local web server bind, default `127.0.0.1:17891`.
- `--offline-stream-dir <path>`: replay from `first_quest_s2c.tcpstream.bin`.
- `--replay-ms <n>`: sleep between offline decoded updates.

## Notes

Start the sniffer before logging into the game when possible. If capture starts
mid-connection, the reassembler attempts to resync on plausible app-frame length
prefixes, but the cleanest live decode begins before the TCP stream starts.
