# Stream-based decode (cycle 384)

Source: streams/first_quest/{c2s,s2c}.tcpstream.bin (pre-reassembled TCP).
Algorithm: 4B LE length + 2B channel + adjacent-XOR body decode (darkdec_decoder.py).

- App frames decoded: **47,987** (C2S 12,727, S2C 35,260)
- Player updates: **6,013**
- Distinct entity IDs: **71**
- Entity movement updates: **17,555**

Player first: x=9741.00 z=872.15
Player last:  x=-8892.58 z=3363.18

Matches docs/first_quest_decode_report.md baseline exactly.

## Top-10 most-active entities

| id | updates | last_x | last_z |
|---|---:|---:|---:|
| 0xb7 | 737 | -8136.66 | 3253.24 |
| 0xa1 | 660 | -4046.21 | 2753.24 |
| 0xc5 | 653 | -10321.75 | 2056.06 |
| 0xa0 | 590 | -4329.6 | 2753.24 |
| 0xc4 | 579 | -10417.73 | 2056.06 |
| 0xba | 517 | -6519.86 | 2753.24 |
| 0xb4 | 513 | -6734.79 | 2753.24 |
| 0xcd | 508 | -10778.05 | 2065.88 |
| 0xb0 | 490 | -571.42 | 2066.86 |
| 0xb2 | 473 | -9114.71 | 3253.44 |
