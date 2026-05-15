# magic32-apk-strings-sweep — broad resource sweep for AES key candidates

## Role & workdir
Static-grep analyst. Workdir: `/home/sdancer/nmss-emu-magic32-apk-strings-sweep` (git worktree of `nmss-emu`, branch `magic32-apk-strings-sweep`). This is a parallel hedge running alongside `magic32-disasm` (deep static disasm of the JNI chain in libUnreal.so).

## Current goal / sub-goal
- `nmss_magic32_numerical_repro` — MAGIC32 reproduced offline from a Google PGS player ID.
- Sub-goal: find the AES-128 key (or any baked secret material) by **string/resource sweep** of the APK + native libs, complementing the call-graph chase done by the sibling worker `magic32-disasm`.

## Success criteria
Closing fact (binary 0/1): `nmss_magic32_numerically_reproduced`. For THIS path specifically, success looks like:
- A candidate 16-byte / 32-hex / 24-base64 string in APK resources or libUnreal.so rodata that, when used as AES-128-ECB key against the captured PGS player ID, produces `MAGIC32 = 2FCF997702C244969BFEAF7F0D6AAA1C`.
- Or, ruling that out: a definitive negative result documenting that no plausible candidate exists in static resources (forces the disasm path to find a runtime-derived key).

## Progress so far (sibling-path findings, for cross-pollination)

Sibling `magic32-disasm` (still running in parallel, in `/home/sdancer/nmss-emu-magic32-disasm`) has determined:
- JNI body at `0x57f7084` is a thin string-marshalling wrapper that submits a 0x30-byte heap work item via dispatcher `0x17fbc7c` to callback `0x5807954` (vtable `0x8cd04b8`).
- The callback reads `wrapper+0x8` with layout `{status@+0, str1@+0x8, str2@+0x18}` and forwards via a global singleton `0x99ece30` (lazy-init pattern, with ready-flag `0x99ece38`).
- Dispatch chain pinned `0x58069f8 → 0x5806a1c → 0x57f6900` (3-level wrapper indirection); chain is even deeper than that.
- `PGSClientSecret` string xrefs lead to **schema/serializer layer** (field registration at object offset 0x200), NOT directly into the crypto code. The AES key is likely **NOT a direct embedding of PGSClientSecret bytes** — could be derived from it, or could be an entirely different baked constant.
- Candidate AES site PCs explored (all turned out to be non-crypto helpers so far): `0x195b9f8`, `0x195f7f4`, `0x195fa80`, `0x195fc18`, `0x195bd54`, `0x195bdc4`.

Closing artifact from prior magic32_origin campaign: `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md`.

## Next 2–3 concrete tasks

1. **Sweep libUnreal.so rodata for AES key candidates.** 
   - Target ELF: `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so` (158 MB).
   - Extract `.rodata` and `.data.rel.ro` sections (use `readelf -S` to find offsets, then `dd` or `objcopy -O binary --only-section`).
   - Inside those sections, hunt:
     - 16-byte runs adjacent to or referenced near PGS-related strings (`PGSClientSecret`, `PGSClientId`, `EncryptPlayerIdKey`, `EncryptedPlayerIdKeys_Key`, `PlayerIds_Key`, etc.).
     - High-entropy 16-byte blocks (Shannon entropy ≥ 7.5 / 8) — AES keys are uniformly random.
     - Base64-encoded strings of length 22-24 (16-byte → base64) or 24 (with padding) near PGS context.
     - Hex strings of length 32 (16-byte hex-encoded).
   - For each candidate, note: file offset, surrounding context (32 bytes before/after), nearest string xref.

2. **Sweep the APK resource tree.** 
   - `/home/sdancer/tmp/nmss_apk/extract/` — full extracted APK. Search:
     - `assets/`, `res/raw/`, `res/values*/strings.xml`, AndroidManifest.xml meta-data tags.
     - `/tmp/thered_baksmali_all/` (classes1/2/3 baksmali) — `const-string` directives in any class that references PGS / I_PID / cpp_native_shared / CommonLogJson.
   - Look for any base64 / hex / opaque constants that could plausibly be a 16-byte AES key.

3. **Test candidates against ground truth.** 
   - Captured ground-truth: `MAGIC32 = "2FCF997702C244969BFEAF7F0D6AAA1C"` (the 16-byte AES-encrypted Google PGS player ID, hex-uppercased).
   - **You do NOT yet have the corresponding plaintext PGS player ID.** Sibling-path effort may surface it; you can also try to pull it from device sharedprefs (`adb pull /data/data/com.google.android.gms/shared_prefs/`) but adb may be down. Without plaintext, you cannot directly verify a key — but you CAN cross-check candidate keys against a corpus of plausible PGS player IDs (typical formats: numeric strings, base58 strings, etc.).
   - Alternative verification path: if you find a candidate key, attempt to use the captured MAGIC32 as ciphertext and decrypt back. If the result looks like a valid Google PGS player ID (~18-22 chars, alphanumeric), that's strong confirmation.

4. **Write closing artifact.** 
   - File: `<workdir>/analysis/task1_apk_strings_sweep_2026-05-14.md`.
   - Document: candidate keys found, surrounding-context evidence, any verification attempts, and falsification status (did the sweep find anything plausible, or did it rule out static-resource keys?).

## Constraints & gotchas

- **No Frida on libUnreal.so** (memory rule — anticheat). This is pure static; you won't touch the running process.
- This is a **PARALLEL HEDGE** to `magic32-disasm`. If you find a key first, that's a win for the campaign and the sibling can be retired. If you reach a clean negative, that strongly suggests the key is runtime-derived (KDF over PGSClientSecret with a salt or other input), which redirects the sibling's effort.
- `MAGIC32_ASCII` is the AES output rendered as 32 uppercase hex chars. The actual ciphertext bytes are the hex-decoded 16 bytes: `0x2FCF997702C244969BFEAF7F0D6AAA1C`.
- Be skeptical of obvious-looking strings; AES keys are often baked deep in `.data.rel.ro` or hidden in resources, not at obvious offsets.
- If you implement a candidate-testing script, write it in Rust or Python with the `aes` crate / `cryptography` library, AES-128-ECB single-block, NO padding (input is exactly 16 bytes).

## Relevant files / references

- `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so` — target ELF (158 MB).
- `/home/sdancer/tmp/nmss_apk/extract/` — full extracted APK.
- `/tmp/thered_baksmali_all/` — Java baksmali (classes1/2/3).
- `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md` — prior closing artifact with PGS string inventory.
- `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md` — sibling path's JNI body artifact (read for context).
- `/home/sdancer/nmss-emu/WIKI.md` — overall campaign state.
- `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` — where `compute_magic32` lands once we find the key.
- Captured ground truth: `MAGIC32 = "2FCF997702C244969BFEAF7F0D6AAA1C"`.
- Tools: `readelf`, `objcopy`, `dd`, `strings`, `xxd`, `python3 (cryptography)`, `cargo (aes crate)`, `rg`.

## Falsification

This path is killed if any of the following:
- 3 planner cycles (~18h) with no plausible candidate emerging from systematic sweep.
- The sweep produces clear evidence that the key cannot be a static baked constant (e.g., libUnreal.so has explicit code paths that derive the key via PBKDF2 / HKDF / hash-truncation from a non-static input).
- Sibling `magic32-disasm` closes the goal first (this hedge then naturally retires).
