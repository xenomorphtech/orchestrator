# const32b-hwbp — TURN 2: brute-force 32-char-hex sweep (POST-RESTART)

## TURN 2 — NEW PRIMARY APPROACH

The previous turn closed inconclusive (Vector A in 2 native arenas → 0 candidates for `com.netmarble.thered`; Vector B HW-BP restart destabilized Waydroid). Waydroid recovered. New session: **PID 27342, libnmsssa base 0x6cd061e000**.

**New primary hypothesis:** the `+0x210` field is a 32-char hex string (like MAGIC32). Scan ALL writable mappings of the process for ASCII hex 32-char sequences. For each unique candidate, brute-force the SHA chain to see if any reproduces CONST_32B.

Sweep `prefix2` candidates from the XOR'd JSON template field names — see addendum.

## TURN 2 PROCEDURE (run all, no stopping)

1. **Read briefing in full including addenda**. Confirm new PID + libnmsssa base from `/proc/$(pidof com.netmarble.thered)/maps`.

2. **Dump every `rw-p` mapping** of the game process. Use `process_vm_readv` (su 0 should have CAP_SYS_PTRACE) or `dd if=/proc/PID/mem ...` via `su 0`. Skip read-only and very tiny mappings.
   - Priority regions (likely to contain source_object):
     - `[anon:dalvik-main space (region space)]` (huge ART heap)
     - `[anon:dalvik-free list large object space]` (huge ART large-object heap)
     - `[anon:scudo:primary]` and `[anon:scudo:secondary]` (native heaps)
     - `[anon:.bss]` near libnmsssa @ 0x6cd0...
     - `[anon:bionic_alloc_small_objects]`, `[anon:libc_malloc]` if present
   - Stream-dump to `/tmp/proc_dump_<region_idx>.bin`; don't put any binary content in events.jsonl.

3. **Extract 32-char ASCII hex strings** (regex `[0-9A-Fa-f]{32}`) from all dumps. Deduplicate. Save to `/tmp/const32b_hex32_candidates.txt`.

4. **Brute-force SHA chain.** For each candidate `prefix1`:
   ```python
   for p1 in candidates:
     for p2 in ['com.netmarble.thered', 'netmarbles', 'thered', 'Security',
                'rockchip rk3588_s', 'thered\x00\x00\x00...', 'I_DeviceModel',
                'NMDeviceKey', '<other plausible 20B strings>']:
       d0 = SHA256(p1 + b'\x00'*32)
       d1 = SHA256(p2 + d0)
       result = SHA256(p1 + d1)
       if result.hex() == '04210d42...9399abf8': PRINT MATCH
   ```
5. **Also try prefix2 candidates derived from the XOR'd JSON templates** (see addendum) — strings like `I_OldNMDeviceKey`, `I_UDID`, `I_PlatformADID` etc., possibly with the trailing `\x00` pad.

6. **If match found**: record candidate p1 and p2 with their source region offsets in `analysis/const32b_hwbp_capture.json`. Set fact `nmss_const_32b_numerical_reproduced`. Update `cert-rust-repro` with the verified preimage. STOP.

7. **If no match**: save full sweep results, set fact `nmss_const_32b_hex32_brute_force_exhausted_2026_05_12`, write closing artifact. STOP.

## NEW SESSION live state (TURN 2)

- adb localhost:5558 (recovered)
- PID 27342
- libnmsssa.so live base: **0x6cd061e000**
- Producer entry-ish PC (mov x27, x1): **0x6cd061e000 + 0x108790 = 0x6cd0726790**

## Notes & guardrails

- The previous Vector A only scanned native arenas (20 MB total). The **large ART managed heap is where MAGIC32 ended up via SharedPreferences XML — try those regions FIRST**.
- Don't restart the game again — Waydroid is fragile.
- Use `process_vm_readv` (or `dd` via su 0 from `/proc/PID/mem`). Reading from a running process is safe; we are NOT attaching.
- Don't dump >100KB to events.jsonl in any single call. Stream binary to `/tmp/`.
- Pure-Python SHA chain is sub-millisecond per candidate; tens of thousands of candidates is fine.



## Role & workdir

Codex worker. Workdir: `/home/sdancer/nmss-emu-const32b-hwbp`. Device available at `adb localhost:5558`. Game process running.

## Goal

`nmss_const_32b_numerical_repro` — reproduce `CONST_32B = 04210d42 9fe12f54 569d0314 d65f8f10 1910d16e 5d21410f 978ebfba 9399abf8` numerically in pure Python from device-captured field values. Metric: binary 0/1.

Success fact: `nmss_const_32b_numerical_reproduced`.

## Substrate (already confirmed available)

- **Device:** `adb localhost:5558` connected. Target process `com.netmarble.thered` PID 30844 (verify it's still alive at start; may have rotated).
- **libnmsssa.so live base (this session):** `0x6ccbc0f000` per `/proc/PID/maps` — VERIFY current base, ASLR may have changed it.
- **Module identity:** the static "9781e236" module IS `libnmsssa.so` (originally analyzed at static base 0x78c678d000).

## Recovered structural chain (from prior `nmss_const_32b_origin` campaign, 8/8 closed)

```
seed = 0x00 × 32                                    (zero-init at module+0x1092a8 → sp+0x7b0)
d0   = SHA256( source_object+0x210 || seed )        finalizer at module+0x1199d8
d1   = SHA256( source_object+0x1f8 || d0   )        finalizer at module+0x121010
CONST_32B = SHA256( source_object+0x210 || d1 )     finalizer at module+0x125388
```

Producer function entry-ish PC: `module+0x108790` (`mov x27, x1` — saves source_object pointer for the entire function).

Source-object field layout (libc++ `std::string` tagged at +0x210):
- `source+0x1f8`: **20B inline string** (capture-time: `com.netmarble.thered`)
- `source+0x210`: tag byte (low-bit = long-string mode), then padding
- `source+0x218`: 8B length (capture-time: `0x20`)
- `source+0x220`: 8B heap pointer (TBI-tagged) → 32B ASCII string (capture-time: `F61DFB2DA2C94AA1B67CAFCD51DA7E85`)

Orchestrator-side falsification: SHA chain over **capture-time** field values yields NO match (1352-variant sweep + structural variants). Production-time values must differ.

## Two attack vectors (try in order)

### Vector A: Live memory scan (try FIRST — fast, no ptrace needed)

The source-object is likely STILL in heap. If we can find a candidate, we don't need HW-BP.

1. `adb shell su 0 cat /proc/$(pidof com.netmarble.thered)/maps` to enumerate writable heap regions.
2. For each `rw-p` region (heap, anon), dump it via `adb shell su 0 dd if=/proc/PID/mem bs=4096 skip=PAGE count=N of=/sdcard/dump.bin 2>/dev/null && adb pull /sdcard/dump.bin`. Skip `/dev/`, `/system/`, etc.
3. Search dumps for the inline string `\x14com.netmarble.thered\x00` (24B with `0x14` tag) or just `com.netmarble.thered` ASCII. Each hit is a candidate source-object at offset `dump_addr - 0x1f8` (so the inline string starts at +0x1f8).
4. For each candidate:
   - Read +0x210 tag byte; if low bit set → long-string mode.
   - Read +0x218 (length) and +0x220 (TBI-stripped pointer) and dereference to get the 32B heap string.
   - Run the 3-iter SHA chain with these field values.
   - If result == `04210d42...9399abf8` → **MATCH, CAMPAIGN CLOSED**.

Use `python3` with `struct` for parsing, `hashlib` for SHA-256.

### Vector B: HW-BP capture (if A fails)

Producer fires at app init (once). Game is already past init, so vector B requires app restart:

1. Pre-arm: write a small C ptrace-attach-and-BP utility (or adapt `/home/sdancer/nmss-emu/scripts/cert_direct_ptrace_service.c` which already implements `NT_ARM_HW_BREAK` HW-BPs in `set_hw_break_state`).
2. Kill `com.netmarble.thered` via `adb shell am force-stop`.
3. Push utility to device, run as `su 0`, monitor for the new PID after `am start -n com.netmarble.thered/.MainActivity`.
4. Attach early, set HW-BP at live `module_base + 0x108790` (`mov x27, x1`).
5. On hit: PTRACE_GETREGSET NT_PRSTATUS → read x1 (source_object pointer). Then PEEKDATA at x1+0x1f8 (24B inline string) and x1+0x210..+0x228 (40B for tag+len+ptr). Then PEEKDATA at the deref'd pointer for 32B heap string.
6. Detach (or leave attached to single-step past the BP) and write captured values to `analysis/const32b_hwbp_capture.json`.
7. Run pure-Python verification.

## Guardrails

- **No git commits.**
- **NO Frida on libUnreal.so** (anticheat) — but this target is libnmsssa.so, and ptrace was used in cert campaign without anticheat triggering. ptrace+HW-BP is the established working substrate.
- **Don't dump >100KB to events.jsonl** — write large outputs to `/tmp/` files and reference paths. Heap dumps could be GBs; sample selectively.
- **Save findings incrementally** to `analysis/const32b_hwbp_capture.md` as you go.
- **Don't restart the game without checkpointing what you've already tried** — restart is expensive.

## Concrete tasks (ordered)

1. **Verify live state:** PID still alive, libnmsssa base address, executable mappings. Write skeleton artifact `analysis/const32b_hwbp_capture.md`.
2. **Vector A pilot:** sample one writable heap region (~16MB), grep for `com.netmarble.thered`, see how many candidates exist. If feasible, expand to all writable regions. Test each candidate against the chain.
3. **If Vector A produces match:** record fields, write closing artifact, set fact `nmss_const_32b_numerical_reproduced`, update cert-rust-repro tests if appropriate. STOP.
4. **If Vector A produces 0 candidates or no matches:** switch to Vector B. Adapt `cert_direct_ptrace_service.c` or write a small standalone HW-BP attach utility.
5. **Save raw captured bytes** (hex-encoded) to `analysis/const32b_hwbp_capture.json` so future verification can re-run pure-Python.

## CRITICAL ADDENDUM: XOR'd JSON templates in libnmsssa.so rodata

Per user hint + orchestrator-verified: libnmsssa.so rodata contains **single-byte XOR'd JSON templates** for security logging. Multiple keys decode to JSON templates that name the actual device-identification fields. Decoded examples (shard `78c6896000.bin`):

- **K=0xc2 @ 0x2a20be:** `body={"I_GameCode":"netmarbles","I_LogDes":[{"I_OldNMDeviceKey":"0","I_UDID":"0","I_LogId":6,...,"I_PID":%s,"I_ConnectIP":"0",...,"I_DeviceModel":"0",...,"I_PlatformADID":%s,...}]}`
- **K=0x13 @ 0x2a3479:** `body={"I_GameCode":"netmarbles","I_ConnectIP":0,"I_RequestTime":"%s","I_PID":"%s","I_PCSeq":0,"I_LogId":901,..."PlatformAdId":"%s","UdId":"%s",...,"Pkg_Name":"%s",...}`
- **K=0xec @ 0x2a3104:** similar with `I_OldNMDeviceKey`, `I_UDID`, etc.

**Implication for CONST_32B reproduction:** the source_object's `+0x1f8` (inline 20B "com.netmarble.thered") and `+0x210` (32B heap string "F61DFB..." in capture) are likely **populated from one of these JSON templates at runtime with real device values**. The CONST_32B preimage prefix at `+0x210` is most likely a 32-hex-char value from one of: `I_OldNMDeviceKey`, `I_UDID`, `I_PlatformADID`. Same shape as MAGIC32 (`I_PID = 2FCF99..1C`) — AES-encrypted device identifier.

**New Vector A1 — XOR'd JSON template scan (fast, do FIRST):**

1. Read libnmsssa.so file: `adb pull /data/app/.../lib/arm64/libnmsssa.so /tmp/libnmsssa.so` (path from `/proc/PID/maps`).
2. Single-byte XOR scan: for each K∈[1..255], search file bytes XOR'd by K for `b'{"I_'` markers. Extract decoded templates around hits.
3. **Then dump live process memory** for any 32-char hex strings (regex `[0-9A-Fa-f]{32}`) — these are the actual device-ID values populated into source_object at production.
4. For each 32-char hex candidate found, test as `prefix1` in the SHA chain:
   - `d0 = SHA256(candidate_32B || zero[32])`
   - `d1 = SHA256(<inline_20B_candidate> || d0)`
   - `CONST_32B == SHA256(candidate_32B || d1)`?
5. Try `prefix2` candidates from the inline field: `com.netmarble.thered`, `netmarbles`, `thered`, `Security`, plus any 20B string referenced near the producer.

If any combination matches → done.

To dump /proc/PID/mem you need `ptrace_attach` first (or `su 0` + a syscall-savvy reader). Use `cert_direct_ptrace_service.c` as reference, or write a tiny standalone C utility:

```c
#include <sys/uio.h>
#include <sys/ptrace.h>
ptrace(PTRACE_ATTACH, pid, 0, 0); waitpid(...);
struct iovec local = {buf, len}; struct iovec remote = {(void*)addr, len};
process_vm_readv(pid, &local, 1, &remote, 1, 0);
ptrace(PTRACE_DETACH, pid, 0, 0);
```

`process_vm_readv` doesn't even need `PTRACE_ATTACH` if running as `su 0` with `CAP_SYS_PTRACE`. Try `su 0` direct read first.

## Stop criteria

- (a) Numerical reproduction found → CAMPAIGN CLOSE, set fact, write closing artifact.
- (b) Both vectors exhausted with detailed null results → save findings, set fact `nmss_const_32b_hwbp_capture_inconclusive_2026_05_12` with the specific reason (e.g. "scanned 8GB of heap, 0 matches for com.netmarble.thered" or "ptrace attach blocked").
- (c) 2 cycles with no artifact progress → flag for orchestrator review.

## References

- Static analysis closing artifact: `/home/sdancer/nmss-emu-const32b-producer/analysis/const32b_input_provenance.md`
- HW-BP infrastructure example: `/home/sdancer/nmss-emu/scripts/cert_direct_ptrace_service.c` (search for `NT_ARM_HW_BREAK`, `set_hw_break_state`)
- Wiki: `/home/sdancer/nmss-emu/WIKI.md` (CONST_32B section)
- Cross-pollination facts: `nmss_const_32b_primitive_is_3iter_sha256_chain`, `const32b_F61DFB_is_stable_across_5_cert_vectors_2026_05_12`, `const32b_producer_called_via_blr_indirect_2026_05_12`, `const32b_naive_chain_falsified_2026_05_12`
- adb: `localhost:5558`. `su 0` works for root operations.

## Operating mode

`codex_app_server`. Long turn via `harness send --wait --timeout 7200` (up to 2h, since memory scan + ptrace may take a while). **Run all tasks 1→5 in order without stopping unless STOP criteria fire.**
