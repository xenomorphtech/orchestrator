# cert-algo-validate — validate cert_rust_repro algorithm against captured ground-truth pairs

## Role & workdir
Algorithm-validation worker. Workdir: `/home/sdancer/nmss-emu-cert-algo-validate` (create via `git worktree add` from `/home/sdancer/nmss-emu`). Target: thered on RK3588 at `adb -s localhost:5558`.

## Goal of this turn
We have 5 ground-truth (sessionID, output) pairs and a hypothesized algorithm `cert_rust_repro` (per `/home/sdancer/nmss-emu/cert-rust-repro/README.md`): single SHA-256 compression block from canonical IV, applied to a 64-byte input block at known function-frame offset `sp+0x968`, take digest bytes [4..28] byteswapped per word. The algorithm is hypothesized; the input block has never been captured directly. Capture one (input block, output) pair from a live function execution and verify the algorithm reproduces the captured output. This validates or falsifies the published algorithm.

**Per session memory `feedback_nmss_detection_rules` (cycle 1038)** + corrected `frida_spawn_vs_attach_distinction_2026_05_18`: the spawn-with-xerda pattern is the safe form. Detection comes from (a) non-xerda agent fingerprint OR (b) code-patching APIs (`Interceptor.attach`, `Stalker`, `Interceptor.replace`). Read-only `Memory.readByteArray` with NO hooks is logically equivalent to the dd `/proc/PID/mem` reads we validated invisible 4× in cycles 1031-1037.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay`
- **sub_goal_key**: `cert-algo-validate`

## Hypothesis
The 64-byte buffer at `sp+0x968` on entry to the cert function in `libnmsssa.so` (at libnmsssa base + offset `0x17ee1c`) compresses to the on-wire `Token` field of op901 PktLobbyNetmarbleSSecurityVerify via the algorithm documented in `cert-rust-repro/README.md`.

## Falsification criterion
Captured block runs through `cert_rust_repro --session <block>.json` and produces output ≠ the live op901 Token captured in the same run. Then either (a) the documented algorithm is wrong, or (b) `sp+0x968` is the wrong offset, or (c) the block's pre-image was already mutated before our read. Worker documents which and stops.

## Hard rules
- **adb target**: `localhost:5558` only.
- **xerda spawn pattern only**: `dev.spawn(['com.netmarble.thered']) + dev.resume(pid)`. NEVER `dev.attach(running_pid)`.
- **NO `Interceptor.attach`, `Interceptor.replace`, `Stalker`, or any code-patching API.** Read-only mem inspection only.
- The xerda binary is the rebrand of frida-server; spawn it before driving:
  ```
  adb -s localhost:5558 shell "su 0 sh -c 'ls /data/local/tmp/ | grep -i xerda; pgrep -f xerda-server'"
  ```
  If not running, start per prior session pattern (xerda-server bg with default port 27042; check existing `frida_driver.py` at `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/frida_driver.py` for the working invocation).
- 45 min wall cap. 512 MB RSS worker host.

## Step 1 — JS script (constructor-only polling, no hooks)
Write `cert_block_poll.js`:
```js
'use strict';

// Find libnmsssa.so base
function libBase(name) {
  const modules = Process.enumerateModules();
  for (const m of modules) if (m.name === name) return m.base;
  return null;
}

function dumpAtSP() {
  // Periodic poll of any active thread's sp+0x968 region.
  // No hooks. Just enumerate threads and read.
  const threads = Process.enumerateThreads();
  for (const t of threads) {
    try {
      const sp = t.context.sp;
      if (sp.isNull()) continue;
      const block = sp.add(0x968).readByteArray(64);
      if (!block) continue;
      const u8 = new Uint8Array(block);
      // Filter: skip all-zero windows, log only non-trivial reads
      let nz = 0;
      for (let i = 0; i < 64; i++) if (u8[i] !== 0) nz++;
      if (nz < 8) continue;
      const hex = Array.from(u8).map(b => b.toString(16).padStart(2, '0')).join('');
      send({kind: 'sp_0x968', tid: t.id, sp: sp.toString(), hex: hex});
    } catch (e) {
      // Thread state not readable / SP off-stack / etc — skip silently
    }
  }
}

setTimeout(function () {
  const base = libBase('libnmsssa.so');
  send({kind: 'init', libnmsssa: base ? base.toString() : null});
  setInterval(dumpAtSP, 50);  // 20 Hz poll
}, 100);
```

## Step 2 — driver (host-side Python)
Reuse the pattern from `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/frida_driver.py` (xerda spawn + remote device + script load). On each `sp_0x968` message, append to `analysis/artifacts/sp_0x968_observations.jsonl`. Run for 90 s. Concurrently tcpdump on device for op901 ground truth.

## Step 3 — drive login flow
After spawn + script load + driver running:
```bash
adb -s localhost:5558 shell "su 0 sh -c 'pkill -INT tcpdump 2>/dev/null; sleep 1; nohup tcpdump -i any -s 0 -w /sdcard/cav.pcap \"(net 183.110.0.0/16) and port 12000\" > /sdcard/cav_td.log 2>&1 &'"
```
Then drive UI from title screen to character-select to trigger op901 emission. Tap sequence with screenshot verification per the cycle-1022 pattern (see `briefings/thered-enter-game-pcap.md` Step 3).

## Step 4 — correlation + algorithm validation
1. Decode op901 from pcap via vampir test-wire (or python decoder per cycle-1022 work) → live `Token` hex.
2. From observations.jsonl, for each unique 64-byte block, run `cert_rust_repro --session <synthesized.json>` and check if output matches the live Token. Synthesized JSON only needs `sp_0x968_block_hex64` set to the observed hex; other fields can be donor values per README schema.
3. On match: set fact `cert_algo_validated_2026_05_18` with the matching block + offset. Goal stage closes.
4. On no match: catalog the observations + Token + offset analysis, set fact `cert_algo_block_not_at_sp_0x968_2026_05_18` with diagnostic.

## Outputs
- `analysis/artifacts/cert_block_poll.js`
- `analysis/artifacts/frida_driver.py` (copy + adapt)
- `analysis/artifacts/sp_0x968_observations.jsonl` (raw 64-byte hex stream)
- `analysis/artifacts/cav.pcap` + `cav_op901.json` (live ground truth this run)
- `analysis/artifacts/cert_algo_validation.md` (verdict)
- Facts: SUCCESS `cert_algo_validated_2026_05_18=true` (with matching block + Token), FAIL `cert_algo_block_not_at_sp_0x968_2026_05_18=true` (with diagnostic).
- Final line: `CERT_ALGO_VALIDATE_DONE`.

## References
- `/home/sdancer/nmss-emu/cert-rust-repro/README.md` — algorithm spec
- `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/frida_driver.py` — prior working xerda driver
- `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/conn2_op901_extracted.json` — example op901 decode
- Memory: `feedback_nmss_detection_rules` (xerda safe, no hooks)
- Fact: `frida_spawn_vs_attach_distinction_2026_05_18` (spawn pattern only)
- Fact: `cert_mem_snapshot_op902_succeeded_4x_2026_05_17` (passive reads invisible to NMSS, 4× validated)
