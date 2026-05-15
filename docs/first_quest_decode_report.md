# First quest capture decode report

Capture analyzed locally:

- Packets in pcapng: 36,513
- TCP payload segments on port 10001: 23,830
- App frames decoded: 47,987
- Payload bytes C2S: 490,870
- Payload bytes S2C: 1,670,631
- Player updates decoded: 6,013
- Entity IDs decoded: 71
- Entity movement updates decoded: 17,555

## Active near capture end

Entities whose last decoded update is within 5 seconds of the final decoded
update, sorted by distance from the last player position.

| ID | Last t | X | Z | Distance from player |
|---|---:|---:|---:|---:|
| 0xb6 | 1642.47 | -8766.14 | 3264.20 | 160.58 |
| 0xb7 | 1642.32 | -8136.66 | 3253.24 | 763.87 |
| 0xa7 | 1641.45 | -7724.86 | 2753.24 | 1317.42 |
| 0xb4 | 1642.32 | -6734.79 | 2753.24 | 2242.34 |
| 0xb5 | 1638.72 | -6551.13 | 2753.24 | 2419.59 |
| 0xba | 1640.12 | -6519.86 | 2753.24 | 2449.86 |
| 0xbb | 1640.23 | -6303.95 | 2753.24 | 2659.52 |
| 0xa0 | 1642.47 | -4329.60 | 2753.24 | 4603.56 |
| 0xb8 | 1642.47 | -4160.63 | 2753.24 | 4771.10 |
| 0xa2 | 1642.47 | -4117.68 | 2753.24 | 4813.70 |
| 0xa1 | 1642.47 | -4046.21 | 2753.24 | 4884.60 |

## Notes

The IDs above are entity/monster candidates from network movement updates. This
capture alone does not prove the exact entity class, so NPCs, summons, props, or
projectiles may share the same update shape until cross-checked against more
captures or client-side identifiers.
