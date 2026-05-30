# sp+0x968 writer cluster (snapshot path only)

**Confirmed**: cycle ~580, 2026-05-02

## ⚠️ This path is for the snapshot-replay binary, NOT live game
See [walls/snapshot-replay-path.md](../../walls/snapshot-replay-path.md) for why.

## ELF location
- Lib `libnmsssa.so` offsets `0x2d8cd4..0x2d8d14`
- Live PC at base `0x7861e02000` → `0x78620dad04..0x78620dad14`

## Stores at the cluster
```asm
0x2d8cd4: str w28, [sp, #0x930]
0x2d8cd8: str w9,  [sp, #0x934]
...
0x2d8d04: str w16, [sp, #0x968]   ; the byte we want
0x2d8d08: str w18, [sp, #0x96c]
0x2d8d0c: str w3,  [sp, #0x974]
0x2d8d10: str w5,  [sp, #0x978]
0x2d8d14: str w7,  [sp, #0x97c]
```

## Source registers — butterfly recombination, not direct callee return
```
w16 = old[sp+0x968] - old[sp+0x928]
w18 = old[sp+0x96c] - old[sp+0x932]
w3  = old[sp+0x974] - old[sp+0x940]
w5  = old[sp+0x978] - old[sp+0x944]
```

## Upstream helpers feeding the lanes
- `0x2def54` — Ed25519 fe_mul (called many times in this phase)
- `0x2e01e4` — limb materializer/normalizer (calls `0x2e0684` for 0x28-sized limb copies)
- Stack seeding at `0x2d82a4..0x2d82e0` populates `[sp+0x230..0x250]` then `[sp+0x390..0x3af]` etc

## What feeds into the SHA-256 (frag1) at the end
After the writer cluster fills `sp+0x968..0x9a7`, the 64B block is hashed via single-block SHA-256 compress, output bytes are bswap32-per-word, digest[4..28] is the frag1 cert (24B = 48 hex chars).

## Donor reference
`cert-rust-repro/donor_session_2026-04-29.json` field `sp_0x968_block_hex64` is the working reference for this path — yields `641d96f7c8a570dcf1960b205645d13dd96e059bb9426ac0` for 7BDA via the Phase 1 reproducer.
