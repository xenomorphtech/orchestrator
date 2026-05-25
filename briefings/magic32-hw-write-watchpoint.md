# magic32-hw-write-watchpoint — perf_event_open HW_BREAKPOINT_W on cert staging buffers

## Role & workdir
Fresh path. Claude worker, workdir `/home/sdancer/nmss-emu-magic32-hw-write-watchpoint` (branch `magic32-hw-write-watchpoint`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `magic32-hw-write-watchpoint-stage-drv-buffer`

## Why this path
Three prior paths CLOSED on substrate or recapitulation grounds:
1. `cert-hw-bp` v1..v16: substrate-exhausted (heap scans + execute-BPs all 0-hit) → fact `cert_hw_bp_campaign_retired_substrate_exhausted_2026_05_18`.
2. `magic32-syscall-trace`: simpleperf raw_syscalls captures args not contents; /proc/mem deref of 4437 anon-rw-p addrs got 0 hits → fact `magic32_syscall_trace_substrate_gap_2026_05_18`. **CRITICAL POSITIVE**: simpleperf is NOT detected by NMSS.
3. `magic32-pgs-java-frida`: Java bridge ABSENT in xerda V8 (v8+qjs+xerda+frida-17.9.1 all `java_defined=false`); NMSS blocks Java.use introspection; SPAWN also detected (memfd:frida-agent-64.so) → fact `xerda_attach_no_java_bridge_on_nmss_2026_05_18`.

This path is **mechanistically distinct from all three**:
- Uses simpleperf substrate (known-undetected, carries forward positive fact from path 2).
- HW_BREAKPOINT_W (write-watchpoint), NOT execute-BP (the v1..v16 family). Catches the PRODUCER side at the precise byte-write boundary.
- Targets a stage_drv buffer landing zone (not the cert function entry — that's the consumer side, already attacked).

## Hypothesis
The MAGIC32 install constant is written into a stable heap or stack region by some producer (likely libgameservices or libnmsssa data init) shortly after thered launch. Placing a HW_BREAKPOINT_W (length 8, type WRITE) at the runtime VA of a stage_drv buffer's MAGIC32 slot AND letting it fire ONCE during driven login captures the producer's PC + registers at the moment MAGIC32 lands. From the captured PC, disasm reveals the encode function; from registers we extract the value directly (the WRITE buffer IS the MAGIC32 plaintext, by construction of the breakpoint).

## Falsification criteria (any one)
- HW write-BP fires on driven login AND captured value at the BP site is a 32-char ASCII hex that satisfies `MD5(challenge||value||challenge) == 21bd3dc15046f910d7143353d60694de` → **MAGIC32 captured, goal closes 5/6 → 6/6**.
- 3 candidate landing-zone VAs tried, 0 firings across all 3 driven logins (with op901 confirmed in pcap) → MAGIC32 doesn't land at a stable VA — it's recomputed per call OR lives in a different region. Pivot to backlog `magic32-magisk-zygote-frida-gadget-on-thered` or `magic32-lkm-uprobe-jni-entry`.
- HW write-BP detected by NMSS (thered dies on enable) → substrate falsifier; simpleperf isn't detected but perf_event_open with WRITE-BP may be — pivot to LKM uprobe path.

## Hard rules
- **simpleperf substrate** carried forward from magic32-syscall-trace — proven UNDETECTED by NMSS anti-debug.
- **HW WRITE BP**, NOT execute-BP (v1..v16 already exhausted that lane).
- **NO `pm clear`** per memory rule.
- **adb localhost:5558** only device.
- **Verify op901 in pcap** before declaring 0-firing falsification (memory `feedback_verify_precondition_before_probe`).
- **40-min wall cap.**

## Step 1 — Identify candidate landing-zone VAs
The cycle-1206 audit proposed: "HW_BREAKPOINT_W on the stage_drv buffer landing zone (sp+0x810 site, or *(carrier+0x30) where install constants are staged)". Three candidate VAs:
1. **sp+0x810 in cert function frame** — known from cert-hw-bp v11 (cert fn entry runtime VA 0x7468d2d75c, frame 0xb40). The MAGIC32 ASCII region is sp+0x810..sp+0x82f (32 bytes) within the cert function frame. But this is on the stack — short-lived.
2. **`*(carrier+0x30)` heap pointer** — from stage-drv-body cycle-33 finding: cert-contributing state localized to `*(carrier+0x30)` heap pointer (= 0xb400006e84c961d0 in challenge 2B6419320D30CDAE). MAGIC32 may be cached at constant offset within that heap object.
3. **libnmsssa.so rw segment slot** — even though cert-hw-bp v16 found no plaintext, a WRITE BP catches the moment a CIPHERTEXT->PLAINTEXT decode lands (writer fires, then the plaintext is consumed and overwritten). v16 only saw post-state.

Recommended order: try (2) first (heap, stable VA, long-lived), (1) second (stack, must time perfectly), (3) third (rw segment).

## Step 2 — Probe construction
Use the existing perf_event_open HW BP harness from cert-hw-bp (`/home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_probe`) but invoke it with:
- `type = PERF_TYPE_BREAKPOINT`
- `bp_type = HW_BREAKPOINT_W` (NOT `HW_BREAKPOINT_X`)
- `bp_len = HW_BREAKPOINT_LEN_8` (8-byte aligned write tracking)
- `bp_addr = <candidate VA>`
- `sample_type` includes `PERF_SAMPLE_REGS_USER` + `PERF_SAMPLE_STACK_USER` for register capture at fire-time.

You may need to slightly modify the probe binary OR rebuild with HW_BREAKPOINT_W. Source in `/home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v2/` (look for cert_hw_bp_probe.c).

## Step 3 — drive a clean login, capture
1. Force-stop thered. Fresh launch via monkey or `am start`.
2. Re-resolve PID (prior PIDs: 24114, 4199, 6135 — varies).
3. Arm probe at candidate VA.
4. tcpdump on port 12000 to verify op901 emission.
5. 120s wait; collect samples.

## Step 4 — Match captured value
For each sample's register dump and stack dump:
1. Look for any 32-char ASCII hex string.
2. For each candidate: `md5_hex(b"176062C5A333E9E7" + c.encode() + b"176062C5A333E9E7")` vs `21bd3dc15046f910d7143353d60694de`.
3. On match: extract MAGIC32 → patch cert-rust-repro stage_two_step_sha256_cert.rs → re-run on 13 wire pairs.

## Step 5 — verdict + commit
Single commit, message `magic32-hw-write-watchpoint: <verdict>`. Verdict at `analysis/magic32_hw_write_watchpoint_verdict.md`. Final line `MAGIC32_HW_WRITE_WATCHPOINT_DONE`. On match: emit MAGIC32 value, patch cert-rust-repro, set `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`.

## Constraints & gotchas
- Memory `feedback_no_frida` (Java OK via xerda) is STALE for NMSS tier — don't try Frida fallback.
- HW WRITE BP support depends on aarch64 hardware debug regs — only 4 simultaneous BPs per thread. perf_event_open arbitrates.
- Memory `feedback_verify_precondition_before_probe`: verify op901 BEFORE declaring 0-firing falsification.
- 13 wire pairs in /home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v*_verdict.md.

## Relevant files / references
- /home/sdancer/orchestrator/analysis/falsified.md (3 retirement entries: cert-hw-bp, magic32-syscall-trace, magic32-pgs-java-frida)
- /home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_probe (HW BP harness — reuse, recompile with WRITE)
- /home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v11_verdict.md (cert fn VA + frame info)
- /home/sdancer/nmss-emu-stage-drv-body/analysis/ (carrier+0x30 heap pointer findings)
- Ground truth: challenge=`176062C5A333E9E7`, selector=`21bd3dc15046f910d7143353d60694de`, Token=`BA00FE3EAB6937AD8183100FEB4D14B7B76C67EAD7C38D9C`
