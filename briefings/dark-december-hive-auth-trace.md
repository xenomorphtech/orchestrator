# dark-december-hive-auth-trace — Java-side Hive auth trace

## Role & workdir
Java-side reverse-engineering analyst on Dark December's Hive auth stack. Workdir: `/home/sdancer/dark-december-hive-auth-trace` (worktree of `/home/sdancer/dark-december`, branch `hive-auth-trace`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump` — recover Dark December's protocol/auth flow.
- Sub-goal (this path): characterize the **Java-side Hive auth client** in `classes2.dex` and `classes4.dex` — the upstream end of the auth/cert flow that feeds the native side.

## Success criteria
Closing fact: `dark_december_protocol_dumped` (full goal). For THIS path:
- Map every Hive auth endpoint (`/api/login`, `/api/preLogin`, `/api/oauth/token`, `/api/player/get-session`, `GetSocketToken`) to its calling Java class + method.
- Identify the cleartext data shapes (request/response) being passed across each endpoint.
- Identify which Java-side method calls into native (libcompatible.so / libUE4.so) and what JNI args it passes.
- Produce `<workdir>/analysis/hive_auth_trace_2026-05-15.md` with the auth-state-machine diagram, endpoint inventory, JNI boundary shape, and recommendation for the next deep-dive.

## Progress so far (cross-pollination from sibling dark-december paths)

- **dark-december-recon** (closed): UE4 4.26.2 build `RzGame`. INCA AppGuard/Hercules protection. Hive auth stack. APK SHA256 `12a15315601eafb8314a1594e187213c0574b44602923661d6d10053b59577e0`. Closing artifact: `/home/sdancer/dark-december/analysis/recon_2026-05-15.md`.
- **dark-december-libcompatible-disasm** (just closed, partial-falsification): libcompatible.so is the **anti-debug/protection layer, NOT the auth crypto host**. The 3 named strings (`rsaEncryption`, `oJWT`, `asm_ptrace`) are OID/syscall-name metadata; no AES/SHA/PMULL in xref helpers. **Real auth crypto lives elsewhere — likely libUE4.so OR Java-side.** Closing artifacts: `/home/sdancer/dark-december-libcompatible-disasm/analysis/libcompatible_disasm_2026-05-15.md` + `task2_xrefs_2026-05-15.md`.
- Endpoints visible from recon: `/api/login`, `/api/preLogin`, `/api/oauth/token`, `/api/player/get-session`, `GetSocketToken`.

## Next 2–3 concrete tasks

1. **Baksmali extraction + Hive-class location.**
   - Unpacked APK root: `/home/sdancer/dark-december/extract/` (xapk contents).
   - Baksmali `classes2.dex` and `classes4.dex` into `<workdir>/baksmali/{classes2,classes4}/`.
   - Find the Hive package — typical Java packages: `com.hive.*`, `com.com2us.*`, `com.gamevil.*`. Use `rg -l 'preLogin|getSession|GetSocketToken|hive'` over the baksmali tree to locate.
   - Identify the **Hive auth client** class — the one that aggregates the 5 endpoints. Map it to its file path.

2. **Per-endpoint method trace.** For each of the 5 endpoints:
   - Find the Java method that issues the HTTP call (search for the literal endpoint string).
   - Walk backward to find: who calls it, what parameters they pass.
   - Walk forward: how the response is parsed (gson/jackson/manual JSON), what fields it populates.
   - Note any signature / encryption that happens **before** the HTTP call (e.g., HMAC-SHA256 of body, RSA encrypt of password, JWT sign).

3. **JNI boundary.** Search for `native` methods in the Hive package (smali `.method native`) and for JNI calls into `libcompatible.so` (search for `System.loadLibrary("compatible")` or class names that look like protection bridges):
   - List every native method and which Java caller invokes it.
   - For methods with crypto-looking signatures (returning byte[] when taking String, or vice versa), note them as primary candidates.
   - Cross-pollination: sibling libcompatible-disasm task 2 recommends two specific JNI registration table entries — `0x1254a4` (`E270F21E499946b6B8D8`, `(Ljava/lang/String;Ljava/lang/String;)Z`) and `0x12381c` (`EC8A3DFF8EAB09E43C9550`, `(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;Ljava/lang/ClassLoader;II)I`). Find the Java methods that invoke these specific signatures.

4. **Write the artifact.** `<workdir>/analysis/hive_auth_trace_2026-05-15.md` with: per-endpoint methods + payload shapes + JNI calls + auth state-machine sketch + recommendation for the next deep-dive (likely the JNI native method that signs/encrypts the auth payload).

## Constraints & gotchas

- **Frida on Java/system is ALLOWED** per memory rule (only libUnreal.so is forbidden in NMSS; for Dark December, INCA AppGuard's anti-frida is what we need to avoid — same rule: NO Frida on libcompatible.so or libUE4.so. Frida-on-Java is the workaround, but try static-only first).
- This is purely static (baksmali + grep + read smali). No device interaction needed for task 1; Frida-on-Java can come in a follow-up task if static doesn't yield enough.
- Hive SDK is a known Korean game auth library; published documentation may exist online — feel free to look up the SDK's public class structure to accelerate orientation (but verify against the actual baksmali, since obfuscation may have renamed methods).
- This worker runs under systemd `harness-worker@dark-december-hive-auth-trace.service` in `system.slice` with MemoryMax=24G.

## Relevant files / references

- APK extracted: `/home/sdancer/dark-december/extract/`.
- Sibling path artifacts (read first):
  - `/home/sdancer/dark-december/analysis/recon_2026-05-15.md`
  - `/home/sdancer/dark-december-libcompatible-disasm/analysis/libcompatible_disasm_2026-05-15.md`
  - `/home/sdancer/dark-december-libcompatible-disasm/analysis/task2_xrefs_2026-05-15.md`
- Tools: `baksmali`, `apktool`, `rg`, `python3`, `aapt`, `unzip`. Frida-on-Java via `xerda` binary (only if static is insufficient).

## Falsification

- Hive endpoints are pure-Java wrappers passing opaque blobs to JNI with no inspectable Java-side state (3 cycles fail to identify cleartext data shapes).
- Hive class names are heavily obfuscated AND the SDK's public docs don't match the structure → 3-cycle static stall.
- Auth flow turns out to be 100% native (Java only triggers a JNI call that does everything internally) → in that case, retire and refocus on libUE4.so disasm.
