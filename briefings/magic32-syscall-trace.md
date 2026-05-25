# magic32-syscall-trace — catch the MAGIC32 fetch event via syscall trace

## Role & workdir
Fresh path. Codex agent, workdir `/home/sdancer/nmss-emu-magic32-syscall-trace` (branch `magic32-syscall-trace`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `magic32-syscall-trace-fetch-event`

## Why this path
Sibling campaign `cert-hw-bp` (v1..v16) retired 2026-05-18 at cycle 1206 for substrate exhaustion. 6-in-row falsifications across rwxp BPs / r-xp BPs / scudo:primary / libnmsssa-rw / deleted-mod-rw — no MAGIC32 plaintext anywhere in cert-function execution territory. Conclusion: MAGIC32 is either binary-encoded or **fetched and freed per cert call**. This path targets the **fetch event** directly via kernel-level syscall tracing — bypasses every wall the cert-fn-side hit.

## Hypothesis
During a driven thered login the MAGIC32 install constant is delivered into the process via at least one of: `openat`/`read` on a config file under `/data/data/com.netmarble.thered/`, `recvfrom`/`recvmsg` on a GMS-API socket, `pread64` on a shared resource, or `getrandom` (unlikely — install-keyed not random). Tracing those syscalls during a driven login captures the MAGIC32 fetch in a buffer we can inspect.

## Falsification criteria (any one)
- A 32-char ASCII hex string OR a 16-byte buffer whose hex-encoding matches `MD5(176062C5A333E9E7 || x || 176062C5A333E9E7) == 21bd3dc15046f910d7143353d60694de` appears in any traced syscall buffer → **MAGIC32 captured, goal closes 5/6 → 6/6**.
- 120s driven login produces op901 in pcap AND zero candidate 16B/32-hex buffers in syscall trace → MAGIC32 is not delivered via these syscalls (maybe in-memory derivation, IPC binder transaction, or already-cached pre-login). Pivot to `magic32-pgs-java-frida` (Java-side Frida hook on SharedPreferences put/get).
- 5 driven logins, zero op901 emitted in any → session-caching wall, can't trigger cert path. Escalate to user (substrate ask).

## Hard rules
- **Workdir is the new worktree** at /home/sdancer/nmss-emu-magic32-syscall-trace.
- **NO `pm clear`** per standing memory rule.
- **kernel-side instrumentation is allowed** (memory rule `feedback_kernel_instrumentation`): eBPF, kprobes/uprobes on system libs (NOT libUnreal.so).
- **adb localhost:5558** is the only device.
- **Driven login MUST produce op901** before declaring 0-result falsification.

## Step 1 — pick instrumentation
1. Two viable approaches, pick the cheapest to bring up:
   - **a) `strace -f -p $PID -e trace=openat,read,recvfrom,recvmsg,pread64,getrandom -s 4096 -o /tmp/m32_strace.log`** — straightforward but high-overhead; may impact app behavior.
   - **b) eBPF via `bpftrace`** — `tracepoint:syscalls:sys_enter_read { if (pid == TARGET) { ... } }`. Lower overhead, more flexible filter. RK3588 has bcc/bpftrace per prior facts.
2. Recommend (b) if bpftrace is on-device; else fall back to (a). Verify with `which bpftrace` over adb.

## Step 2 — drive a clean login + capture
1. Force-stop thered: `am force-stop com.netmarble.thered`. (NOT `pm clear`.)
2. Resolve fresh PID after relaunch via `monkey -p com.netmarble.thered 1`.
3. Attach trace.
4. Drive UI: tap sign-in via input tap coords (see prior briefings for sign-in tap location, ~ cycle 1118 coords).
5. Run for 120s OR until op901 in `tcpdump -i any port 12000 -nn -A -s 0`.
6. Detach trace.

## Step 3 — scan capture
1. Extract every `read/recv/pread` buffer.
2. For each buffer of length ≥16, slide a 16-byte window: hex-encode → MD5 test against `21bd3dc15046f910d7143353d60694de` with surrounding challenge bytes. ALSO scan for the literal 32-char hex form.
3. On match: print syscall name + fd + buffer offset → save to `analysis/magic32_fetch_evidence.txt`.
4. If fd is a file, `readlink /proc/<pid>/fd/<n>` resolves the file path. If socket, peer address gives the API endpoint.

## Step 4 — verdict + commit
Single commit, message `magic32-syscall-trace: <verdict>`. Verdict at `analysis/magic32_syscall_trace_verdict.md`. Final line `MAGIC32_SYSCALL_TRACE_DONE`. On match: emit MAGIC32 value, patch cert-rust-repro, run 13 wire pairs.

## Constraints & gotchas
- The cert-emitting thread in prior captures was tid `21092` of PID 24114. PID will differ this session — re-resolve.
- Earlier hypotheses memory has fact `magic32_prefs_writer.md` showing 16/16 closure on `nmss_magic32_origin` with PGS player ID as source — Java-side prefs write at install time. Reading-side may be one of: SharedPreferences `getString`, JSON field `I_PID`, or socket recv from a GMS API.
- adb is fragile under sustained large reads — the cert-hw-bp v15 saw 22MB+ reads crash adb. Keep individual reads small.

## Relevant files / references
- /home/sdancer/orchestrator/analysis/falsified.md (cert-hw-bp retirement entry)
- /home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v16_verdict.md (final closure)
- /home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md (producer chain — 16/16 closure)
- Ground truth: challenge=`176062C5A333E9E7`, selector=`21bd3dc15046f910d7143353d60694de`, Token=`BA00FE3EAB6937AD8183100FEB4D14B7B76C67EAD7C38D9C`
- Memory rules: `feedback_kernel_instrumentation`, `feedback_no_frida` (Frida OK on Java/system via xerda — not for this path, this is kernel)
