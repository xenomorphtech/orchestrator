# magic32-lkm-uprobe-jni-entry — LKM-side uprobe on PGS JNI entry

## Role & workdir
Fresh path. Claude worker, workdir `/home/sdancer/nmss-emu-magic32-lkm-uprobe-jni-entry` (branch `magic32-lkm-uprobe-jni-entry`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `magic32-lkm-uprobe-jni-entry-pgs-player-id`

## Why this path
Four prior paths closed on substrate-related grounds:
1. `cert-hw-bp` v1..v16: substrate-exhausted (heap scans + execute-BPs all 0-hit) — fact `cert_hw_bp_campaign_retired_substrate_exhausted_2026_05_18`.
2. `magic32-syscall-trace`: simpleperf captures args not contents; deref of 4437 anon addrs got 0 hits — fact `magic32_syscall_trace_substrate_gap_2026_05_18`. POSITIVE: simpleperf undetected.
3. `magic32-pgs-java-frida`: xerda V8/qjs/frida-17.9.1 all `java_defined=false` (NMSS blocks Java.use); SPAWN also detected — fact `xerda_attach_no_java_bridge_on_nmss_2026_05_18`.
4. `magic32-hw-write-watchpoint`: 3 VAs × 0 fires due to ARM-TIMING RACE (probe arms 6-10s post-launch, cert at t≈5s) — fact `magic32_hw_write_bp_not_detected_by_nmss_2026_05_18` (KEY POSITIVE: HW_BREAKPOINT_W undetected).

This path attacks via **KERNEL-side uprobe** — bypasses ALL three substrate failures:
- Solves the timing race (kernel uprobe arms BEFORE app starts, fires on first JNI call)
- Solves NMSS Java-bridge blocking (operates below ART)
- Solves NMSS ptrace detection (kernel uprobes don't show in /proc/self/maps)
- Substrate proven via `lkm_injector_v1_complete_2026_05_18` (LKM kit already built)

## Hypothesis
Hooking the JNI entry point of `OnGetPGSPlayerIdWithAuthCode` (or the closest call frame in libgameservices/libgms_googlepgs) via kernel uprobe captures MAGIC32 plaintext at the moment the PGS player ID returns to native code. The uprobe fires once per fresh login, dumps `x0..x7` register state and ASCII-readable memory around the return-value pointer. From the dump, extract any 32-char hex that satisfies `MD5(challenge || hex || challenge) == 21bd3dc15046f910d7143353d60694de`.

## Falsification criteria (any one)
- Uprobe fires AND captured ASCII window contains a 32-char hex satisfying the MD5 ground-truth check → **MAGIC32 captured, goal closes 5/6 → 6/6**.
- Uprobe registers OK, 3 driven logins yield 0 fires with op901 confirmed in pcap → JNI entry symbol is wrong OR PGS player ID isn't fetched at this site. Pivot to backlog `magic32-magisk-zygote-frida-gadget-on-thered`.
- LKM uprobe registration FAILS (probe rejected by kernel, even though substrate is proven for cert-hw-bp lineage) → unique kernel constraint for libgameservices/libgms. Document and escalate to user.

## Hard rules
- **LKM substrate** carried forward from `lkm_injector_v1_complete_2026_05_18` — use existing toolkit. Don't rebuild from scratch.
- **adb localhost:5558** only device.
- **NO `pm clear`**.
- **Verify op901 in pcap** before declaring 0-firing falsification.
- **40-min wall cap**.

## Step 1 — locate target JNI symbol
1. `adb shell` then `dumpsys package com.netmarble.thered | grep nativeLibraryDir` — find APK lib root.
2. `find /data/app/...com.netmarble.thered.../lib/arm64 -name '*google*' -o -name '*pgs*' -o -name '*gms*' -o -name '*play*' -o -name '*gameservices*'` — locate the PGS library.
3. `nm -D <libname> | grep -iE 'PGSPlayerId|GooglePlayServices|PlayerID|GetPGSPlayerId'` — find candidate symbols.
4. `readelf -s <libname>` to confirm symbol offset.
5. Document candidate library + symbol + offset in `analysis/jni_entry_candidates.md`.

## Step 2 — LKM uprobe substrate setup
1. Locate existing LKM toolkit referenced by `lkm_injector_v1_complete_2026_05_18`. Search: `find /home/sdancer -name 'lkm_injector*' -o -name '*lkm*' -type d 2>/dev/null | head -10`.
2. Confirm the kit supports `register_uprobe(struct uprobe_consumer)` on userspace shared libraries.
3. Write a small uprobe handler that dumps `x0..x7` + 256B around lr/sp into a kernel-relay buffer drained via `/proc` or `debugfs`.

## Step 3 — drive a clean login + capture
1. Force-stop thered. Insert LKM module FIRST. Verify probe registered (`cat /sys/kernel/debug/tracing/uprobe_events` or LKM debugfs entry).
2. `am start -n com.netmarble.thered/com.epicgames.unreal.SplashActivity`.
3. Drive UI to Game Start tap (use cycle-1118 coords if recorded).
4. tcpdump on port 12000.
5. Drain uprobe samples until tcpdump confirms op901.

## Step 4 — match captured value
For each sample's register dump and ASCII window:
1. Look for any 32-char ASCII hex string.
2. Compute `md5_hex(b"176062C5A333E9E7" + candidate.encode() + b"176062C5A333E9E7")` vs `21bd3dc15046f910d7143353d60694de`.
3. On match: extract MAGIC32 → patch cert-rust-repro at `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` → run on 13 wire pairs.

## Step 5 — verdict + commit
Single commit on `magic32-lkm-uprobe-jni-entry`, message `magic32-lkm-uprobe-jni-entry: <verdict>`. Verdict at `analysis/magic32_lkm_uprobe_jni_entry_verdict.md`. Final line `MAGIC32_LKM_UPROBE_JNI_ENTRY_DONE`. On match: emit MAGIC32, patch cert-rust-repro, set `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`.

## Constraints & gotchas
- Memory `feedback_kernel_instrumentation` — kernel-side is preferred when userspace AC blocks; uprobes don't show in `/proc/self/maps`.
- Memory `feedback_lkm_library_injection` — LKM-side library injection bypasses /proc/maps scans.
- Memory rule `feedback_no_stall_kernel_aggressive` — when AC blocks userspace, escalate to kernel side; don't ask user for resources.
- Memory rule `feedback_verify_precondition_before_probe` — verify op901 BEFORE declaring 0-firing falsification.
- Carry-forward facts: simpleperf undetected (path 2), HW_BREAKPOINT_W undetected (path 4) — both kernel-mediated.
- Possible JNI entry candidates (search order): `Java_com_google_android_gms_games_PlayerIdGetter_nativeGet`, `Java_io_invertase_*`, or libgms's `*Player*Id*` exports. Inspect after Step 1.

## Relevant files / references
- /home/sdancer/orchestrator/analysis/falsified.md (4 retirement entries)
- /home/sdancer/nmss-emu-magic32-hw-write-watchpoint/analysis/magic32_hw_write_watchpoint_verdict.md (timing-race finding)
- /home/sdancer/nmss-emu-magic32-syscall-trace/analysis/magic32_syscall_trace_verdict.md (simpleperf undetected)
- /home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs (MAGIC32 patch site)
- Ground truth: challenge=`176062C5A333E9E7`, selector=`21bd3dc15046f910d7143353d60694de`, Token=`BA00FE3EAB6937AD8183100FEB4D14B7B76C67EAD7C38D9C`
- Memory rules: `feedback_kernel_instrumentation`, `feedback_lkm_library_injection`, `feedback_no_stall_kernel_aggressive`, `feedback_verify_precondition_before_probe`, fact `lkm_injector_v1_complete_2026_05_18`
