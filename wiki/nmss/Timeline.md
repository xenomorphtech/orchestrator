# NMSS Timeline

This is a condensed timeline of the main NMSS cert-emulation findings recorded so far.

## 2026-04-04

### Early cert-path debugging

- Narrowed the initial failure to CFF dispatch and the missing `0xce75c` side effect that should populate `sp+0x50`.
- Found multiple guards that could skip hash production when key SSO/string slots were empty.
- Determined that several early "hash not running" issues were real, but not the final explanation for cert mismatch.

### Control-flow and image fidelity

- Confirmed that the live JIT code materially differs from static/module assumptions.
- Found that partial overlaying of live JIT windows caused mixed-image execution.
- Identified out-of-range predicate/GOT-like slots that broke CFF state decisions in the emulator.

### Descriptor / helper-chain understanding

- Moved attention from trivial output writes to descriptor/session objects and helper chains.
- Identified that descriptor objects came from the live object at `session+0x390`, not the assumed root object.

## 2026-04-05 Morning

### Owner/container structure became clearer

- Fresh JIT dumps from the current install were captured.
- The detection-state container was identified at `owner+0x390`.
- Live owner fields were captured:
  - `+0x210 = "FERUN"`
  - `+0x288 = UDID-like value`
  - `+0x314 = score`
  - `+0x340 = device id`
- Confirmed that `owner` and `manager` are the same object.

### Emulator started producing real work

- Emulator outputs moved from trivial fallbacks toward structured cert-like values.
- Partial matches showed that detection records were contributing and that the path was no longer completely bypassed.

## 2026-04-05 Midday

### Memory snapshot and address sensitivity

- A full NMSS process memory snapshot was received under `/home/sdancer/nmss/memdump/`.
- Snapshot analysis showed the dumped heap was incomplete, especially the Dalvik heap region.
- Cert-critical owner metadata narrowed heavily toward offsets `0x320` to `0x338`.
- `sprintf`-style formatting was found to leak address-shaped values into the cert path.

## 2026-04-05 Afternoon

### Device truth and process dependence

- Fresh live-device certs were captured for both `0000` and `AABBCCDDEEFF0011`.
- Shortly after, certs were shown to change across process spawns.
- This established that cert computation is process-dependent, not just challenge-dependent.

### Deeper hash-path understanding

- Captured full-device data including JIT base, `SHA_INPUT_192`, detection buffer, and S1 args.
- Established that the initial SHA fast-forward is only part of the story.
- Identified a deeper 16-block SHA chain that appears to be the real cert-producing stage.

## 2026-04-05 Evening

### Same-session capture became the main requirement

- A major divergence root cause was identified: session key and cert values had often been mixed across different sessions.
- New same-session capture scripts were prepared to collect all required values from one spawn.
- A partial same-session capture recorded session key / score / sid but crashed before cert retrieval.
- A later same-session emulator run with matched JIT data produced a concrete cert, but device comparison from the same session was still pending.

### Native harness status

- The native ARM64 harness had cleared many earlier crash families.
- It progressed past libart infinite loops and into later runtime code.
- The remaining blocker shifted to control-flow integrity, including bad returns / null-ish return addresses.

## Current Takeaway

By the end of the recorded work:

- the problem was no longer "make the cert path run"
- the problem became "reproduce the exact same-session data and address-sensitive inputs that the live process uses"

