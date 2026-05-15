# dark-december-hive-guest-bypass — bypass Hive login WebView via SDK guest-API or UI-hidden route

## Role & workdir
Hive-SDK auth bypass analyst. Workdir: `/home/sdancer/dark-december-hive-guest-bypass` (worktree of `/home/sdancer/dark-december`, branch `hive-guest-bypass`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump`.
- Sub-goal: get past the post-patch Hive login WebView without real Google/Apple/Facebook credentials, so the game can reach the `:10001` ingame socket and the protocol becomes capturable.

## Success criteria
Concrete deliverable:
- App advances past `com.hive.ui.HiveUiActivity` and reaches the lobby/world (any post-auth screen).
- `:10001` pcap captures ≥ 1 round-trip of game-server traffic.
- Sets fact `dark_december_hive_guest_bypass_succeeded` (or per-method falsification fact on failure).

## Why this is distinct from `dark-december-patch-driver`

That path drove the UI as a normal user. Result: Hive WebView only offers Google/Apple/Facebook; no guest button surfaced.

This path bypasses the UI gate using SDK-level mechanisms:
- Hive Android SDK (com.hive.*) has a **GUEST** auth provider type in its public API. The fact that no guest button rendered in the WebView means the WebView's HTML/JS hid it for this region/build — but the underlying Java SDK call may still work.
- Java-on-system Frida is ALLOWED per memory rule [[feedback-no-frida]] (the rule forbids Frida on libUnreal.so specifically; Hive SDK is regular Java in com.hive.*).
- INCA AppGuard's anti-Frida hooks target libcompatible.so / libUE4.so, not Hive's Java tree.

## Progress so far (sibling closing artifacts — read first)

- `/home/sdancer/dark-december/analysis/recon_2026-05-15.md` — Hive auth stack identified.
- `/home/sdancer/dark-december-hive-auth-trace/analysis/task1_hive_class_location_2026-05-14.md` — **Hive Java class map** for this build. Key classes: `com.hive.auth.AuthNetwork$LoginCenter`, `com.hive.protocol.UrlManager$Membership`, `com.hive.auth.AuthImpl`. Read this artifact first — it has the actual class names you'll hook.
- `/home/sdancer/dark-december-hive-auth-trace/analysis/task2_session_key_derivation_2026-05-14.md` — HTTP wrapper crypto (AES-128-CBC zero-IV, SHA1/SHA256 of timestamp first 16 hex chars UTF-8). WS layer = plain JSON.
- `/home/sdancer/dark-december-patch-driver/analysis/ingame_protocol_capture_2026-05-14.md` — patch-driver closure with screenshots of the Hive WebView blocker.
- Cross-pollination fact: `dark_december_patch_blocked_at_9940400KB_hive_login_required` — patch download is DONE, only auth is blocking.

## Next 2–3 concrete tasks

1. **Static API inspection of Hive auth surface.**
   - Look at the baksmali'd Dark December APK for the Hive SDK auth entry points:
     - `find /tmp/dd_baksmali -name 'AuthV4*.smali' -o -name 'AuthImpl*.smali' 2>/dev/null` (decompose the APK first if not done: `apktool d -o /tmp/dd_baksmali /home/sdancer/dark-december/apk/<base.apk>`).
     - Grep for `GUEST`, `guest`, `signIn`, `signInGuest`, `ProviderType`. Hive's public API typically has `com.hive.AuthV4.signIn(AuthV4.ProviderType, AuthV4.AuthV4SignInListener)`.
     - Look at `HiveUiActivity` smali — see how it currently invokes the WebView and whether `ProviderType.GUEST` is a valid enum.
   - Document the exact method signature + enum value in `analysis/task1_hive_api_inventory_2026-05-14.md`.

2. **Frida-on-Java hook to invoke Guest sign-in.**
   - Memory rule: Frida-on-Java is ALLOWED via xerda binary (the libUnreal-only restriction does NOT cover Hive's Java tree). 
   - Approach: after the app reaches HiveUiActivity, attach via xerda and run:
     ```javascript
     Java.perform(() => {
       const AuthV4 = Java.use('com.hive.AuthV4');  // adjust to actual class path from task 1
       const ProviderType = Java.use('com.hive.AuthV4$ProviderType');
       const listener = Java.registerClass({
         name: 'com.hive.AuthV4$AuthV4SignInListener_GuestStub',
         implements: [Java.use('com.hive.AuthV4$AuthV4SignInListener')],
         methods: {
           onAuthV4SignIn(result, /*PlayerInfo*/info) {
             console.log('[guest-signin]', JSON.stringify(result), JSON.stringify(info));
           }
         }
       });
       AuthV4.signIn(ProviderType.GUEST.value, listener.$new());
     });
     ```
     (Adjust class names + signatures to match task1 findings.)
   - Verify the signIn callback fires with success result; capture the PlayerInfo / Session details.

3. **Drive into ingame + capture `:10001`.**
   - If guest sign-in succeeds, the app should advance past HiveUiActivity. Take screenshot to confirm.
   - Start on-device tcpdump again (same setup as patch-driver):
     - `adb shell "nohup tcpdump -i any -w /sdcard/Download/dd_10001_guest_$(date +%s).pcap port 10001 > /dev/null 2>&1 &"`
   - Drive UI to lobby + one inventory/menu action. Sleep 5 minutes.
   - Pull pcap: `adb pull /sdcard/Download/dd_10001_guest_*.pcap captures/`.
   - Verify with tshark: `tshark -r captures/dd_10001_guest_*.pcap -Y 'tcp.port == 10001' -T fields -e tcp.payload`.

4. **Falsification alternative**: if Frida hook doesn't work (xerda fails to attach to HiveUiActivity due to anti-frida in the WebView host process, OR ProviderType.GUEST throws "not supported"), pivot to JavaScript-level bypass:
   - Sniff the WebView's URL: `adb shell dumpsys window | rg HiveUiActivity` — find the underlying Activity, then `chrome://inspect` or extract WebView URL via UiAutomator dump.
   - The WebView loads JS from Hive's CDN. Try appending `?platform=guest` or `?provider=guest` to the URL — if the JS reads URL params, it may render the hidden button.
   - Or set the JS-loaded HTML's `display:none` on guest button → use Frida WebView hook to inject `document.querySelector('.guest-btn').click()`.

5. **Write artifact** `<workdir>/analysis/hive_guest_bypass_2026-05-14.md`:
   - API inventory (what guest providers exist in the SDK).
   - The Frida script used + xerda attachment evidence.
   - Post-bypass screenshot (lobby/world).
   - `:10001` pcap path + frame count + decoded JSON sample.
   - Fact: `dark_december_hive_guest_bypass_succeeded` on success OR `dark_december_hive_guest_blocked_<reason>` (e.g., "Provider.GUEST returns IS_NOT_SUPPORTED" — that's a real falsification meaning the build was compiled with guest-mode disabled).

## Constraints & gotchas

- **No Frida on libUnreal.so / libcompatible.so / libUE4.so / any UE4-native anticheat lib** — but `com.hive.*` Java tree is fair game (memory rule [[feedback-no-frida]]).
- adb root + SELinux permissive confirmed.
- VAMPIR dialog may appear during xerda spawn — dismiss with KEYCODE_BACK.
- com.netmarble.thered is `pm disable`d per prior cycle (cross-pollination from dark-december-recon era). Do NOT re-enable.
- This worker runs under systemd `harness-worker@dark-december-hive-guest-bypass.service` (system.slice, MemoryMax=24G).

## Relevant files / references

- Hive Java class map: `/home/sdancer/dark-december-hive-auth-trace/analysis/task1_hive_class_location_2026-05-14.md`.
- Hive HTTP crypto: `/home/sdancer/dark-december-hive-auth-trace/analysis/task2_session_key_derivation_2026-05-14.md`.
- patch-driver closing artifact: `/home/sdancer/dark-december-patch-driver/analysis/ingame_protocol_capture_2026-05-14.md`.
- patch-driver hive_dump: `/home/sdancer/dark-december-patch-driver/screenshots/hive_dump_1778766923.xml`.
- xerda Frida binary: confirmed working on this device for Java/system hooks per prior memory rule.
- Tools: `xerda`, `adb`, `apktool`, `baksmali`, `tcpdump`, `tshark`.

## Falsification

- The Hive SDK in this build has `ProviderType.GUEST` removed at compile time (e.g., enum doesn't exist) → real falsification: guest-mode was disabled by the publisher. Escalate as resource ask (need credentials).
- xerda attach to HiveUiActivity gets detected and self-suicides on attach (no prior evidence of Hive having anti-Frida, but possible).
- Bypass succeeds via API call but the server-side checks for a real Google/Apple/Facebook OAuth token in the request body → guest sign-in returns "VALIDATION_FAILED" from Hive servers. (This is the publisher's anti-cheat-on-guest measure.)

If falsified at the SDK level (no GUEST enum), the goal genuinely needs a credentials resource ask — that's a real falsification, not a missing path.
