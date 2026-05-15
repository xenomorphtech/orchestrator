# dark-december-recon — Dark December protocol/cert reconnaissance

## Role & workdir
Reverse-engineering recon analyst on a fresh APK: **Dark December** by Needs Games Inc. Workdir: `/home/sdancer/dark-december` (fresh git-init dir, no parent worktree).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump` — recover the protocol / network / cert / auth flow used by Dark December, to the depth needed for offline reproduction of any anti-tamper or licensing cert it computes.
- Sub-goal (this path / first turn): **Initial reconnaissance** — download the APK, extract, inventory native libs and Java entry points, identify anti-tamper / crypto family used, and recommend the first deep-dive target.

## Success criteria
Closing fact: `dark_december_protocol_dumped` (full goal). For THIS path's task 1, success is producing a concrete recon doc that lets the next path pick a precise attack surface (e.g. "AES site at file_offset X in libNative.so, called from JNI Java_com_…", or "uses Themida wrapper at section .protect — fall back to dynamic", etc.).

## Next 2–3 concrete tasks

1. **Download the APK.** Source: https://www.apkmirror.com/apk/needs-games-inc/dark-december/dark-december-1-2-039-release/dark-december-1-2-039-android-apk-download/. APKmirror typically delivers via a redirect after a "Download APK" button — use `curl -L` with a real browser User-Agent. If you hit a captcha or interstitial, fall back to `wget` with `--user-agent` and document the failure. Save to `<workdir>/apk/com.needsgames.darkdecember-1.2.039.apk` (or whatever the canonical name is). Verify size and SHA256.

2. **Extract + inventory.** Unzip into `<workdir>/extract/`. Then:
   - `find extract/lib -type f -name "*.so"` — note arch dirs (arm64-v8a, armeabi-v7a, x86_64) and each lib's size.
   - `aapt2 dump packagename extract/` (or `aapt dump badging`) for package id + version.
   - `apkanalyzer manifest application-id extract/` for the entry-point Activity.
   - List `assets/`, `res/raw/`, and any `META-INF/CERT.*` (look for Themida/VMP/anti-debug indicators).
   - Run `strings` over each native lib and grep for anti-tamper markers: `Frida`, `xposed`, `ptrace`, `seccomp`, `_NSGetEnviron`, `gum-js`, `re.frida`, common Themida watermarks (`vmprotect`, `themida`, `wibu`), plus PGS/Google billing strings, AES instruction families, and known crypto-library identifiers (`libsodium`, `openssl`, `mbedtls`, `wolfSSL`).

3. **Identify the dominant anti-tamper family + crypto primitives.** From the inventory:
   - Is there a clearly named native lib (e.g. `libNative.so`, `libgame.so`, `libUnity.so`) that holds the protocol logic?
   - Is it a Unity / Unreal / GameMaker / Cocos build? Each has different RE conventions.
   - Are there obvious vendor binaries (Naver / Netmarble / Kakao SDK, Nexon antifraud)?
   - Look for AES family symbols and the same JNI pattern we saw in NMSS: `Java_*_OnGet*`, `Encrypt*`, `*ClientSecret`. If present, NMSS-style recon applies; otherwise document the divergence.

4. **Write the recon doc.** `<workdir>/analysis/recon_2026-05-15.md` with:
   - APK metadata (package id, version, SHA256, size, native arch availability).
   - Native lib inventory (path + size + brief role guess).
   - Anti-tamper family + evidence.
   - Crypto primitives detected.
   - **Recommended attack surface for next path** (concrete file + symbol + reasoning).
   - Open questions worth a follow-up path.

## Constraints & gotchas

- **No Frida on anything that looks anticheat-protected.** Static disasm + manual run first. If the native lib uses Themida/VMP, document and flag — don't try to run it inside a Frida agent without authorization.
- This is **fresh recon on a different game** from NMSS — don't import NMSS assumptions blindly. Note any structural similarity to NMSS in the report so the orchestrator can decide whether to reuse infrastructure (e.g., aeon datalog, oracle service patterns).
- If the APK is signed / DRM-wrapped, the `apk` we download from APKmirror is the **app bundle base** — note this and check for split APKs (split_config.* etc.).
- Use the **systemd-isolated worker pattern**: the harness wrapper at `/home/sdancer/orchestrator/bin/harness-spawn-worker` runs you under `/system.slice/system-harness-worker.slice/harness-worker@dark-december-recon.service`. Per-service MemoryMax=24G. If you spawn aeon-mcp via codex, it'll be charged to this cgroup.
- The orchestrator currently has NMSS workers (magic32-disasm, magic32-apk-strings-sweep, aeon-memory-investigation) running concurrently. Be mindful of shared compute resources — the box is 124 GB RAM but aeon can balloon (NMSS's magic32-disasm currently uses 21 GB for libUnreal.so analysis).

## Relevant files / references

- APKmirror page: https://www.apkmirror.com/apk/needs-games-inc/dark-december/dark-december-1-2-039-release/dark-december-1-2-039-android-apk-download/
- For comparison / methodology, the NMSS recon artifacts:
  - `/home/sdancer/nmss-emu/WIKI.md`
  - `/home/sdancer/nmss-emu-magic32-strings/analysis/magic32_prefs_writer.md` (origin chain template)
  - `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md` (JNI body decomposition template)
- Tools available: `unzip`, `aapt2` / `apkanalyzer`, `apksigner`, `strings`, `readelf`, `nm`, `llvm-objdump`, `aarch64-linux-gnu-objdump`, `python3 (with cryptography)`, `r2` (radare2), `curl`/`wget`, `sha256sum`.
- No adb / live-device assignment yet — this is purely static recon. Live-device work waits for a later sub-goal if needed.

## Falsification

This recon path is killed if:
- APK cannot be downloaded after 2 attempts (different download mirrors / sources) — escalate to user for the APK file.
- After full inventory + strings + manifest, NO crypto / network / protocol surface is identifiable (game is pure-client with no auth/server — different problem class entirely).
- 3 cycles produce no recon doc → retire with falsified conclusion + reframe.

After task 1 completes, the orchestrator will decide whether to fork specific deep-dive paths (e.g., `dark-december-jni-disasm`, `dark-december-strings-sweep`) modeled on the NMSS portfolio structure.
