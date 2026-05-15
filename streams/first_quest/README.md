# First quest TCP stream export

Raw TCP payload streams extracted from the Dark December first quest capture.

Files:

- `first_quest_c2s.tcpstream.bin`: client to server payload stream.
- `first_quest_s2c.tcpstream.bin`: server to client payload stream.
- `first_quest_tcp_stream_manifest.json`: segment counts, byte counts, SHA-256 hashes, and flow metadata.

The original `.pcapng` is intentionally not committed. These streams are enough for decoder work without uploading the full capture.
