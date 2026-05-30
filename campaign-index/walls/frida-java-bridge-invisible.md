# Wall: Frida Java bridge invisible on spawn-owned attach-before-resume

**Confirmed**: cycle 717, 2026-05-02
**MISDIAGNOSED — superseded cycle 719**: Real cause was synchronous RPC starving Frida's JS event loop, NOT attach-before-resume freezing pre-ART-init. See `findings/paths/event-loop-vs-sync-rpc.md`.

## Symptom
On `com.netmarble.thered` spawn-owned Frida session with attach-before-resume:
- `Java.available` returns `false` across **111 polls in a 90-second window** (every 500ms)
- `Java.perform` callback never fires
- `class_name`, `singleton_exists`, `activity_exists`, `inst_handle`, `activity_handle` all remain null/false throughout
- Process and nmcore stay ALIVE — this is NOT a probe-load kill

**Evidence**: `analysis/checkpoints/nmss_live_cert_service_spawn_attempt_v2_2026-05-02.jsonl` — 111 probejava events.

## Why (hypothesis)
Attach-before-resume freezes the process before ART (Java VM) initializes for that pid. Frida's Java bridge requires libart to be loaded and the JVM observable. When the agent loads pre-ART, the bridge never picks up the JVM even after resume.

## Implication
Both the spawn-mode RPC agent (cycle 692) and the native-service loader (cycle 710) depend on `Java.perform` for the initial NmssSa/Activity handoff. Both stall at the same readiness check.

## What to try instead
See `lateral/frida-java-bridge-invisible.md` for 6 bypass experiments. Most promising untried lateral: `frida -U -f --no-pause` (let the app boot fully, agent observes ART when it loads naturally).

## Forbidden
- Don't increase the polling timeout further. 90s × 500ms = 111 samples is enough; the bridge isn't appearing.
- Don't try the same loader pattern with longer waits. The architecture is wrong, not the timing.
