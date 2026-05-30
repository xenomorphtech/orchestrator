# Wall: Frida-spawn with probe script → instant kill (NARROWER scope as of cycle 692)

**Confirmed**: cycle 666, 2026-05-02
**Bypass discovered**: cycle 692, 2026-05-02 — see `findings/paths/spawn-mode-rpc-survives.md`

## ⚠️ Scope clarification (cycle 692)
This wall applies to **hot-reload of one-shot probe scripts** into a spawn-owned session, NOT to all Frida-spawn approaches. The breakthrough: **spawn-owned + attach-before-resume + rpc.exports-shaped agent** survives script load. See `findings/paths/spawn-mode-rpc-survives.md`.

What still trips the wall: any of the legacy probe scripts (`nmss_java_probe_2026-05-02.js`, `nmss_native_cert_single_template_2026-05-02.js`, etc.) hot-reloaded into a session that was spawned and resumed normally.

## Symptom
```
$ frida -U -f com.netmarble.thered -l <probe-script>.js -q -t inf
Spawned `com.netmarble.thered`. Resuming main thread!
... (a few lines of agent output) ...
Process terminated
```

Both `com.netmarble.thered` and `nmcore` then absent from `pidof`.

## Scope
- **Hot-reload of any non-trivial script** into a spawn-owned session: instant kill
- **Even the once-positive script** `nmss_java_probe_2026-05-02.js` (which produced the only live cert 7BDA → 3763E965 in cycle 633) now triggers this
- The trivial no-op `stealth_spawn_keepalive_2026-05-02.js` survives ~10–30 min before the standard watchdog kill (different mechanism)

## Why
NMSS anti-debug has been hardened (between cycles 633 and 666) to detect Frida instrumentation on script load — likely scanning for Frida JS runtime stubs or hook trampolines. The cycle-633 success was a transient race that's no longer reproducible.

## What to try instead
1. **Spawn-owned RPC agent with attach-before-resume** (current direction, see `open/rpc-agent-build.md`) — different injection timing/shape, may slip past the script-load detector
2. Non-Frida: `dumpsys`, `am broadcast`, content provider queries
3. Repackaged-with-frida-gadget APK (high effort, escalate before doing)

## Forbidden
Do NOT retry hot-reloading any of these into a fresh spawn:
- `nmss_java_probe_2026-05-02.js`
- `nmss_native_cert_single_template_2026-05-02.js`
- `nmss_java_attach_live_2026-05-02.js`
- `nmss_java_attach_minimal_2026-05-02.js`
- `live_sp968_spawn_2026-05-02.js`
