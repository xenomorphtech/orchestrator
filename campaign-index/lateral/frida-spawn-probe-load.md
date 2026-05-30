# Lateral: Frida-spawn probe-load detection

**Wall**: `walls/frida-spawn-probe-load.md` — anti-debug killed many probe scripts on script-load. As of cycle 692/710, spawn-owned + attach-before-resume + RPC-agent shape AND spawn-owned + native-service loader BOTH survive script load. The remaining variant of this wall: trampoline-style hooks (Interceptor / Java.use intercept) that write code stubs into game/nmcore address space.

## Experiments

1. `[x]` **Spawn-owned + attach-before-resume + rpc.exports agent** — confirmed working cycle 692 (`findings/paths/spawn-mode-rpc-survives.md`).

2. `[x]` **Spawn-owned + native-service loader (System.load + dlopen)** — confirmed working cycle 710 (loader survives, see `tools/built/INDEX.md`). Java/activity readiness is the remaining tuning issue, not the wall.

3. `[ ]` **Memory.protect r-x → r-- as hookless interception** — proposed by fact-checker cycle 715, NOT YET TESTED. Theory: Frida's `Memory.protect(addr, size, 'r--')` strips execute permission from a target page; when the game tries to execute that page, the SIGSEGV is caught by Frida's exception handler. No trampoline / code stub is ever written into game pages — only page metadata changes, which the trampoline-scanner allegedly doesn't observe. Caveat: must run before anti-debug scanner initializes; target must be lazy-called code (not startup paths); exception handler must be registered before `resume()`. Worth testing in the cert-ptrace native-service direction once the readiness-polling fix lands. Real Frida pattern, not novel — but unverified for this NMSS build.

4. `[ ]` **Use Frida only for the bootstrap, drop after** (cert-ptrace's current direction) — `System.load()` from `Java.perform`, capture refs, then native code takes over with no further Frida activity. The `nmss_live_cert_service.so` lane embodies this. Effectively narrows the trampoline window to one-shot.

## Forbidden (until confirmed otherwise)
Hot-reloading these scripts into a spawned-and-resumed session still trips the wall:
- `nmss_java_probe_2026-05-02.js`, `nmss_native_cert_single_template_2026-05-02.js`, `nmss_java_attach_live_2026-05-02.js`, `nmss_java_attach_minimal_2026-05-02.js`, `live_sp968_spawn_2026-05-02.js`
