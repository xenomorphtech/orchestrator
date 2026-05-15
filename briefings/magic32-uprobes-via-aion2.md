# magic32-uprobes-via-aion2 — capture MAGIC32 derivation from sibling Netmarble-SDK game

## Role & workdir
Kernel-uprobe analyst leveraging a sibling app. Workdir: `/home/sdancer/nmss-emu-magic32-uprobes-via-aion2` (worktree of `/home/sdancer/nmss-emu`, branch `magic32-uprobes-via-aion2`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: capture (AES_key, plaintext) from a Netmarble-SDK sibling game (com.nctaiwan.aion2 OR com.ncsoft.lineagen) which HAS working PGS sign-in per gms-state-read evidence. Same SDK family → same algorithm. Transfer to thered.

## Why this path is distinct
- `magic32-uprobes-aes-pc` (prior, done-partial-falsification): uprobe mechanism validated, but com.netmarble.thered's PGS sign-in fails Status=10 on this Waydroid → producer never runs.
- `magic32-snapshot-key-bruteforce` (just falsified): AES key not contiguous-resident in heap.
- This path: **a different app in the same Netmarble SDK family** that has WORKING PGS sign-in (sign_in_records confirmed aion2/lineagen entries for ktion23 per gms-state-read). Same I_PID encryption scheme.

## Cross-pollination facts
- `dark_december_anti_frida_detected_2026_05_14`: Frida detected on Dark December. Use kernel uprobes (AC-invisible) here too — even though aion2 may have different AC, uprobes work regardless.
- `magic32_uprobes_capture_failed_update_gate_and_pgs_signin_status10`: thered fails PGS, aion2/lineagen don't.
- `gms_state_playerid_recovered_2026_05_14`: ktion23 is signed-in to both aion2 (a_1408633172786630918) and lineagen.
- Kernel: Linux 5.10.160 aarch64, CONFIG_UPROBES=y, CONFIG_UPROBE_EVENTS=y, /sys/kernel/debug/tracing/uprobe_events writable as root, tracefs mounted.

## Success criteria
Concrete deliverable:
- (KEY_bytes, plaintext_bytes) captured from aion2 or lineagen's AES helper during a fresh PGS sign-in.
- AES_ECB(KEY, plaintext) == aion2's I_PID (verify via shared_prefs read after capture).
- Algorithm derivation `compute_magic32(pgs_player_id) -> [u8;16]` in pure Rust.
- Test asserts: applying same derivation to ktion23's playerId `a_1408633172786630918` should match the I_PID we predict for thered (although we can't VERIFY thered's I_PID since it's never been signed-in; but algorithm + playerId + Rust impl is enough to set `nmss_magic32_numerically_reproduced`).

## Next concrete tasks

1. **Check installed sibling apps.**
   - `adb shell pm list packages | grep -E 'aion|lineag|netmarble'`
   - If aion2/lineagen NOT installed: try installing from /sdcard or via Play Store. May need user help if Play Store requires login.
   - If installed: get launcher activities + libUnreal.so path:
     - `adb shell cmd package resolve-activity -c android.intent.category.LAUNCHER com.nctaiwan.aion2`
     - `adb shell 'find /data/app -name libUnreal.so 2>/dev/null | grep -E "aion2|lineagen"'`

2. **Find aion2's AES PC.** (The PC is library-relative, similar to thered's `0x195b9f8`.)
   - Pull aion2's libUnreal.so to host.
   - `aarch64-linux-gnu-objdump -d aion2_libUnreal.so | grep -B2 -A2 'aese\|aesimc'` → find AES instruction sites.
   - Compare with thered's `0x195b9f8`/`0x195be04` — if libraries are same UE4 SDK version, AES helper PC may be at similar offset.
   - Identify the AES wrapper function entry (function prologue: `sub sp, sp, #0xd0` or similar).
   - Compute file offset (file_offset = virt_addr - executable_load_segment_virt + executable_load_segment_file_offset).

3. **Install uprobe + force fresh sign-in.**
   ```bash
   # Get base APK path on device
   LIB=$(adb shell 'find /data/app -name libUnreal.so 2>/dev/null | grep aion2 | head -1')
   adb shell "cp $LIB /data/local/tmp/aion2_libUnreal.so"
   adb shell "echo 'p:aes_aion2 /data/local/tmp/aion2_libUnreal.so:0x<file_offset> x0=%x0:u64 x1=%x1:u64 x2=%x2:u64 k0=+0(%x0):x64 k1=+8(%x0):x64 p0=+0(%x1):x64 p1=+8(%x1):x64' > /sys/kernel/debug/tracing/uprobe_events"
   adb shell "echo 1 > /sys/kernel/debug/tracing/events/uprobes/aes_aion2/enable"
   adb shell "timeout 600 cat /sys/kernel/debug/tracing/trace_pipe > /sdcard/Download/aion2_trace.txt 2>&1 &"
   
   # Force fresh sign-in
   adb shell pm clear com.nctaiwan.aion2
   adb shell am start -n com.nctaiwan.aion2/<launcher>
   # Wait 60s for cold-start + PGS sign-in
   sleep 60
   # Check trace
   adb pull /sdcard/Download/aion2_trace.txt
   ```

4. **Verify + decode.**
   - If the uprobe fires: extract key bytes (16 from k0+k1) + plaintext bytes (16 from p0+p1).
   - Read aion2's sharedprefs after launch: `adb shell cat /data/data/com.nctaiwan.aion2/shared_prefs/*.xml | grep -i I_PID` — should contain a hex32 I_PID.
   - Verify: AES-128-ECB(captured_key, captured_plaintext).hex_upper == aion2's I_PID hex32.
   - Identify the derivation:
     - Compare key bytes to known constants (search aion2_libUnreal.so rodata for the captured key bytes — they might be 16 bytes from a "PGSClientSecret_aion2" or similar string).
     - Compare plaintext to encodings of aion2's playerId (a_8735521947955340805 or a_1408633172786630918).
   - Implement `compute_magic32(pgs_player_id, [client_secret_bytes]) -> [u8; 16]` in cert-rust-repro/src/magic32.rs.
   - Add test using captured pair.
   - `cargo test`.
   - Set fact `nmss_magic32_numerically_reproduced`.

5. **Write artifact** `analysis/uprobes_via_aion2_2026-05-14.md`:
   - sibling app installed + path + AES PC + file offset.
   - uprobe trap event(s) raw output.
   - decoded (key, plaintext, ciphertext) triple.
   - Rust impl diff + test.
   - Cross-app derivation transfer notes (does thered share PGSClientSecret with aion2? Both are Netmarble SDK builds — likely same secret per-game).

## Constraints & gotchas

- DO NOT use Frida. Use only kernel uprobes via tracefs (validated working).
- adb root + SELinux permissive confirmed.
- aion2 may have its own anticheat (NCSoft games often use NEAC / EAC). uprobes are kernel-mediated — should still be invisible.
- If aion2 is not currently installed on the Waydroid → escalate (need user to install it OR use Play Store automation similar to dark-december patch-driver).
- Per memory rule [[feedback-no-stall-kernel-aggressive]]: this is the right "kernel-side bypass" route. Don't fall back to "we need the user to do X" if there's a kernel/uprobe option.
- This worker runs under systemd `harness-worker@magic32-uprobes-via-aion2.service`.

## Falsification
- Neither aion2 nor lineagen installed on Waydroid AND can't install (Play Store requires user login that we don't have).
- Sibling app installs but its PGS sign-in also fails Status=10 (then it's a Waydroid-systemic issue, not a thered-specific issue).
- uprobe fires but captured (key, plaintext) doesn't produce sibling's I_PID — the AES PC found isn't the MAGIC32 producer.
- aion2's algorithm produces correct aion2 I_PID but doesn't transfer to thered (each game uses unique PGSClientSecret) → still a partial win, set fact `nmss_magic32_algorithm_recovered_per_app_secret`.

## Relevant files / references
- Prior uprobes-aes-pc artifact: `/home/sdancer/nmss-emu-magic32-uprobes-aes-pc/analysis/uprobes_aes_capture_2026-05-14.md` (mechanism details).
- Prior key-bruteforce artifact: `/home/sdancer/nmss-emu-magic32-snapshot-key-bruteforce/analysis/key_bruteforce_2026-05-14.md` (key NOT in heap, hence runtime capture needed).
- gms-state-read sibling app evidence: `/home/sdancer/nmss-emu-magic32-gms-state-read/analysis/task1_gms_state_inventory_2026-05-14.md`.
- cert-rust-repro at `/home/sdancer/nmss-emu/cert-rust-repro/`.
- Tools: `adb`, `aarch64-linux-gnu-objdump`, `readelf`, `python3 (cryptography)`, `cargo`.
