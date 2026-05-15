# magic32-java-blackbox — Java-side black-box pair collection for MAGIC32 derivation

## Role & workdir
Java-side dynamic analyst. Workdir: `/home/sdancer/nmss-emu-magic32-java-blackbox` (worktree of `nmss-emu`, branch `magic32-java-blackbox`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: bypass the deep static dispatch chain and the cached-login HW-BP miss by collecting **(plaintext_PGS_player_id, MAGIC32)** observation pairs from black-box Java-side hooks, then **algorithm-search** over candidate derivations to identify the AES key derivation function offline.

## Success criteria
Closing fact: `nmss_magic32_numerically_reproduced`. Concrete deliverable:
- ≥3 verified (player_id, MAGIC32) pairs from independent app sessions.
- Algorithm derivation that maps player_id → MAGIC32 for ALL captured pairs, expressed as `compute_magic32(pgs_player_id: &[u8]) -> [u8; 16]` in pure Rust.
- Test passing in `cert-rust-repro` for the captured pairs.

## Progress so far

Sibling paths' findings inform this approach:
- **magic32-disasm** (retired-stalled): chain depth 4+, googleClientSecret confirmed at payload+0x78 in serializer 0x57f6900, AES PCs `{0x195b9f8, 0x195be04}` confirmed as pure AESE/AESIMC. Key is runtime-derived, NOT a baked constant.
- **magic32-apk-strings-sweep** (retired-falsified): definitively confirmed AES key is NOT in any static lib/resource.
- **magic32-hw-bp** (retired-falsified): HW-BP infrastructure works (helper at `/home/sdancer/nmss-emu-magic32-hw-bp/aes_hwbp_capture.c` + validator) but cached-login startup path skipped the producer on 3 fresh-launch attempts.
- Captured ground truth (one pair from original analyzed session): MAGIC32 = `2FCF997702C244969BFEAF7F0D6AAA1C`. The corresponding player_id was NEVER captured in earlier work.

## Next 2–3 concrete tasks

1. **Frida-on-Java hook on the JNI caller.** Memory rule: Frida on Java/system is ALLOWED via `xerda` binary; Frida on libUnreal.so is FORBIDDEN.
   - Target: `com.epicgames.unreal.GameActivity.OnGetPGSPlayerIdWithAuthCode(String playerId, String authCode)` — the Java method that hands the player_id into JNI.
   - Hook capture: timestamp, playerId argument, authCode (just length — don't leak), thread/stack.
   - The hook must fire on app launch when PGS sign-in completes.

2. **Collect MAGIC32 from sharedprefs after the JNI returns.**
   - After hook fires, read `/data/data/com.netmarble.thered/shared_prefs/cpp_native_shared.xml` via `adb shell cat` (need `adb root`).
   - Parse XML, find `CommonLogJson`, parse the JSON, extract `I_PID` field → MAGIC32 hex.
   - Pair = `(playerId_captured_at_hook, MAGIC32_read_from_xml)`.
   - **Important**: cached login may skip producing a NEW MAGIC32 — sharedprefs may already hold the cached value. To force regeneration: `adb shell pm clear com.netmarble.thered` then relaunch. Each `pm clear` + relaunch + (Google sign-in if needed) = one independent session = one independent pair.
   - Repeat ≥3 times. Different player IDs across sessions are NOT expected (same Google account = same PGS player ID), so pairs will have IDENTICAL inputs. That's still useful as the FIRST verification step (consistency). For ALGORITHM identification we need either (a) multiple Google accounts, OR (b) cleverness with the derivation.

3. **Algorithm search.** With even ONE (playerId, MAGIC32) pair, attempt these standard AES-128 key sources:
   - `key = sha256(playerId)[:16]`
   - `key = sha256(PGSClientSecret_baked || playerId)[:16]` — need to find PGSClientSecret bytes in libUnreal.so rodata (search by string-context, then 32-byte windows nearby).
   - `key = HKDF-SHA256(salt=??, ikm=playerId or device_id, info='magic32')`
   - `key = PBKDF2(salt=device_id, password=playerId, iter=N)`
   - For each candidate key: `AES-128-ECB(key, playerId_bytes_padded_to_16)` and compare hex_upper to MAGIC32. If match → derivation found.
   - The playerId is typically a numeric string like `g1234567890123456789`. Plaintext padding/encoding matters: try truncate-to-16, zero-pad-to-16, PKCS#7-padded-with-block-count-of-1.

4. **If algorithm found**: write `compute_magic32()` in `cert-rust-repro` and verify against all captured pairs. Set fact `nmss_magic32_numerically_reproduced`.

## Constraints & gotchas

- **No Frida on libUnreal.so** (anticheat). Frida on Java is OK via `xerda`.
- `adb root` + SELinux permissive confirmed on this Waydroid per magic32-hw-bp campaign.
- Forcing re-auth via `pm clear` will trigger Google account selection — may not auto-complete on a headless Waydroid. Test manually first; if blocked, adapt (could use `adb input tap` for Google selection screen).
- This worker runs under systemd `harness-worker@magic32-java-blackbox.service` in `system.slice` with MemoryMax=24G.

## Relevant files / references

- Sibling artifacts (read first):
  - `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md`
  - `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md` (the original 16/16 close that traced the producer chain)
  - `/home/sdancer/nmss-emu-magic32-hw-bp/analysis/task2_aes_hwbp_capture_2026-05-14.md` (helper code reference if you need it for cross-checking)
- `cert-rust-repro` at `/home/sdancer/nmss-emu/cert-rust-repro/` (already has `compute_const_32b`; this path adds `compute_magic32`).
- Captured ground-truth MAGIC32 from prior analysis: `2FCF997702C244969BFEAF7F0D6AAA1C` (need its corresponding playerId).
- Tools: `xerda` (Frida-on-Java), `adb`, `python3 (cryptography, aes crate via cargo)`, `xmllint`/`grep` for sharedprefs XML.

## Falsification

- Frida-on-Java hook can't attach (anticheat detects xerda) for 2 fresh attempts.
- ≥3 pairs collected and NO standard derivation (SHA-trunc, HKDF, PBKDF2 over reasonable salts) yields the captured MAGIC32 within a 1000-candidate sweep.
- `pm clear` + Google sign-in flow can't be automated on Waydroid (account picker stalls) → escalate (user-supplied test account or pre-completed sign-in flow needed).
