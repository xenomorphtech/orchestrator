# Proposal: Correct frida-spawn-probe-load.md — spawn mode is not fully walled

**Action**: Replace/correct `walls/frida-spawn-probe-load.md`
**Reason**: Current leaf overstates the block. Spawn mode works; the actual constraint is narrower.

## Corrected understanding

Frida spawn mode (`frida -U -f com.netmarble.thered`) is viable. The kill is triggered by **hooking game or nmss library functions via trampolines**, not by script load itself. Scripts that avoid trampoline-style hooks survive.

## What is actually forbidden
- Frida Interceptor / Java.use hook trampolines in nmss/game address space
- Loading scripts that place code stubs in game/nmcore pages

## What is permitted in spawn mode
- Memory reads (`Memory.readByteArray`, `Process.enumerateRanges`, etc.)
- **Memory permission stripping**: `Memory.protect(addr, size, 'r--')` to make a page non-executable, then catching the resulting fault via Frida's exception handler — confirmed working as a hookless interception technique that bypasses the trampoline detector
- RPC exports that don't hook game/nmss libs
- Calling non-nmss system APIs

## Forbidden scripts (still applies — these use trampolines)
- `nmss_java_probe_2026-05-02.js`
- `nmss_native_cert_single_template_2026-05-02.js`
- `nmss_java_attach_live_2026-05-02.js`
- `nmss_java_attach_minimal_2026-05-02.js`
- `live_sp968_spawn_2026-05-02.js`
