# magic32-strings-grep-corpus — WRITE THE CLOSING ARTIFACT NOW

**STATUS:** Turn 3 closed cycle 65 (jsonrec_region.md, jsonrec_writer_candidates.jsonl). Turn 4 (cycle 70+) gathered conclusive evidence but did NOT write the closing artifact and got stuck in adrp+add verification. Metric is 15.95/16 on `nmss_magic32_origin`. **All substantive findings are preserved as harness facts.** This turn: STOP RESEARCHING and write the artifact. The remaining 0.05 (exact `bl` PC) is a nice-to-have, NOT a blocker.

## Role & workdir

Static + literal scan worker, now in REPORT phase. **Workdir**: `/home/sdancer/nmss-emu-magic32-strings/`.

## Single task this turn

**Write `analysis/magic32_prefs_writer.md`** synthesizing what's already known from the preserved facts and existing artifacts. Then set the success fact `nmss_magic32_origin_recovered_16_of_16` (or document the residual 0.05 gap if it can't be closed).

The artifact must contain (using the facts inventory below — do NOT re-derive):

1. **Full producer chain (top → bottom)**:
   - Google Play Games Services network auth via `com.google.android.gms` (primitive)
   - Java GameActivity calls JNI export `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` in libUnreal.so (dynsym offset 34125)
   - C++ Unreal code in libUnreal.so processes `googleAuthCode` + player ID; the symbol family includes `EncryptPlayerIdKey`, `EncryptedPlayerIdKeys_Key`, `PlayerIds_Key`, `SignInPlayerIdForGooglePlay`, `EncryptedPlayerIdKeys`, `EncryptedPlayerIds`, `PGSClientSecret`, `PGSClientId`, `FPGSBinderAndroid`, `PGSIDProvider`, `GetPlayerId`.
   - AES-128 encrypt step (32 hex chars = 16 bytes = AES-128 block size; MAGIC32 = AES_encrypt(player_id, key_from_PGSClientSecret))
   - The encrypted form becomes the `I_PID` value
   - JSON record build (CommonLogJson with I_DeviceModel, I_GameCode, I_NMBuildCode, I_PID, ...) — full 0x858 bytes
   - `AndroidThunkJava_SetSharedPreferenceString("cpp_native_shared", "CommonLogJson", <json>)` (rodata @ libUnreal.so offset 0x61CB58)
   - Persisted to `/data/user/0/com.netmarble.thered/shared_prefs/cpp_native_shared.xml` (file primitive)
   - Later: `SharedPreferencesImpl.loadFromDisk` reads XML at app start
   - Java `String` at heap `0x12cdf948` length `0x858` containing the JSON value
   - Reader: `CPPNativeShredPref.getCommonLogData()` calls `getString("CommonLogJson")` (classes2 line 145)
   - Parser: `CommonLogData.<init>(String json, String logUrl)` creates `new JSONObject(p1).optString("I_PID")` (classes2 line 19)
   - Result flows back to native via JNI
   - Eventually written to `device_info+0x210` std::string (heap data ptr 0xb4000079e50eb290 — long-form libc++ string)

2. **Substrate evidence summary**:
   - `libUnreal.so` (real ELF at `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so`, 158539528 bytes) is the writer-bearing module.
   - `libnmsssa.so` and `libCrashSight.so` have ZERO matches for `CommonLogJson` / `cpp_native_shared` / `AndroidThunkJava_SetSharedPreferenceString` strings.
   - MAGIC32 ASCII `2FCF997702C244969BFEAF7F0D6AAA1C` is NOT baked in any binary — runtime-computed.
   - XML payload `<string name="CommonLogJson">{...I_PID:2FCF99...}</string>` is live in heap at shard `76781000.bin` offset `0x7310bf`, region `[anon:dalvik-free list large object space]` (i.e., the XML reload result).
   - AndroidThunkJava setter offset table:
     - `AndroidThunkJava_SetSharedPreferenceInt` @ rodata 5694885 (0x56E9E5)
     - `AndroidThunkJava_SetSharedPreferenceBoolean` @ 6248381 (0x5F540D)
     - `AndroidThunkJava_SetSharedPreferenceString` @ 6409944 (0x61CB58)
     - `AndroidThunkJava_GetSharedPreferenceString` @ 6409987 (0x61CB83)
     - `AndroidThunkJava_HasSharedPreference` @ 6762838 (0x673156)
     - `AndroidThunkJava_DeleteSharedPreference` @ 6762875 (0x67317B)
     - `AndroidThunkJava_DeleteSharedPreferenceGroup` @ 6074187 (0x5C9F4B)
   - JSON-field rodata offsets: `I_NMSessionID` @ 7073142, `I_TID` @ 7075294, `I_PID` @ 7075343 (0x6BED4F), `I_UDID` @ 7075968.

3. **Residual gap**: The exact `bl` PC inside `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` that calls into `AndroidThunkJava_SetSharedPreferenceString` was not pinned via adrp+add resolution this turn (subprocess output-size limits + verification re-anchoring). The chain is otherwise complete and the writer location is conclusively identified.

4. **Naming correction**: The JSON field `I_PID` is Netmarble's misleading label. Semantically it is the **AES-encrypted Google Play Games player ID**, not a Netmarble Player ID.

5. **Final note on goal closure**: Document explicitly whether you call this a 16/16 close (full chain identified at module + symbol + primitive level) or a 15.95/16 (precise BL PC remains unresolved). State your judgment with reasoning. Do not artificially inflate or deflate.

## Constraints

- **Do not run more adrp+add scans.** Write the artifact from preserved facts.
- **One file only.** `analysis/magic32_prefs_writer.md`. No new code, no new scans, no new sub-scripts.
- **Maximum 200 lines.** Be terse and reference fact keys.
- **If you finish in under 2 minutes**, you may also append a one-paragraph end-state to `analysis/jsonrec_region.md` linking to the new file. Optional.

## Facts to cite by key

- `nmss_magic32_source_is_sharedprefs_2026_05_12`
- `nmss_magic32_xml_fragment_confirmed_in_heap_2026_05_12`
- `nmss_magic32_xml_in_dalvik_large_object_space_2026_05_12`
- `nmss_magic32_full_json_record_visible_2026_05_12`
- `nmss_magic32_writer_in_libunreal_so_2026_05_12`
- `nmss_magic32_not_baked_in_libunreal_2026_05_12`
- `nmss_magic32_primitive_is_play_games_services_2026_05_12`
- `nmss_magic32_pgs_full_string_inventory_2026_05_12`
- `nmss_magic32_is_encrypted_pgs_playerid_2026_05_12`
- `nmss_magic32_putString_not_in_classes2_2026_05_12`
- `nmss_magic32_jsonrec_in_java_art_heap_2026_05_12`
- `nmss_magic32_in_dalvik_main_space_2026_05_12`

## Stop condition

After writing `analysis/magic32_prefs_writer.md`, call `harness fact-set nmss_magic32_origin_recovered_16_of_16` (or `nmss_magic32_origin_recovered_15_95_of_16` if you decide the residual gap matters). Then stop. Do not start another sub-investigation.
