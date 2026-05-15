# magic32-gms-state-read — Extract PGS playerId from Google Play Services local storage

## Role & workdir
Offline Google-side state miner. Workdir: `/home/sdancer/nmss-emu-magic32-gms-state-read` (worktree of `/home/sdancer/nmss-emu`, branch `magic32-gms-state-read`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: recover the raw Google PGS playerId (companion to captured MAGIC32 `2FCF997702C244969BFEAF7F0D6AAA1C`) by reading **Google Play Services' own local storage** on the device — bypassing NMSS entirely.

## Success criteria
Closing fact: `nmss_magic32_numerically_reproduced`. Concrete deliverable:
- (playerId, MAGIC32) ground-truth pair extracted from GMS-side state.
- AES-128 key derivation that produces MAGIC32 from playerId, verified end-to-end.
- Pure-Rust `compute_magic32(pgs_player_id: &[u8]) -> [u8; 16]` in `cert-rust-repro` + passing test.

## Why this is distinct from the 5 falsified paths

The previous 5 paths all attacked NMSS's address space (libUnreal.so, NMSS heap, NMSS process Java method table). GMS runs in `com.google.android.gms` — a completely different UID/process. **NMSS's Hercules anticheat has no visibility into GMS storage.** This path requires zero instrumentation of the protected process.

Falsified sibling paths (read closing artifacts):
- `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md` — chain depth 4+, key runtime-derived.
- `/home/sdancer/nmss-emu-magic32-apk-strings-sweep/analysis/task2_broader_sweep_2026-05-14.md` — key NOT a baked constant.
- `/home/sdancer/nmss-emu-magic32-hw-bp/analysis/task2_aes_hwbp_capture_2026-05-14.md` — cached-login skips producer.
- `/home/sdancer/nmss-emu-magic32-java-blackbox/analysis/...` — NMSS blocks Java.use() introspection.
- `/home/sdancer/nmss-emu-magic32-snapshot-mining/analysis/...` — NMSS heap had no PGS string format match.

## Next 2–3 concrete tasks

1. **Inventory GMS state directories.**
   - `adb root` confirmed on this Waydroid.
   - `adb shell ls /data/data/com.google.android.gms/` — look for `databases/`, `shared_prefs/`, `files/`, `cache/`.
   - `adb shell ls /data/user/0/com.google.android.gms/databases/` — common locations: `games.db`, `games_<n>.db`, `playgames.db`, `gservices.db`, `metadata.db`.
   - `adb shell ls /data/user/0/com.google.android.gms/shared_prefs/` — look for files containing `players`, `games`, `auth`, `account`.
   - Also check `/data/data/com.google.android.play.games/` if separate Play Games app installed.

2. **Pull every plausible candidate file + grep for PGS playerId format.**
   - PGS playerId format: `g<17-22 digit decimal>` ASCII string OR sometimes `gPID:<digits>` OR base64 of an integer.
   - Pull approach: `adb pull /data/user/0/com.google.android.gms/databases/games.db ./gms-state/` (for each candidate).
   - For SQLite files: `sqlite3 games.db .dump | rg 'g[0-9]{17,22}'` — see what rows hold the playerId.
   - For XML: `grep -E 'g[0-9]{17,22}' shared_prefs/*.xml`.
   - For binary: `strings -n 20 file | rg 'g[0-9]{17,22}'`.

3. **Cross-reference with the device's signed-in account.**
   - `adb shell dumpsys account | head -50` — shows currently-signed-in Google accounts (email level).
   - The MAGIC32 was captured against ONE specific Google account (the one the NMSS app used for PGS sign-in). If multiple accounts are present on the device, the right playerId is the one belonging to the account that was used.
   - `adb shell content query --uri 'content://com.google.android.gms.games.provider/games_owners'` (if exposed) — direct query.

4. **Algorithm sweep.** Once a candidate playerId is found, run the standard AES-128 derivation sweep against MAGIC32 `2FCF997702C244969BFEAF7F0D6AAA1C`:
   - `key = sha256(playerId)[:16]` → AES-128-ECB(key, playerId[:16] zero-padded) =?
   - `key = sha256(PGSClientSecret_candidate || playerId)[:16]` — PGSClientSecret candidate strings in libUnreal.so rodata from prior magic32-strings work.
   - `key = HKDF-SHA256(salt=playerId, ikm=device_id, info='magic32')` — Android device_id readable via `Settings.Secure.ANDROID_ID`.
   - `key = MD5(playerId || PGSClientSecret)[:16]`.
   - Plaintext padding: try truncate-to-16, zero-pad-to-16, PKCS#7, MD5(playerId).
   - Verification: AES-128-ECB(key, plaintext) → hex_upper == `2FCF997702C244969BFEAF7F0D6AAA1C`.
   - On match: implement in pure Rust at `/home/sdancer/nmss-emu/cert-rust-repro/src/magic32.rs`, add test, set fact `nmss_magic32_numerically_reproduced`.

5. **If GMS-side state yields the playerId AND key derivation matches**: this path closes the entire stalled-meta goal. Write `<workdir>/analysis/gms_state_read_2026-05-14.md` with: file inventory, exact playerId, the derivation that matched, the test result.

## Constraints & gotchas

- **adb root + SELinux permissive** confirmed on this Waydroid per prior magic32-hw-bp campaign.
- **No instrumentation of `com.netmarble.thered`** — this path's whole premise is NOT touching the protected app. Do not run Frida, ptrace, or hooks against NMSS.
- **GMS may encrypt some state at-rest** — Google Account credentials, OAuth tokens are encrypted with `accounts.db.key` (a per-device key). However, the PGS playerId is typically NOT considered secret and is stored in cleartext for offline access. If the candidate db is encrypted, look at adjacent files — caches and metadata stores often contain plaintext copies.
- **Multiple PGS player IDs**: a device with multiple Google accounts has multiple PGS playerIds. The MAGIC32 in `cpp_native_shared.xml` was generated against the account that was active at NMSS sign-in time. Use timestamps + `dumpsys account` to cross-match.
- **Captured MAGIC32**: `2FCF997702C244969BFEAF7F0D6AAA1C` (16 bytes). The sharedprefs source file is `/data/data/com.netmarble.thered/shared_prefs/cpp_native_shared.xml` — DO NOT read it via xerda/Frida; `adb shell cat` is fine since cat doesn't touch NMSS's address space.
- This worker runs under systemd `harness-worker@magic32-gms-state-read.service` (system.slice, MemoryMax=24G).

## Relevant files / references

- Sibling artifacts (do read first):
  - `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md`
  - `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md` (lists PGSClientSecret etc. in libUnreal.so rodata)
- cert-rust-repro at `/home/sdancer/nmss-emu/cert-rust-repro/`.
- Captured MAGIC32: `2FCF997702C244969BFEAF7F0D6AAA1C`.
- Tools: `adb`, `sqlite3`, `xxd`, `strings`, `rg`, `python3 (cryptography)`, `cargo`.

## Falsification

- Exhaustive search of `/data/data/com.google.android.gms/` (databases, shared_prefs, files, caches) yields NO `g[0-9]{17,22}` matches AND no other plausible PGS playerId format.
- A playerId IS found but exhaustive sweep of ≥1000 AES-128 key derivations does not produce MAGIC32. (In that case the artifact still documents the captured playerId — useful for subsequent paths.)
- GMS storage is fully encrypted on this Waydroid build (unusual but possible).

If falsified: escalate to the next path in the divergence list — `magic32-pre-launch-frida` (spawn-mode injection).
