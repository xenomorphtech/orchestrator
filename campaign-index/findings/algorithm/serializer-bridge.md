# 0x2d7284 bridge serializer — exact append spec

**Confirmed**: cycle ~582, 2026-05-02

## Frame layout
At entry, original arguments are rebound:
- `x23 = arg0`, `x21 = arg1`, `x19 = arg2`, `x22 = arg3`, `x20 = arg2 + 0x20`
- Saved: `[sp+0x10] = arg0`, `[sp+0x20] = arg1`, `[sp+0x70] = arg2`

## Three append calls into SHA-512 builder (via 0x2863f8)
1. **append1** (PC 0x2d8124..0x2d812c): append `arg2[0x00..0x1f]` (32 raw bytes)
2. **append2** (PC 0x2d8134..0x2d8140): append `arg2[0x20..0x3f]` (next 32 raw bytes)
3. **append3** (PC 0x2d8144..0x2d8150): append `arg0[0..arg1]` (caller-supplied byte stream, verbatim)

## Working E2E digest input
```
digest_input = arg2[0x00..0x20] || arg2[0x20..0x40] || arg0[0..arg1]
```
No separator, no length tag, no local mutation.

Then:
- SHA-512 finalize → 64B digest
- Ed25519 `sc_reduce`
- Ed25519 signed-window `slide`

## Argument shapes
- **arg2**: flat 64-byte binary blob with Ed25519 sig canonicality:
  - `arg2[63] ≤ 0x10`
  - if `arg2[63] == 0x10`: extra check `memcmp(arg2+0x30, ..., 0x0f) == 0` AND `arg2[0x2f] < 0x14`
  - NOT a libc++ string, NOT ASCII rows
- **arg3**: separate raw 32-byte binary, consumed bytewise by arithmetic path (not appended through 0x2863f8)
- **arg0/arg1**: byte pointer + length

## Source dispatch (0x240f3c → bridge arg2)
- `0x240f3c` is reached via **method-table dispatch** (`.data.rel.ro` slot `0x39c0c0` → runtime ptr to `0x240f3c`)
- No direct `bl 0x240f3c` callers in static graph
- Indirect callers `0x246c80` and generic shim `0x24f228` both forward incoming `x1` unchanged
- `arg2` (the Ed25519-sig-shaped blob) is therefore **caller-supplied dynamic** from a higher layer — not rodata, not constructed in-frame
- Static analysis bottoms out here; live capture needed for byte-exact arg2

## To debug Rust port mismatches
Verify in this order:
1. arg2 byte order / half order (likely R||S 32+32)
2. arg0/arg1 actual message bytes + length
3. 0x284e3c finalize/truncation variant
4. 0x2d6538 packing endianness
