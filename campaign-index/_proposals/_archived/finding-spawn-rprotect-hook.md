# Proposal: New finding — spawn-mode r-- page-fault interception works

**Action**: Add `findings/paths/spawn-rprotect-hook.md`

## Finding: Memory.protect r-x→r-- as hookless interception in Frida spawn

**Confirmed**: 2026-05-02

Setting a target page's protection from `r-x` to `r--` via `Memory.protect(addr, size, 'r--')` in a Frida spawn session causes a fault when the game attempts to execute that page. Catching the fault via Frida's process exception handler (or equivalent) allows interception without placing any trampoline or code stub in game/nmss address space.

## Why it bypasses anti-debug
NMSS trampoline detection scans for Frida JS runtime stubs and inline hooks written into code pages. A permission-stripping approach writes nothing into the code — only changes page metadata — so it does not match the hook signature the detector looks for.

## Applicability
- Only viable in spawn mode (must call `Memory.protect` before the anti-debug scanner initializes)
- Target must be a page that is executed after the protection change (i.e. lazy-called code, not startup paths)
- Exception handler must be registered in the same script before `resume()`

## What to do with it
Use this technique in the cert-ptrace / rpc-agent direction to intercept `nmssNativeGetCertValue` without triggering the kill.
