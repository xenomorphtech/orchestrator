# magic32-status10-bypass — Turn 10: plant fake serverClientId + observe apis.netmarble.com SNI

## Role & workdir
APK-repack Codex worker. Workdir: `/home/sdancer/nmss-emu-magic32-status10-bypass/`.

## Current goal / sub-goal
- **goal_key**: `nmss_magic32_fresh_capture_enabled` (0.6/1.0)
- **sub_goal_key**: `status10-bypass-turn10-fake-server-client-id`

## Prior cleared gates (Turns 1-9)
1. Native `libUnreal.so` patch at `0x58038e8`+`0x580398c` — DEPLOYED. Hercules `.text`-hash check ruled OUT.
2. apktool smali patches on PGS$1/$2 + inner-classes — DEPLOYED. Hercules DEX-hash check ruled OUT.
3. Manifest `app_id` patched from `""` to `"204407174407"` (Firebase sender ID format). Status=10 / non-numeric-app-id error GONE.
4. **Currently blocked at**: GMS `DEVELOPER_ERROR` with `serverClientId=null` + OAuth registration mismatch. GMS resolved Google account `ktion23@gmail.com` but the request bundle has `serverClientId=null` so authorization fails.

## This turn's scope (ONE Codex turn, ≤60 min)
Plant a fake `serverClientId` (a syntactically-valid OAuth web client ID format string) somewhere thered's code can consume it during `requestServerSideAccess` so the request bundle is no longer null. Goal is to test whether GMS will let the call propagate to thered's code (which then makes the apis.netmarble.com POST with whatever auth-code GMS hands back, even if Netmarble's server rejects).

## Hypothesis
The `serverClientId` value flows from Java code (probably a resource string OR a hardcoded constant in `PGS$1$1$1` near the `requestServerSideAccess(oAuthWebClientId, false)` call we identified in Turn 1). Patching either (a) the resource string or (b) the smali code to pass a fake-but-syntactically-valid OAuth web client ID makes GMS proceed past the "serverClientId=null" rejection. Either:
- GMS accepts the fake ID, returns a synthesized auth-code (or null) → thered POSTs to apis.netmarble.com → SNI hit → metric advances
- GMS rejects the fake ID with a different error → new gate identified → escalation

## Falsification (3 outcomes)
- (a) apis.netmarble.com appears in SNI → SUCCESS, metric 0.6 → 0.85 (partial since fresh MAGIC32 still needs real Google auth, but endpoint reachability confirmed).
- (b) GMS rejects fake ID with new error (different than Status=10/DEVELOPER_ERROR) → new gate; document the new error code.
- (c) App crashes on fake ID → patch was too aggressive.

## Success criteria
**Primary**: 60s tcpdump SNI contains `apis.netmarble.com` AND thered alive.

**Closing artifact**: `analysis/turn10_fake_oauth_client_2026-05-15.md` with patch site + before/after + SNI result + classification.

**Fact key on success**: `magic32_status10_apis_netmarble_hit_with_fake_oauth`.

**Fact keys on falsification**:
- `magic32_status10_fake_oauth_rejected_new_gate` (b)
- `magic32_status10_fake_oauth_crashes` (c)

## Execution flow — atomic, all steps in one turn

**Step 1** — Find where the OAuth web client ID enters Java code. Search the decoded smali:
```bash
cd analysis/thered_decoded
grep -rln "requestServerSideAccess\|oAuthWebClientId\|server_client_id\|serverClientId" smali_classes*/ | head -20
grep -rln "default_web_client_id" smali_classes*/ res/ | head -20
grep "default_web_client_id\|server_client" res/values/strings.xml 2>/dev/null | head -10
```
Common locations:
- A resource string named `default_web_client_id` or `server_client_id` in `res/values/strings.xml`
- A hardcoded string literal in `PGS$1$1$1.smali` near the `requestServerSideAccess` invocation
- A `getString(R.string.default_web_client_id)` call somewhere

**Step 2** — Pick a fake but valid-format value. Real OAuth 2.0 web client IDs look like:
```
123456789012-abcdefghijklmnopqrstuvwxyz0123456789.apps.googleusercontent.com
```
12 digits, hyphen, 32 alphanumeric chars, `.apps.googleusercontent.com` suffix.

Use: `"204407174407-fakefakefakefakefakefakefakefake.apps.googleusercontent.com"` (matches our project_id 204407174407 prefix).

**Step 3** — Apply the patch:
- If resource string: edit `res/values/strings.xml` (add or replace the string), rebuild.
- If smali literal: edit the `const-string` line in `PGS$1$1$1.smali` near the `Lcom/google/android/gms/games/GamesSignInClient;->requestServerSideAccess(Ljava/lang/String;Z)Lcom/google/android/gms/tasks/Task;` invocation.

Document the exact edit applied.

**Step 4** — Rebuild base + sign + install (re-use Turn 9 split path):
```bash
apktool b analysis/thered_decoded -o analysis/thered_patched_base_v4.apk
zipalign -f 4 analysis/thered_patched_base_v4.apk analysis/thered_patched_base_v4_aligned.apk
apksigner sign --ks /tmp/debug.ks --ks-pass pass:android --key-pass pass:android analysis/thered_patched_base_v4_aligned.apk
adb shell 'su 0 am force-stop com.netmarble.thered' || true
adb install-multiple -r -t \
  analysis/thered_patched_base_v4_aligned.apk \
  analysis/thered_patched_arm64_aligned.apk \
  analysis/split_config.en.aligned.apk \
  analysis/split_config.mdpi.aligned.apk 2>&1 | tee analysis/turn10_install.log
```

**Step 5** — 60s smoke (same pattern as Turn 9):
```bash
adb shell 'su 0 logcat -c'
adb shell 'su 0 sh -c "rm -f /sdcard/turn10_tcp.pcap; nohup tcpdump -i any -s 0 -w /sdcard/turn10_tcp.pcap \"tcp port 443 or udp port 53\" > /dev/null 2>&1 &"'
sleep 2
adb shell 'monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1' >/dev/null 2>&1
sleep 60
adb shell 'su 0 pkill tcpdump'
sleep 2
adb pull /sdcard/turn10_tcp.pcap analysis/captures/turn10_tcp.pcap
adb shell 'pidof com.netmarble.thered' || echo "DEAD"
adb shell 'su 0 logcat -v threadtime -d' > analysis/captures/turn10_logcat.txt
```

**Step 6** — SNI extraction + Status= grep:
```bash
python3 -c "
import re
data = open('analysis/captures/turn10_tcp.pcap','rb').read()
hosts = set()
for m in re.finditer(rb'(?:[a-z0-9-]+\.)+(?:netmarble|netmarble\.com|googleapis|google|appsflyer|wetest|facebook)\.[a-z]+', data):
    hosts.add(m.group(0).decode('ascii','ignore'))
print('SNI:', sorted(hosts))
print('apis.netmarble.com hit:', any('apis.netmarble' in h for h in hosts))
" > analysis/captures/turn10_sni.txt 2>&1
cat analysis/captures/turn10_sni.txt
egrep -i 'Status.*=|signInPerformer|serverClientId|DEVELOPER_ERROR|push/v1' analysis/captures/turn10_logcat.txt | head -20
```

**Step 7** — Classify + set fact + write artifact. Print `STATUS10_FAKE_OAUTH_DONE`.

## Constraints
- **500 MB memory.**
- Re-use Turn 9 debug keystore `/tmp/debug.ks`.
- If smali edit is required: preserve indentation/whitespace exactly.
- Don't touch the arm64 split or libUnreal.so.

## Relevant files / references
- worktree: `/home/sdancer/nmss-emu-magic32-status10-bypass/`
- decoded base: `analysis/thered_decoded/`
- prior fact: `magic32_status10_manifest_patched_new_gate_remains`
- success fact: `magic32_status10_apis_netmarble_hit_with_fake_oauth`
- falsification facts: `magic32_status10_fake_oauth_rejected_new_gate`, `magic32_status10_fake_oauth_crashes`
