# Frida Java bridge: event-loop visibility (not invisibility)

**Confirmed**: cycle 719, 2026-05-02
**Evidence**: `analysis/checkpoints/frida_java_bridge_no_pause_success_2026-05-02.json`, `analysis/checkpoints/java_bridge_visibility_diag_spawn_2026-05-02.log`

## Finding
On `com.netmarble.thered` plain Frida spawn (`frida -H 127.0.0.1:27042 -f com.netmarble.thered -l <script>.js -q -t 35`):
- **`Java.available` becomes true at 501ms** after spawn
- `Java.use("nmss.app.NmssSa")` works immediately after that
- `singleton_exists` and `activity_exists` are true throughout
- 41 probes over 30s, all visible

## What this means
The previous wall (`walls/frida-java-bridge-invisible.md` cycle 717) was **misdiagnosed**. The real problem with the host driver wasn't "attach-before-resume freezes pre-ART-init" — it was that the `rpc.exports.startservice` function ran a **synchronous spin-wait** for Java/Activity readiness. That synchronous loop **starved Frida's JavaScript event loop**, which is what updates the Java bridge state.

Java becomes visible to Frida via the event loop. A sync RPC handler that blocks for 90 seconds blocks the event loop for 90 seconds, so the bridge never sees ART register the JVM.

## Architectural fix
Use async patterns:
- `setInterval()` / `setTimeout()` for polling, not `while` loops with `Thread.sleep`
- Resolve a Promise from an event handler, not from a sync poll
- Let the host wait via separate RPC calls, not via a single blocking `startservice`

The split-poll diagnostic that just succeeded works exactly this way: `setInterval` triggers `Java.perform(()=>{...})` which yields back to the event loop between calls.

## Implication for the campaign
The native-service loader and RPC agent stacks need their `waitforready` rewritten as **event-loop polling that yields**, not as a sync spin in an RPC export. Then both should bootstrap successfully into a working state.
