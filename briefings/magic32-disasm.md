# magic32-disasm — static disassembly to recover MAGIC32 AES key

## Role & workdir
Static binary analyst on `libUnreal.so`. Workdir: `/home/sdancer/nmss-emu-magic32-disasm` (git worktree of `nmss-emu`, branch `magic32-disasm`).

## Current goal / sub-goal
- `nmss_magic32_numerical_repro` — MAGIC32 numerically reproduced offline from Google PGS player ID.
- Sub-goal: extract the AES-128 key (or KDF + secret bytes) and mode/padding used inside `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode`, producing a pure-Rust `compute_magic32(pgs_player_id) -> [u8; 16]` that, when hex-uppercased, matches the captured `MAGIC32 = "2FCF997702C244969BFEAF7F0D6AAA1C"`.

## Success criteria
Closing fact: `nmss_magic32_numerically_reproduced`. Concrete deliverable:
- Rust function `compute_magic32(pgs_player_id_bytes: &[u8]) -> [u8; 16]` in `cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` (alongside `compute_const_32b`).
- A unit test that consumes the known PGS player ID (captured per task 4 below) and asserts `hex_upper(compute_magic32(...)) == "2FCF997702C244969BFEAF7F0D6AAA1C"`.
- Closing artifact at `<workdir>/analysis/magic32_numerical_repro_CLOSE.md` documenting key bytes, mode (likely ECB, possibly CBC with derived IV), padding scheme (likely none — single-block PKCS#7 isn't required for an exact 16-byte plaintext), and the KDF (if any).

## Progress so far (from prior `nmss_magic32_origin` 16/16 close)

- Producer: `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` in `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so` (158 MB ARM64 ELF).
- Primitive: AES-128 over the Google PGS player ID; key derived from `PGSClientSecret` (string is present in libUnreal.so rodata).
- Symbol family identified in libUnreal.so: `EncryptPlayerIdKey`, `EncryptedPlayerIdKeys_Key`, `PlayerIds_Key`, `SignInPlayerIdForGooglePlay`, `EncryptedPlayerIdKeys`, `EncryptedPlayerIds`, `PGSClientSecret`, `PGSClientId`, `FPGSBinderAndroid`, `PGSIDProvider`, `GetPlayerId`.
- Output rendered as uppercase hex → MAGIC32 (32 ASCII chars).
- Persistence: written as `I_PID` field inside `CommonLogJson` in `cpp_native_shared.xml`, reloaded into native at `device_info+0x210`.
- **Residual gap (this campaign's target):** exact AES key bytes / KDF were NOT extracted. Whole rest of cert algorithm reduces to known constants once MAGIC32 is known.
- Closing artifact from origin: `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md`.
- Baksmali'd APK Java side: `/tmp/thered_baksmali_all/` (classes1/2/3) — contains the Java GameActivity that invokes the JNI.

## Next 2–3 concrete tasks

1. **Locate the JNI body.** In `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so`, find `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` via `nm -D --defined-only`, `readelf -s`, or `objdump -t`. Note its file offset and load-base offset. Disassemble the body with `objdump -d --start-address=<va> --stop-address=<va+0x4000> --no-show-raw-insn` or `r2 -A0` (use `aaa` then `pdf @ sym.Java_...`).

2. **Identify the AES primitive call.** Inside the JNI body, look for:
   - A BL into an AES routine (recognizable by access to a 176-byte expanded key schedule, or repeated XOR/SubBytes/ShiftRows/MixColumns patterns — or, more likely in libUnreal, a single BL into a wrapped helper that itself does the AES via NEON `aese`/`aesmc` instructions).
   - Track the key argument (typically `x1` or `x2` at the call) back to its source. Options: (a) a 16-byte buffer derived from `PGSClientSecret` via a KDF (likely a hash truncation — SHA1[:16] or SHA256[:16]); (b) a static `.data`/`.rodata` constant; (c) loaded from another JNI helper.
   - Track the plaintext argument back to the JNI `playerId` jstring argument and verify it's converted to bytes (GetStringUTFChars / GetStringChars) without modification before encryption. Check for any input padding/canonicalization (length, zero-pad to 16, hashing).

3. **Determine the key bytes.** If the key is a constant: dump those 16 bytes from the ELF. If it's derived: identify the derivation precisely (input bytes + transform). Find `PGSClientSecret` string in rodata via `strings -t x | grep -i pgsclientsecret`, then find xrefs from the JNI body to confirm the secret material chain.

4. **Acquire a ground-truth (player_id, MAGIC32) pair.** On the Waydroid device (`adb connect localhost:5558`):
   - Pull `/data/data/com.netmarble.thered/shared_prefs/cpp_native_shared.xml` and any GMS sharedprefs (`/data/data/com.google.android.gms/shared_prefs/com.google.android.gms.games.PLAYER_*.xml` or similar) to locate the raw Google PGS player ID.
   - Alternative: hook the **Java side** (allowed per memory rule — Frida-on-Java is OK via the `xerda` binary; Frida on libUnreal.so is FORBIDDEN due to anticheat) to capture the `playerId` argument that GameActivity passes to the JNI.
   - Target verification: encrypt the captured player_id with the extracted key, hex-upper, compare to `2FCF997702C244969BFEAF7F0D6AAA1C`.

5. **Wire into cert-rust-repro.** Add `compute_magic32` next to `compute_const_32b` in `cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs`. Add `aes = "0.8"` to the crate's Cargo.toml. Write a verification test. Write the closing artifact.

## Constraints & gotchas

- **No Frida on libUnreal.so** (memory rule — anticheat). Static analysis or ptrace-based HW-BPs only against the native lib. Frida on Java/system via `xerda` binary IS allowed and is the right tool for capturing the player_id input.
- The JNI symbol uses Unreal Engine's mangling; the *full* symbol may include trailing arg type tags. Search by prefix.
- libUnreal.so is 158 MB; full r2 analysis (`aaa`) is slow. Prefer `aaaa` only on the JNI function neighborhood (use `s` + `af`).
- AES in modern libs on aarch64 often uses NEON crypto extensions (`aese`, `aesmc`, `aesd`, `aesimc`). The disasm will show those instructions inline rather than a separate AES function — track the key buffer pointer through the `ld1` that loads the round keys.
- The PGS player ID is typically a numeric/alphanumeric opaque string (often 18–22 chars). AES-128 needs exactly 16 plaintext bytes. So there is almost certainly an input transform (truncate, hash-truncate, or pad). Identify it precisely.
- The original NMSS code may be UE4 macro-expanded; the symbol is mangled long-form.

## Relevant files / references

- `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so` — target ELF.
- `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md` — prior closing artifact with PGS string inventory.
- `/home/sdancer/nmss-emu/WIKI.md` — overall state of understanding.
- `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` — where `compute_magic32` lands.
- `/home/sdancer/nmss-emu-const32b-hwbp/analysis/const32b_numerical_repro_CLOSE.md` — sibling closing artifact (same template).
- `/tmp/thered_baksmali_all/` — Java side, in case you need to verify Java's call into JNI.
- Captured ground-truth: `MAGIC32 = "2FCF997702C244969BFEAF7F0D6AAA1C"` (analyzed session).
- Captured device snapshot (if needed for ptrace replays): `root@162.244.80.97:/root/nmss-emu-trampoline/trampoline_proc_memdump_5558/`.
- Tools available: `nm`, `readelf`, `objdump`, `r2` (radare2), `aarch64-linux-gnu-objdump`, `strings`, `xxd`, `adb`, `xerda` (Frida-on-Java).

## Falsification

Drop this path if any of the following holds after ≤3 planner cycles (~18 hours):
- The AES key is determined to be runtime-derived from data NOT statically present in libUnreal.so (e.g., requires live Google Play Services network response material beyond the static `PGSClientSecret` string).
- The "AES" pre-conclusion turns out to be wrong: disassembly shows the primitive is some other block cipher, an HMAC, or a custom rolling-hash construction.
- Worker cannot reach the JNI body's AES site at all (Unreal-internal indirection too deep for static, no symbols, no usable xref chain).

In those cases, hand off to a sibling path (HW-BP capture on the live process, or Frida-on-Java capture of both ends of the JNI for an oracle-style key search).
