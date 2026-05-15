# const32b-verify — numerically reproduce CONST_32B

**Goal:** `nmss_const_32b_numerical_repro` (new campaign). Metric: pure-code reproduces CONST_32B `04210d42 9fe12f54 569d0314 d65f8f10 1910d16e 5d21410f 978ebfba 9399abf8` — binary 0/1.

## Role & workdir

Codex worker. Workdir: `/home/sdancer/nmss-emu-const32b-verify`.

## Context from prior campaign (closed 8/8 2026-05-12T18:53Z)

`const32b-producer-disasm` recovered the structural producer chain in module 9781e236:

```
seed = 0x00 * 32                                  (zero-init at PC 0x78c68962a8, addr sp+0x7b0)
d0  = SHA256(source_object+0x210 || seed)         finalizer 0x78c68a69d8
d1  = SHA256(source_object+0x1f8 || d0)           finalizer 0x78c68ae010
CONST_32B = SHA256(source_object+0x210 || d1)     finalizer 0x78c68b2388
```

Capture-time field values (in `trampoline_proc_memdump_5558`):
- `source+0x210`: 32-byte heap string ASCII `F61DFB2DA2C94AA1B67CAFCD51DA7E85`
- `source+0x1f8`: inline 20-byte `com.netmarble.thered`

I verified pure-Python that with THOSE capture-time field values, the 3-iter chain does NOT produce CONST_32B. This means at CONST_32B-production time, one or both fields held different values.

Closing artifact: `/home/sdancer/nmss-emu-const32b-producer/analysis/const32b_input_provenance.md` (3967B). Read this first.

## Hypothesis space (rank by likelihood)

**H1**: `source+0x210` at production time was a DIFFERENT 32-char ASCII hex string. F61DFB... might be a session/login-derived value that overwrites whatever seeded CONST_32B. The original could be a baked rodata constant.
  - Falsification: no 32-byte ASCII hex string in rodata of libnmsssa.so / module 9781e236 / libUnreal.so produces CONST_32B when run through the chain with `com.netmarble.thered` as field +0x1f8.

**H2**: `source+0x1f8` at production time was NOT `com.netmarble.thered`. Could be a different package identifier or a binary blob.
  - Falsification: with F61DFB... fixed, no plausible 20-byte string produces CONST_32B.

**H3**: The chain has MORE than 3 iterations, or different ordering. Worker may have undercounted.
  - Falsification: 4–8 iteration variants with alternating field choices all miss.

**H4**: The "fields" are interpreted differently — e.g. `source+0x210` is hex-decoded (16 bytes) instead of ASCII (32 bytes); or the inline string at +0x1f8 includes the tag byte and trailing nulls.
  - Falsification: all interpretation variants fail.

## Concrete tasks (ordered)

1. **Write skeleton artifact** at `/home/sdancer/nmss-emu-const32b-verify/analysis/const32b_verify.md` — section headers only.

2. **Run exhaustive interpretation sweep** in pure Python: try all combinations of
   - source+0x210: {ASCII 32B as given, hex-decoded 16B, capture-time bytes, padded variants}
   - source+0x1f8: {`com.netmarble.thered` 20B, with tag prefix `\x28`, padded to 24B, capture-time inline 24B}
   - iteration count: 1, 2, 3, 4, 5
   - prefix||digest vs digest||prefix at each iter
   - seed: {0x00*32, 0xff*32, empty}
   Save full results to `/tmp/const32b_sweep.jsonl`. If H1+H2+H3+H4 all miss with capture-time fields, that confirms +0x210 changed at production time.

3. **Scan rodata for 32-char ASCII hex strings** in module 9781e236 (`/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78c6896000.bin`) AND the cert-region module CFF3FAD10 AND libnmsssa.so. Pattern: `[0-9A-F]{32}` aligned. Test each as a candidate prefix1 with `com.netmarble.thered` as prefix2.

4. **If still no match**, scan the cert-rust-repro crate (`/home/sdancer/nmss-emu/cert-rust-repro/`) for the actual CONST_32B baked value to verify it's `04210d42...` as documented.

5. **On success**, update `/home/sdancer/nmss-emu/cert-rust-repro/` with the verified preimage (new test if appropriate), set fact `nmss_const_32b_numerical_reproduced`, write closing artifact.

## Guardrails (same as prior campaign)

- **No git commits.**
- **No Frida on libUnreal.so**.
- **No wide disasm dumps** — this campaign is mostly pure-Python crypto sweep + rodata grep; should produce KB not MB of events.jsonl.
- **Save intermediate findings to artifact incrementally** — write what worked AND what failed.
- **Stop conditions**: (a) reproduction found → CAMPAIGN CLOSE; (b) all H1–H4 falsified → save findings, recommend live HW-BP capture; (c) 2 cycles no progress → flag falsification.

## References

- Prior closing artifact: `/home/sdancer/nmss-emu-const32b-producer/analysis/const32b_input_provenance.md`
- Wiki: `/home/sdancer/nmss-emu/WIKI.md` (CONST_32B section)
- Module shard: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78c6896000.bin` (base 0x78c6896000, contains 9781e236)
- Module shard: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/78cd296000.bin` (base 0x78cd296000, contains CFF3FAD10)
- libnmsssa: `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libnmsssa.so` (if present)
- cert-rust-repro: `/home/sdancer/nmss-emu/cert-rust-repro/`
- Cross-pollination facts: `nmss_const_32b_primitive_is_3iter_sha256_chain`, `const32b_iv_materialization_at_0x78c68a2ed4`

## Operating mode

`codex_app_server`. One long turn via `harness send --wait --timeout 3600`. Expected runtime 20–40 min. Stop on H-criteria above.
