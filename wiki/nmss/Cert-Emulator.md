# NMSS Cert Emulator

## Summary

The emulator is no longer failing at the earliest control-flow gates. It now executes enough of the JIT-driven cert pipeline to produce structured, challenge-dependent outputs. The remaining mismatch is not a single missing branch or one bad field; it is a data-parity problem across JIT, owner/container state, session state, and address-bearing values that enter the hash path.

## Major Findings

### 1. Earlier failures were real, but they were not the final root cause

Earlier work found and fixed or bypassed several blockers:

- missing `0xce75c` side effects
- empty-string gates in the hash path
- mixed `jit_module.bin` vs `jit_live_flat.bin` overlays
- out-of-range GOT-like predicate slots
- stale bypasses that skipped real formatter/helper code

Those findings were useful because they got the emulator to execute more of the real path, but they did not by themselves produce parity.

### 2. The live JIT image matters

Important JIT conclusions:

- static `nmsscr` disassembly was not enough; live JIT code diverges materially
- partial overlays caused mixed-image seams
- fresh `jit_module_live_current.bin` and `jit_live_flat_current.bin` differ from older dumps

The practical consequence is:

- the emulator must be driven from the correct live code and matching live data for the same session/install

### 3. The owner object and detection container are central

The cert path is not driven by a separate manager object. The current understanding is:

- wrapper singleton resolves directly to the owner object
- owner `+0x390` points to the detection-state container
- the container vectors feed record material into the cert path

The owner fields that currently look important are:

- `+0x210`: `"FERUN"`
- `+0x288`: UDID-like value
- `+0x314`: score
- `+0x320` to `+0x338`: cert-critical scalar/code-pointer region
- `+0x340`: device id

### 4. The cert is process-dependent

This is one of the strongest findings in the entire project.

Observed behavior:

- the device produced different certs for the same challenges on different spawns
- the emulator also moved when address-bearing metadata changed

Best explanation so far:

- address-shaped values leak into the hash path
- ASLR and process layout therefore affect the cert

### 5. `_sprintf_fast` / S1 data matters, but not in the naive way first assumed

The investigation showed:

- S1 formatting includes stack/heap/JIT/libc-looking values
- those values differ between emulator and device
- some late stack-slot patch attempts had no effect

That suggests:

- not every formatted address that looks suspicious is actually consumed by the final hash
- but the cert path absolutely does incorporate process-specific address data somewhere in the feeder chain

### 6. The cert hash path is deeper than the initial SHA fast-forward

Current model:

- the SHA fast-forward logic only covers an initial address-table style hash
- the actual cert-producing path includes a 16-block SHA chain
- the real SHA input is assembled on the stack, not from the naive `x22` source used by the older fast-forward shortcut

This is why "just speed up SHA" was not enough.

## Known Device Outputs

These values are useful as anchor points, but must be interpreted carefully because certs vary by process spawn.

Examples captured from live device processes:

- 2026-04-05 spawn A:
  - `0000000000000000 -> AD4C135E981B9EE794CA9742DDBBE3B0B6C32CE7A5A2A6F7`
  - `AABBCCDDEEFF0011 -> 3DBAB9F744F6B601E62D42B80212A8DFCD8E42847FE6C6D2`
- 2026-04-05 spawn B:
  - `0000000000000000 -> DDD9031D1449CCBFA633510C438DD920BEFDB8789664192E`
  - `AABBCCDDEEFF0011 -> 990A67178283B9DC63257107C5F6E8E1A1D1968B525351D8`
- 2026-04-05 same-session capture v2:
  - `0000000000000000 -> 01162241465E4A0CE42D7170C4A252B70CF5479F7819EE34`
  - `AABBCCDDEEFF0011 -> A1F2B81F7F4B6FD7E6608F4605A9907EF47CF511858DFFFE`

The key takeaway is not the specific hex values. The key takeaway is:

- cert output changes with the process/session

## Known Emulator Outputs

Representative milestones:

- fallback-style outputs early in the project
- partial real computation like `B4C7C064...`
- later same-session-JIT test output `B45A50FB6406666B06A59F7559E52B0DA98EC931D5A8DDC7`

These show that the emulator is doing meaningful work, but still not reproducing the exact live environment.

## Current Best Explanation

The cert mismatch is best explained by a combination of:

- mixed-session data
- stale snapshot pages
- process-specific address material entering the hash path
- incomplete parity for owner/container-derived inputs

In short:

- the emulator is close enough to expose the real data dependencies, but not yet faithful enough to reproduce the live cert

## Next Steps

1. Capture one clean same-session bundle:
   - session key
   - score / sid / owner scalars
   - JIT base
   - `SHA_INPUT_192`
   - detection buffer
   - S1 args
   - cert values for at least `0000` and `AABB...`
2. Re-run emulator against only that bundle.
3. Compare the 4 known cert-feeder memcpy inputs between device and emulator.
4. Keep hooks minimal; hook-heavy debug runs were shown to perturb computation.

