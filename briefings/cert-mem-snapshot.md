# cert-mem-snapshot — capture sp+0x968 block via kernel-invisible mechanisms

## Role & workdir
Memory-capture worker. Workdir: `/home/sdancer/nmss-emu-device-webview-signin`. The cert-rust-repro tree at `/home/sdancer/nmss-emu/cert-rust-repro/`.

## Why this turn exists
Cycle 1027 H-CL2 found Frida triggers NMSS poisoning. Cycle 1030-1031 cert-mem-snapshot run **validated** the kernel-invisible hypothesis: op902 Result=0 SUCCESS with `dd /proc/PID/mem` running in parallel (fact `cert_mem_snapshot_op902_succeeded_2026_05_17`). Passive cross-process mem reads are NOT detected.

But the block wasn't found due to a **mechanical bug** (fact `cert_mem_snapshot_wrong_stack_end_dumped_2026_05_17`): the snap loop dumped the LAST 512KB of each `[stack]` mapping, but ARM64 stacks grow DOWN — the HIGH end is unused tail. 62 snap files all zero-byte. Fix is needed.

Two ground-truth (sessionID, cert) pairs now available:
- Pair 1 (cycle 1022): sessionID `D0D2691014BE1858` → cert `6045D74EB176E0C4A4DB23F0CEBEBEB7EE7644730D213147`
- Pair 2 (cycle 1031): sessionID `CE37A6C13EE831D6` → cert `9D6C5B875902C1062670232A76DFE5AC1191E64A97767C55`

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay`
- **sub_goal_key**: `cert-mem-snapshot`

## Hypothesis
A passive read of `/proc/<pidof_thered>/mem` during the op901-emit window will capture the cert input block without triggering NMSS detection, because NMSS specifically intercepts injection-style debuggers (ptrace attach, Frida agent load) — NOT passive cross-process reads from a privileged uid.

## Falsification criterion
EITHER (a) the captured memory snapshots contain no SHA-256-padded 64-byte block whose `cert_rust_repro` output matches `6045d74eb176e0c4a4db23f0cebebeb7ee7644730d213147` (cycle 1022 ground truth), OR (b) op902 still returns Result≠0 during this run (NMSS detected even passive reads) — then escalate to LKM kprobe.

## Hard rules
- **adb target**: `localhost:5558` only.
- **DO NOT attach Frida.** Per the new finding, Frida triggers cert poisoning. No `frida-server`, no `frida-trace`, no `xerda` agent injection. Kill any running frida-server on device before starting (`am force-stop`-friendly).
- **DO NOT ptrace-attach.** Same detection risk. Use only `/proc/<pid>/mem` reads via `dd` or `cat`.
- **DO NOT clear thered app data.** `am force-stop` is OK.
- **30 min wall cap.** 512 MB RSS cap (worker host) + ≤ 1 GB pcap+memdump size on device.
- If host-side privilege is needed (likely is, for /proc/PID/mem cross-uid), use `su 0` on device — root is available there.

## Plan — three tracks

### Track A v3 — FULL stack + heap + higher cadence (cycle 1034 retry)
**v2 verdict**: 32-bit overflow bug fixed; 260 snaps × 256KB scanned cleanly; substrate validated in triplicate (3 successful op902 captures with parallel mem reads). BUT: cert thread's SP was outside the `[SP-256KB, SP+64KB]` window. Even sessionID raw bytes 0 hits.

**v3 strategy:**
1. **Dump FULL anon-stack mappings**, not centered windows. The `[anon:stack_and_tls:<tid>]` mappings are typically 1MB each, ~180 threads = 180MB raw. With tar+gzip on device that's manageable (~10MB compressed) for transfer.
2. **Include heap regions**: `[anon:libc_malloc]`, `[anon:scudo:*]` — the `sp_0x968_block` might live on the heap (a 64-byte arena alloc), not stack.
3. **Higher frequency sampling**: 50ms not 500ms. Cert function runtime is <1ms — current cadence misses it.
4. **Start sampling BEFORE the login tap.** The cert is generated very early in the login flow; if you tap then start sampling you've already missed it.
5. **tar+gzip on device** before pull to keep transfer cheap (cycle 1034 iter1+2 = 66MB raw → 3MB tgz).

### Track A v2 — kstkesp window (CYCLE 1031-1034, FALSIFIED: cert thread SP outside window)
**Verdict**: bug-fixed but block not in window. Falsifies the "cert thread SP is near startstack" assumption.

### Track A v1 — naive last-512KB (CYCLE 1030, FALSIFIED MECHANICALLY)

### Track A v1 — `/proc/PID/mem` passive snapshot (LEGACY, FALSIFIED MECHANICALLY)
1. **Kill all Frida processes on device** to remove anti-cheat detection signal:
   ```bash
   adb -s localhost:5558 shell "su 0 sh -c 'pkill -9 frida-server xerda-server 2>/dev/null; sleep 1; pidof frida-server xerda-server'"
   # Expect empty output
   ```

2. **Identify the cert function's frame range on the device.** From facts:
   - libnmsssa.so cert function = absolute base + offset 0x17ee1c
   - sp+0x968 is computed inside the function frame
   - Stack typically lives in a region marked `[stack]` in `/proc/PID/maps`

   Pre-load the libnmsssa base + thread stacks:
   ```bash
   adb -s localhost:5558 shell "su 0 sh -c '
     PID=$(pidof com.netmarble.thered)
     echo \"PID=$PID\"
     grep -E \"libnmsssa|\\[stack\" /proc/$PID/maps
   '"
   ```

3. **Start broad tcpdump for ground-truth correlation:**
   ```bash
   adb -s localhost:5558 shell "su 0 sh -c '
     pkill -INT tcpdump 2>/dev/null; sleep 1
     nohup tcpdump -i any -s 0 -w /sdcard/cert_mem.pcap \"(net 183.110.0.0/16) and port 12000\" > /sdcard/tcpdump_cert_mem.log 2>&1 &
   '"
   ```

4. **Start a high-frequency memory-snapshot loop on device** — sample all `[stack]` regions of thread PIDs every ~100ms, save uniquely-hashed snapshots:
   ```bash
   adb -s localhost:5558 shell "su 0 sh -c '
     PID=$(pidof com.netmarble.thered)
     OUT=/sdcard/mem_snaps
     rm -rf $OUT; mkdir -p $OUT
     # Capture each thread stack region; sleep 100ms between iterations; 60s total
     for i in $(seq 1 600); do
       # Find each thread PID under task/ and its stack range
       for tid in $(ls /proc/$PID/task 2>/dev/null); do
         RANGE=$(grep \"\\[stack\" /proc/$PID/task/$tid/maps | head -1 | awk \"{print \\$1}\")
         [ -z \"$RANGE\" ] && continue
         START=$(echo $RANGE | cut -d- -f1)
         END=$(echo $RANGE | cut -d- -f2)
         START_DEC=$((16#$START))
         SIZE_DEC=$((16#$END - 16#$START))
         dd if=/proc/$PID/task/$tid/mem bs=1 count=$SIZE_DEC skip=$START_DEC of=$OUT/t${tid}_s${i}.bin 2>/dev/null
       done
       sleep 0.1
     done
     du -sh $OUT
     ls $OUT | wc -l
   '" 2>&1 | head -20
   ```
   (Adjust `count` / `bs` if dd performance is bad. The simpler `dd if=mem bs=4096 ... skip=<page>` is more robust.)

5. **Force-stop + relaunch thered to trigger fresh login** (Frida-less, same as cycle 1022):
   ```bash
   adb -s localhost:5558 shell "am force-stop com.netmarble.thered"
   sleep 2
   adb -s localhost:5558 shell "monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1"
   # tap through any "Please tap" screen
   sleep 5
   adb -s localhost:5558 shell input tap 960 540
   ```

6. **Wait 60s** for the memory loop + login flow to complete.

7. **Pull captures and decode:**
   ```bash
   adb -s localhost:5558 shell "su 0 pkill -INT tcpdump; sleep 1"
   adb -s localhost:5558 pull /sdcard/cert_mem.pcap analysis/artifacts/cert_mem.pcap
   adb -s localhost:5558 pull /sdcard/mem_snaps analysis/artifacts/mem_snaps/
   ```

8. **Decode the new op901 from the pcap** (same as cycle 1022 — vampir framing, XOR key, opcode 901 to lobby) to get the **ground-truth cert + sessionID** for this run.

9. **Scan all memory snapshots for SHA-256-padded 64-byte blocks**:
   ```bash
   python3 -c "
   import os, hashlib, struct, glob
   gt_cert_hex = open('analysis/artifacts/cert_mem_op901.json').read()  # filled in step 8
   # SHA-256 IV
   IV = bytes.fromhex('6a09e667bb67ae853c6ef372a54ff53a510e527f9b05688c1f83d9ab5be0cd19')
   # ...scan each .bin, slide a 64-byte window, run one SHA-256 compression block, take digest[4..28], byteswap per word, compare
   "
   ```
   For each 64-byte candidate window in any snapshot, compute `cert_rust_repro(window)` and check match.

10. **If a match found** → write `analysis/artifacts/cert_mem_snapshot_match.json` with the captured `sp_0x968_block`, the matching cert, and the snapshot file/offset. Set fact `cert_rehydrate_block_captured_via_mem_2026_05_17` = true.

### Track B (escalation) — LKM kprobe (only if Track A's mem-snapshot misses the block OR op902 returns Result≠0 meaning even mem-reads were detected)
Defer to a separate path `cert-lkm-kprobe`. Don't attempt in this turn; emit a "needs LKM" fact.

### Track C — fall-through (only if Tracks A+B run out of budget)
Capture the op901 traffic (ground-truth pair for the run) anyway. Even without a block match, document the new cert/sessionID pair as additional ground truth for future algorithm-fit work.

## Outputs
- `analysis/artifacts/cert_mem.pcap` — tcpdump of the run
- `analysis/artifacts/cert_mem_op901.json` — decoded op901 (new ground-truth cert + sessionID)
- `analysis/artifacts/mem_snaps/` — directory of memory-snapshot binaries
- `analysis/artifacts/cert_mem_scan_results.json` — for each snapshot file: candidate blocks tested, match status
- On SUCCESS: `analysis/artifacts/cert_mem_snapshot_match.json` with the validated block
- Facts:
  - SUCCESS: `cert_rehydrate_block_captured_via_mem_2026_05_17` = true
  - PARTIAL: `cert_mem_snapshot_op901_captured_no_match_2026_05_17` = true (new ground-truth pair, no block found)
  - FAIL: `cert_mem_snapshot_op902_rejected_2026_05_17` = true (passive reads also detected — escalate to LKM)
  - Final line: `CERT_MEM_SNAPSHOT_DONE`

## References
- `/home/sdancer/nmss-emu/cert-rust-repro/README.md` (algorithm)
- `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/conn2_op901_extracted.json` (cycle 1022 clean ground truth)
- `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/cert_rehydrate_track_B.json` (cycle 1027 Frida-poison evidence)
- Memory: `feedback_kernel_instrumentation` (kernel substrate proven on this device)
- Memory: `feedback_lkm_library_injection` (LKM substrate proven for VMA manipulation — same kernel)
- Fact `nmss_poisons_cert_under_frida_attach_2026_05_17`
