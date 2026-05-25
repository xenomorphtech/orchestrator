# magic32-magisk-zygote-frida-gadget — pre-zygote Frida gadget injection

## Role & workdir
Fresh path. Claude worker, workdir `/home/sdancer/nmss-emu-magic32-magisk-zygote-frida-gadget` (branch `magic32-magisk-zygote-frida-gadget`).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `magic32-magisk-zygote-frida-gadget-pgs-capture`

## Why this path (final attempt before stalled-meta)
Five prior paths closed on substrate grounds in this session:
1. `cert-hw-bp` v1..v16: substrate exhausted (heap + execute BP).
2. `magic32-syscall-trace`: simpleperf captures args not contents.
3. `magic32-pgs-java-frida`: xerda V8/qjs Frida — NMSS blocks Java.use introspection.
4. `magic32-hw-write-watchpoint`: 3 VAs × 0 fires (arm-timing race).
5. `magic32-lkm-uprobe-jni-entry`: NMSS overwrites BRK + Security Alert blocks login.

**Critical 6-in-row pattern looming.** This path is the **last backlog hypothesis** for the goal. If it falsifies, the orchestrator will mark goal `stalled-meta` and escalate.

The differentiator: **pre-zygote injection via Magisk module**. NMSS init code runs *after* zygote fork — if we inject Frida gadget into the global libc/linker preload chain via Magisk's `zygisk` hook, gadget runs BEFORE NMSS's anti-debug init thread spawns. This is the ONLY remaining substrate that doesn't intersect with already-falsified detection mechanisms.

## Hypothesis
A Frida gadget loaded via a Magisk zygisk module that pre-empts libnmsssa.so's NMSS_Initialize() can hook `SharedPreferences.getString(MAGIC32-key)` and/or `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` from inside the JVM *before* NMSS Java-reflection blocking kicks in. The gadget's JS payload exfiltrates the captured 32-char hex via a local socket or file.

## Falsification criteria (any one)
- Gadget loads, Java hook fires AT LEAST ONCE during driven login, captured value satisfies `MD5(challenge||x||challenge) == 21bd3dc15046f910d7143353d60694de` → **MAGIC32 captured, goal closes 5/6 → 6/6**.
- Zygisk module loads but gadget crashes thered at startup (memfd:frida-agent-64.so detection) — same signature as dark-december-hive-guest-bypass — pivot to **stalled-meta + escalate**.
- Gadget loads, hook misfires (0 callbacks across 3 driven logins with op901 confirmed) — wrong hook target. Pivot to **stalled-meta**.
- Magisk zygisk module fails to install (root denied, signature, etc.) — substrate ask. Pivot to **stalled-meta**.

## Hard rules
- **Magisk required**. Check: `adb shell 'su 0 magisk -v' 2>&1`. If no Magisk → substrate ask + stalled-meta.
- **adb localhost:5558** only device.
- **NO `pm clear`**.
- **Verify op901 in pcap** before declaring 0-firing falsification.
- **40-min wall cap. After that, mark stalled-meta + escalate.**

## Step 1 — verify substrate
1. `adb shell 'su 0 magisk -v && magisk --denylist exec com.netmarble.thered ls /'` — confirm Magisk + that denylist isn't hiding root from thered.
2. `adb shell 'ls /sbin/magisk; ls /data/adb/magisk 2>/dev/null'` — locate module dir.
3. Check existing zygisk modules: `ls /data/adb/modules/` — see if dark-december-magisk-frida-gadget already deployed (memory references it).

## Step 2 — build zygisk Frida gadget module
1. Download Frida gadget for android-arm64. Check if cached at /home/sdancer/.cache/frida or in sibling worktrees (`find /home/sdancer -name 'frida-gadget*.so' 2>/dev/null`).
2. Create zygisk module structure:
   ```
   module/
     module.prop           # name, version
     libfrida-gadget.so    # the gadget
     zygisk/
       arm64-v8a.so        # zygisk entry that loads gadget
     config.json           # gadget config: script-runtime=qjs, script=hooks.js
     hooks.js              # Java hooks
   ```
3. Write hooks.js: Java.use SharedPreferencesImpl$EditorImpl, Java.use JSONObject, log every (key,value) where value matches `/^[0-9a-fA-F]{32}$/` to /data/local/tmp/m32_gadget.log.
4. Zip module → `module.zip`.

## Step 3 — install + reboot
1. `adb push module.zip /data/local/tmp/`.
2. `adb shell 'su 0 magisk --install-module /data/local/tmp/module.zip'`.
3. `adb shell 'su 0 reboot'`. Wait for adb to reconnect.
4. Verify module loaded: `magisk --modules | grep magic32`.

## Step 4 — drive login + capture
1. Force-stop thered (if alive). Fresh launch via `am start`.
2. tcpdump on port 12000.
3. Drive UI sign-in. 120s wait.
4. Pull /data/local/tmp/m32_gadget.log to analysis/m32_gadget.log.

## Step 5 — match captured value
For each `(key, value)` pair in m32_gadget.log: `md5_hex(b"176062C5A333E9E7" + value.encode() + b"176062C5A333E9E7")` vs `21bd3dc15046f910d7143353d60694de`. On match: save MAGIC32, patch cert-rust-repro.

## Step 6 — verdict + commit
Single commit on `magic32-magisk-zygote-frida-gadget`, message `magic32-magisk-zygote-frida-gadget: <verdict>`. Verdict at `analysis/magic32_magisk_zygote_frida_gadget_verdict.md`. Final line `MAGIC32_MAGISK_ZYGOTE_FRIDA_GADGET_DONE`. On match: emit MAGIC32, patch cert-rust-repro, set `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`.

## Constraints & gotchas
- Per memory `dark-december-hive-guest-bypass` (falsified.md): SPAWN-owned xerda is detected via memfd:frida-agent-64.so. Zygisk-loaded gadget MIGHT differ if loaded before fd-table scan, but plausible same fate.
- Memory `feedback_kernel_instrumentation`: kernel-side options exhausted by `magic32-lkm-uprobe-jni-entry`.
- Memory `xerda_attach_no_java_bridge_on_nmss_2026_05_18`: V8 attach has no Java.use; gadget script via qjs runtime may bypass.
- Memory `feedback_no_stall_kernel_aggressive`: don't escalate prematurely; try this substrate at least once.
- **THIS IS THE LAST BACKLOG PATH**. If falsified, write verdict noting `stalled-meta` candidacy.

## Relevant files / references
- /home/sdancer/orchestrator/analysis/falsified.md (5 retirement entries in this session)
- /home/sdancer/nmss-emu-magic32-lkm-uprobe-jni-entry/analysis/magic32_lkm_uprobe_jni_entry_verdict.md (latest verdict)
- /home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs (MAGIC32 patch site)
- Ground truth: challenge=`176062C5A333E9E7`, selector=`21bd3dc15046f910d7143353d60694de`, Token=`BA00FE3EAB6937AD8183100FEB4D14B7B76C67EAD7C38D9C`
- Memory rules: `feedback_no_frida` (xerda allowed for Java but introspection blocked), `xerda_attach_no_java_bridge_on_nmss_2026_05_18`
