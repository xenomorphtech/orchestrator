# Active blockers (cycle 687)

## P0 — title-screen tap gate
App is launched and stable but won't proceed past Vampir title screen until tapped. `adb shell input tap` doesn't work (see `walls/adb-input-tap.md`). **Needs physical tap on device.**

## P1 — Frida-based probing is hardened
All Frida injection paths fast-kill on script load (see `walls/frida-spawn-probe-load.md`). Even the once-working `nmss_java_probe_2026-05-02.js` now fails. Active workaround in flight: building proper RPC agent + Python driver from in-tree trampoline pattern (see `open/rpc-agent-build.md`).

## P2 — no autonomous probe path validated post-tap
After tap, we don't yet know which probe lane will work. Plan: try the RPC agent first; if walled, fall back to non-Frida (`dumpsys`, `am broadcast`, content provider, `nmsscr.dec` cache).

## Resolved
None of the algorithmic work is currently blocked — that's complete (see `findings/algorithm/`). The blocker is purely instrumentation: getting one valid native_cert per challenge from a live authenticated session.
