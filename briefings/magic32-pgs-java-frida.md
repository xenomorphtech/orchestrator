# magic32-pgs-java-frida — Java-side Frida hook on SharedPreferences/PGS path

## Role & workdir
Fresh path. Claude worker, workdir `/home/sdancer/nmss-emu-magic32-pgs-java-frida` (branch `magic32-pgs-java-frida`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `magic32-pgs-java-frida-shared-prefs`

## Why this path
Sibling path `magic32-syscall-trace` retired 2026-05-18 (commit 139446d) with substrate-gap verdict: simpleperf raw_syscalls captures syscall args but NOT buffer contents; post-hoc /proc/mem deref of 4437 addrs found ZERO MAGIC32 hits. Kernel has no CONFIG_FTRACE_SYSCALLS / no kprobe_events / no bpftrace. **Critical positive fact retained**: simpleperf is NOT detected by NMSS anti-debug (strace IS).

This path attacks the **producer side**: MAGIC32 originates from Google Play Games Services player ID (per cycle-942 closure on `nmss_magic32_origin`). The producer chain writes via Java `SharedPreferences$EditorImpl.putString` and reads via `JSONObject.optString("I_PID")`. Both are pure-Java APIs reachable from xerda Frida injection (memory rule `feedback_no_frida` explicitly allows Java/system targets via xerda binary).

## Hypothesis
Hooking Java `android.app.SharedPreferencesImpl$EditorImpl.putString` AND `org.json.JSONObject.optString` during a driven thered login captures MAGIC32 plaintext at write-time or read-time. xerda's Java interceptor is undetected by NMSS (it operates against ART, not libUnreal — anticheat boundary is the native side).

## Falsification criteria (any one)
- xerda Frida hook fires on `SharedPreferencesImpl$EditorImpl.putString` AND captures a 32-char hex string with key matching "I_PID" / "MAGIC32" / "PlayerId" / "CommonLogJson" → MAGIC32 identified. Patch cert-rust-repro; verify against 13 wire pairs.
- Frida injection succeeds, app stays alive, but NO putString fires during driven login OR fires with non-32-hex content → MAGIC32 isn't passed through SharedPreferences. Pivot to `magic32-hw-write-watchpoint` (perf_event_open HW_BREAKPOINT_W on stage_drv buffer landing zones, distinct substrate).
- Xerda injection detected by NMSS / app dies on attach → falsified at substrate boundary (memory rule contradiction would trigger; prior memory says xerda Java is OK). Document and escalate.

## Hard rules
- **xerda binary only** (memory `feedback_no_frida`). NEVER attach to libUnreal.so — that triggers anticheat. Java-side classes ONLY.
- **adb localhost:5558** is the only device.
- **NO `pm clear`** per standing memory rule.
- **Driven login MUST produce op901** in pcap before declaring 0-result falsification (per memory `feedback_verify_precondition_before_probe`).
- **40-min wall cap** total.

## Step 1 — verify xerda + Java attach substrate
1. Locate xerda binary on host or device. Check `/data/local/tmp/xerda`, `/system/bin/xerda`, or build from sibling worktree if present (`ls /home/sdancer/nmss-emu*/xerda*`).
2. Force-stop thered. Spawn fresh with xerda attached at Java-load time (xerda's `--spawn` mode if available, else `--attach <pid>` shortly post-launch).
3. Verify app stays alive 30s post-attach AND tcpdump shows outbound TCP (not dead in seconds like strace was).

## Step 2 — write Java hooks
1. Hook `android.app.SharedPreferencesImpl$EditorImpl.putString(String, String)`. Log `(key, value)` to /data/local/tmp/m32_prefs.log when value is 32-char ASCII hex.
2. Hook `org.json.JSONObject.optString(String)` and `org.json.JSONObject.getString(String)`. Log `(key, value)` when value is 32-char ASCII hex AND key matches `(I_PID|MAGIC32|PlayerId|CommonLogJson|playerId)`.
3. Optional belt+suspenders: hook `java.lang.String.<init>(byte[])` and `Base64.encodeToString` for AES-result→hex paths if obvious.

## Step 3 — drive a clean login + collect
1. `am force-stop com.netmarble.thered; am start -n com.netmarble.thered/com.epicgames.unreal.SplashActivity`
2. Wait for GameActivity focus.
3. Tap Game Start (use cycle-1118 tap coords; same as `magic32-syscall-trace` briefing if recorded).
4. 120s wait OR until tcpdump shows op901 to 183.110.205.25:12000.
5. Pull /data/local/tmp/m32_prefs.log. Save to `analysis/m32_prefs.log`.

## Step 4 — match against ground truth
1. For each `(key, value)` pair logged: compute `md5_hex(b"176062C5A333E9E7" + value.encode() + b"176062C5A333E9E7")`.
2. Compare to `21bd3dc15046f910d7143353d60694de` (cycle-1176 v11 selector).
3. On match: MAGIC32 found. Save to `analysis/magic32_found.txt`.

## Step 5 — verdict + commit
Single commit on `magic32-pgs-java-frida`. Message `magic32-pgs-java-frida: <verdict>`. Verdict at `analysis/magic32_pgs_java_frida_verdict.md`. Final line `MAGIC32_PGS_JAVA_FRIDA_DONE`. On match: emit MAGIC32 value, patch cert-rust-repro at /home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs, run on 13 wire pairs, set `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`.

## Constraints & gotchas
- Memory `feedback_no_frida`: Frida on libUnreal triggers anticheat; Java-side via xerda is OK.
- xerda binary path: prior briefings reference it; if not found at standard paths, check `which xerda`, `find /home/sdancer -name xerda -executable`, or build from /home/sdancer/xerda* sibling.
- Ground truth: challenge=`176062C5A333E9E7`, selector=`21bd3dc15046f910d7143353d60694de`, Token=`BA00FE3EAB6937AD8183100FEB4D14B7B76C67EAD7C38D9C`.
- 13 captured wire pairs documented in /home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v*_verdict.md.

## Relevant files / references
- /home/sdancer/orchestrator/analysis/falsified.md (magic32-syscall-trace retirement)
- /home/sdancer/nmss-emu-magic32-syscall-trace/analysis/magic32_syscall_trace_verdict.md (substrate gap)
- /home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md (16/16 closure on producer chain)
- /home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs (MAGIC32 constant for patch)
- Memory rules: `feedback_no_frida` (xerda OK for Java), `feedback_verify_precondition_before_probe` (verify op901 first)
