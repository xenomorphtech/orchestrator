# Lateral: Frida Java bridge invisible on spawn-owned attach-before-resume

**Wall (new, cycle 717)**: On `com.netmarble.thered` spawn-owned + attach-before-resume, **`Java.available` never becomes true** across 111 polls / 90 seconds. Process stays alive (11446/11547), but Frida's Java bridge never observes the JVM. Both the RPC agent and native-service loader currently depend on `Java.perform` for the initial NmssSa/Activity handoff.

**Evidence**: `analysis/checkpoints/nmss_live_cert_service_spawn_attempt_v2_2026-05-02.jsonl` — 111 probejava samples all show `java_available: false`.

## Experiments (cheap → heavy)

1. `[x]` **`frida -U -f --no-pause`** — **CONFIRMED WORKING cycle 719.** `Java.available` becomes true at **501ms** under plain `frida -f -l <script>.js -q -t 35`. `class_name`, `singleton_exists`, `activity_exists` all true throughout. The actual root cause was NOT attach-before-resume — it was that the host's `rpc.exports.startservice` did a synchronous spin-wait that starved Frida's JS event loop, which is what updates the Java bridge. See `findings/paths/event-loop-vs-sync-rpc.md`. Architectural fix: rewrite waitforready as event-loop polling (setInterval+Promise), not sync spin in an RPC handler.

2. `[ ]` **Hook `android_dlopen_ext` for libart.so** — `Interceptor.attach(Module.findExportByName(null, 'android_dlopen_ext'), ...)` triggers when libart loads. Inside the hook, *then* call `Java.perform(...)`. This sidesteps the `Java.available` polling — we KNOW the JVM is up because we just saw it load. **Cost: small JS rewrite, 5 min.**

3. `[ ]` **Native dlopen + ART runtime singleton** — completely bypass `Java.perform`. From the Frida agent's native side: `dlopen("libart.so", RTLD_NOW)`, find `art::Runtime::Current()` exported symbol or scan ART's static globals to recover the `JavaVM*`. Then `JavaVM->AttachCurrentThread(&env, NULL)` to get a `JNIEnv*`. With env in hand, look up `nmss/app/NmssSa` directly via `FindClass`. **Cost: ~1 hour native code; surgical and Frida-free after handoff.** This is essentially what `nmss_live_cert_service.so` was supposed to do, but invoked via `dlopen` directly (not via Java's `System.load`).

4. `[ ]` **Direct call to `nmssNativeGetCertValue` at offset 0x1400ec** — we have the JNI symbol address from cycle 651. With `JNIEnv*` from experiment 3, just CallNonvirtualObjectMethod (or call the function pointer directly, with manual jstring/jobject construction). Skips `NmssSa.singleton()` entirely. Risky — the function may rely on java-side state set up by `init()` and other prelude calls, but worth knowing whether it returns a string at all.

5. `[ ]` **Wait-then-attach via two-stage Frida** — first `frida -f --no-pause` with a tiny script that just polls `Java.available` and `console.log()`s the moment it becomes true, then on a separate run use the timing observed to attach at the right moment. Diagnostic only; tells us when ART is observable in this build.

6. `[ ]` **Skip Frida entirely — use frida-gadget-injected APK** — repackage the APK with frida-gadget pre-loaded as one of the linked .so dependencies. The gadget runs from app load with full Java visibility. **Cost: half-day, requires APK signing; durable solution if everything else fails.**

## Recommended next: **experiment 1** (`--no-pause`) — 30 seconds to test. If Java.available becomes true within seconds of the app booting normally, the architecture is right — we just had attach-before-resume in the wrong place. If it still never becomes true, jump to experiment 2 (libart hook) which gives us the precise moment to act.
