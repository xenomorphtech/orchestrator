# cert-hw-bp-v2 — point HW BP at deleted runtime module (where cert actually lives)

## Role & workdir
Same worker, same worktree continuing from cert-hw-bp v1 (now closed at commit `91ce8a7`). Workdir: `/home/sdancer/nmss-emu-cert-hw-bp` on branch `cert-hw-bp`. Target: thered on RK3588 at `adb -s localhost:5558`.

## Goal of this turn
Cycle 1104: v1 falsified its address (libnmsssa+0x17ee1c not exercised) **but validated the substrate** (HW BP via PMU invisible to NMSS — 144 tids, 113s, zero AC reaction, no byte rewrites). The mismatch is location. Per facts `cert_algorithm_in_deleted_runtime_modules` (cycle 942) and `cert_algorithm_module_identified` (cycle 947), the cert algorithm body lives in a **deleted runtime module** at `/data/data/com.netmarble.thered/files/<hash> (deleted)`, NOT in libnmsssa.so.

Find the cert function entry inside the deleted module on the live device, point the proven HW BP probe at it, drive a SUCCESSFUL login (one that emits op901 — Run #2's "Failed to download configuration file" failure must be debugged), capture sp+0x968 pre-image block, verify via `cert_rust_repro` against live op901 Token.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay`
- **sub_goal_key**: `cert-hw-bp-v2`

## Hypothesis
The cert function entry is at one of two r-xp ranges of `/data/data/com.netmarble.thered/files/45afd84ac456abc (deleted)`:
- `0x74684ca000-0x74685d1000` (size 0x107000)
- `0x74685d3000-0x74688db000` (size 0x308000)

Hash filename rotates per install — use the pattern (ELF-mapped deleted file under `files/`) as the discriminator. HW BP at the correct offset inside one of these ranges will fire on each cert call. With sp+0x968 captured, the 4/4-verified `cert_rust_repro` algorithm reproduces the live Token.

## Falsification criteria (any one)
- All function-entry candidates inside the deleted module's r-xp ranges yield 0 firings during a SUCCESSFUL login (op901 emitted in pcap) — function not at any of the tested addresses.
- HW BP fires + captures block, but `cert_rust_repro` output ≠ live Token — algorithm wrong (would contradict 4/4 cycle-1100 verifier validation).
- NMSS belatedly detects HW BP via debug-register read or perf-event-counter introspection — close substrate too.

## Hard rules
- **adb target**: `localhost:5558` only.
- **HW BP only**, no kernel uprobe (NMSS rewrites BRK SWBP — cycle 1090).
- **No code modification** anywhere (PMU debug registers are state-only).
- **Driven login MUST emit op901** before declaring 0-firings a falsification. If login fails before cert step ("Failed to download configuration file" or similar), debug the UI/network path first.
- **45 min wall cap.** 512 MB worker host RSS. ≤ 100 MB device disk for artifacts.

## Step 1 — locate the deleted module and identify cert function entry
1. Refresh thered PID and re-scan maps:
   ```bash
   PID=$(adb -s localhost:5558 shell pidof com.netmarble.thered | tr -d $'\r')
   adb -s localhost:5558 shell "su 0 awk '/r-xp.*files\\/[0-9a-f]+ \\(deleted\\)/' /proc/$PID/maps"
   ```
2. For each r-xp range, dump the bytes via `dd if=/proc/$PID/mem skip_bytes=...`. Disassemble with `aarch64-linux-gnu-objdump -D -b binary -m aarch64`. Look for the cert function signature: 
   - **SHA-256 K-table xref** (canonical addr 0x6a09e667 etc. or via gadget-scan fact `nmss_cert_primitive_identified`)
   - **`movz x?, 0xCCA6, lsl 0x10`** (cycle 919/942 disasm marker)
   - **stp/ldp around sp+#0x968** to confirm stack layout
3. If multiple candidates, pick the entry with the strongest match. Estimate runtime address (range_start + offset_within_blob).

## Step 2 — re-aim probe and drive a SUCCESSFUL login
1. Verify thered is at title screen ready to tap login. **First fix the "Failed to download configuration file" issue from Run #2** — possibly stale config, time skew, or network DNS. If app needs `force-stop` + relaunch + tap, do it cleanly.
2. Compute final HW BP address (one or two top candidates if disasm is ambiguous).
3. Launch `cert_hw_bp_probe --target-pid $PID --target-vaddr <cand1>` from `analysis/cert_hw_bp_probe` (reuse v1 binary; recompile only if probe code needs change).
4. Concurrently `tcpdump` LOBBY. Drive the UI past login → character select → cert step. Verify **op901 emission** in pcap before declaring run useful.
5. Drain samples. Read sp+0x968 for each sample via `/proc/$tid/mem`.

## Step 3 — validate algorithm against live Token
1. Decode op901 Token from pcap (reuse `analysis/artifacts/decode_op901.py`).
2. For each captured 64-byte block, run the v1-verified Python verifier (it's in `analysis/verify_blocks.py`). On match → success.
3. Cross-check with `cert_rust_repro --session <synth.json>` for a second-implementation confirmation.

## Step 4 — verdict + commit
- **MATCH**: set fact `cert_algo_validated_via_hw_bp_v2_2026_05_18=true` with block + Token + cert function VA. Goal 6/6. Celebrate.
- **0 firings with op901 emitted**: cert function NOT at this candidate address. Try next candidate. After 3 candidates, document and fall back to backlog `cluster1-derivation-static-re`.
- **firings but no Token match**: contradicts 4/4 algorithm verification — investigate (maybe sp+0x968 offset shifted, try sp+0x960 / +0x970).
- **NMSS belated detection**: set fact `nmss_actively_defends_hw_bp_2026_05_18=true`, retire substrate.

Final line: `CERT_HW_BP_V2_DONE`.

## Outputs
- `analysis/cert_hw_bp_v2_target_search.md` — disasm findings for the deleted module
- `analysis/artifacts/cav_v2_<ts>.pcap` + decoded op901
- `analysis/artifacts/cert_hw_bp_v2_samples_<ts>.jsonl` + `_blocks_<ts>.bin`
- `analysis/cert_hw_bp_v2_verdict.md` (one of the 4 outcomes above)
- One commit per Step on branch `cert-hw-bp`.

## Constraints & gotchas
- Hashed filename `45afd84ac456abc` from v1 maps is from PID 2272 at cycle 1103 timestamp. PID + hash both rotate. Re-resolve every run.
- Deleted modules are mapped from memfd; the inode is still in /proc/<pid>/map_files/. `process_vm_readv` and `/proc/<pid>/mem` both work.
- POLLERR fix from v1 (cycle 1099-1100) is already in the probe — DO NOT regress to the dark-december logging pattern.
- 4/4 cert algorithm verification cycle 1100 means we have known-correct math; if block captured but no Token match, **the block is wrong**, NOT the algorithm.
- Worker survives a /clear (briefing-pointer prompt re-seeds). Memory file `fact_cert_hw_bp_detected_by_nmss_2026_05_18.md` was an earlier draft from kuprobe lane — actually invalidated by v1 substrate-validation finding. Disregard.
- "Failed to download configuration file" UI error from v1 Run #2 may indicate config server / DNS / time-skew issue. Test with `curl` or `ping` to thered's CDN before tapping login.

## Relevant files / references
- `analysis/cert_hw_bp_probe.c` + binary — v1 substrate (reuse)
- `analysis/verify_blocks.py` — 4/4-validated cert algorithm verifier
- `analysis/cert_hw_bp_verdict.md` — v1 closure with full v2 background
- `analysis/artifacts/decode_op901.py` — op901 decoder
- Fact `cert_hw_bp_substrate_validated_2026_05_18` — substrate proven open
- Fact `cert_rust_repro_algorithm_verified_4_of_4_2026_05_18` — algorithm correct
- Fact `cert_algorithm_in_deleted_runtime_modules` (cycle 942) — anchor for v2 targeting
- Fact `cert_algorithm_module_identified` (cycle 947) — snapshot-era cert address
- Fact `nmss_actively_defends_kuprobe_2026_05_18` (cycle 1090) — what we're routing around
- 7 ground-truth (sessionID, Token) pairs across `thered_cert_pair_*` facts
