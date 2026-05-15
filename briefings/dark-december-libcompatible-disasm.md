# dark-december-libcompatible-disasm — Static disasm of libcompatible.so (Dark December)

## Role & workdir
Native binary analyst on Dark December's `libcompatible.so`. Workdir: `/home/sdancer/dark-december-libcompatible-disasm` (worktree of `/home/sdancer/dark-december`, branch `libcompatible-disasm`).

## Current goal / sub-goal
- Goal: `dark_december_protocol_dump` — reverse-engineer Dark December's protocol/cert/auth flow.
- Sub-goal (this path): decompose `libcompatible.so` — the protection/auth boundary between libUE4.so and the INCA AppGuard layer.

## Success criteria
Closing fact: `dark_december_libcompatible_decomposed`. Concrete deliverable:
- Inventory of all exported JNI symbols (de-obfuscated where possible).
- Localized PCs for `rsaEncryption`, `oJWT`, `asm_ptrace` callsites.
- Identification of the cert/auth flow signature: which JNI calls hand off to the Hive auth stack vs the INCA AppGuard.
- Recon artifact at `<workdir>/analysis/libcompatible_disasm_2026-05-15.md` with: function bounds for the top 5 highest-leverage symbols, dataflow sketch for the auth/cert path, and recommended next sub-target.

## Progress so far (from dark-december-recon task 1)

Closing artifact from recon: `/home/sdancer/dark-december/analysis/recon_2026-05-15.md`. Key facts:
- Game engine: UE4 4.26.2 shipping build (`RzGame`).
- Protection family: **INCA AppGuard / Hercules** (Korean anticheat).
- Auth family: **Hive** (Korean game auth/social SDK by Com2uS-spinoff Smilegate-like vendor); visible endpoints `/api/login`, `/api/preLogin`, `/api/oauth/token`, `/api/player/get-session`, plus `GetSocketToken`.
- APK: `dark-december-1.2.039.xapk` SHA256 `12a15315601eafb8314a1594e187213c0574b44602923661d6d10053b59577e0`.
- `libcompatible.so` location: `extract/arm64/lib/arm64-v8a/libcompatible.so` (under the unpacked xapk).
- Strings present: `rsaEncryption`, `oJWT`, `asm_ptrace` (anti-debug indicator — ptrace-syscall blocker).

## Next 2–3 concrete tasks

1. **Inventory.** From `<workdir>/extract/arm64/lib/arm64-v8a/libcompatible.so`:
   - `readelf -W -s` — full symbol table (note GOT/PLT vs defined).
   - `nm -D --defined-only` — exported JNI symbols. JNI exports look like `Java_com_…_methodName`.
   - `strings -t x | rg -i 'jni|hive|inca|appguard|rsa|jwt|oauth|ptrace|cert|key|secret|token'` — annotate string offsets.
   - Note: section layout (`readelf -W -S`), notable PT_LOAD segments, RELRO posture.

2. **Localize the named strings.** For each of `rsaEncryption`, `oJWT`, `asm_ptrace`, `GetSocketToken`:
   - File offset of the string.
   - All code xrefs (use `aeon get_xrefs` if needed; otherwise `objdump -d | rg <hex addr>` with adrp+add patterns).
   - Enclosing function bounds + a brief role-classification (is this where the crypto happens, or just metadata?).

3. **Sketch the auth/cert dataflow.** Trace from one JNI entry point (pick the one closest to `rsaEncryption` xrefs) outward:
   - What arguments come in (jstring / jbyteArray / jint sizes)?
   - What does the function do with them (RSA encrypt? JWT sign? HMAC?)?
   - Where does the output go (returned to Java? written to a JNI field? POSTed via the HTTP stack?)?
   - Compare with NMSS structure as a methodology baseline — note divergences. The NMSS pattern was JNI → marshalling → work-item submission → vtable dispatch → AES. If libcompatible follows a similar pattern, that helps prediction.

4. **Write the artifact.** `<workdir>/analysis/libcompatible_disasm_2026-05-15.md` with sections matching tasks 1–3 + a clear recommendation for the next sub-target (e.g., "trace the RSA private-key source" or "instrument the Hive HTTP endpoint" or "investigate INCA AppGuard's anti-frida hooks").

## Constraints & gotchas

- **No Frida on libcompatible.so** — INCA AppGuard is a Korean anticheat that aggressively detects Frida and other instrumentation. Stay static.
- The 158 MB libUnreal.so analysis in NMSS taught us that aeon's whole-binary closures are expensive. **libcompatible.so is much smaller** but if it depends on libUE4.so (the recon report says it does), aeon queries that follow that linkage may blow up memory. Prefer narrow address-window aeon queries (`get_function_at <pc>`) over `find_call_paths` or `call_graph_transitive` on whole-binary scope.
- This worker runs under systemd `harness-worker@dark-december-libcompatible-disasm.service` in `/system.slice/system-harness-worker.slice/` with MemoryMax=24G. If you hit OOM on aeon, fall back to `llvm-objdump -d` over small address windows.
- The Hive auth Java side is the **secondary** target — out of scope for this path. Note it in your recommendation but don't drift into Java work.

## Relevant files / references

- APK extraction root: `/home/sdancer/dark-december/extract/` (xapk unpacked).
- Target ELF: `/home/sdancer/dark-december/extract/arm64/lib/arm64-v8a/libcompatible.so` (verify path; recon may have placed it differently — check `find /home/sdancer/dark-december -name libcompatible.so`).
- Recon artifact (read first): `/home/sdancer/dark-december/analysis/recon_2026-05-15.md`.
- NMSS methodology reference: `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md` — for JNI body decomposition style.
- Tools: `readelf`, `nm`, `objdump`, `llvm-objdump`, `strings`, `aarch64-linux-gnu-objdump`, `r2`, `aeon` MCP (narrow queries only).

## Falsification

- `libcompatible.so` is not at the recon-named path OR its size is trivial (<100 KB, just a shim) → escalate, the recon may have misidentified the target.
- 3 cycles produce no decomposition artifact → retire and let the planner propose alternatives (e.g., libUE4.so direct, or Java-side Hive trace).
- The named strings (`rsaEncryption`, `oJWT`, `asm_ptrace`) are present as dead-string-pool entries with no code xrefs (the strings exist but aren't used) → reframe; the auth/protection layer is somewhere else, possibly in a sibling lib.
