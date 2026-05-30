# Spawn-mode RPC agent survives script load (BREAKTHROUGH)

**Confirmed**: cycle 692, 2026-05-02

## What survived
`python3 frida/nmss_live_rpc_5558.py --frida-host 127.0.0.1:27042 --service-only` with the spawn-owned + attach-before-resume design loaded `analysis/frida/nmss_live_rpc_agent_2026-05-02.js` into `com.netmarble.thered` and **both game (pid 5588) and nmcore (pid 5717) survived**:

```jsonl
{"event": "spawn", "pid": 5588, "spawn_wait_seconds": 20.0, "spawn_owned": true}
{"event": "ping", "ok": true, "java_available": false, "pid": 5588, "arch": "arm64"}
{"event": "waitforready", "ok": false, "error": "Java not available"}
```

Post-attempt `pidof`: game=5588, nmcore=5717 — alive.

## Why this matters
This is the **first confirmed Frida-injection that did NOT trigger anti-debug fast-kill since the cycle-666 hardening**. All other approaches (attach mode, hot-reload of probe scripts, even hot-reload of the once-positive `nmss_java_probe.js`) instantly killed the process. The combination that works:

1. **Spawn mode** (`-f com.netmarble.thered`)
2. **Attach-before-resume** (Python host attaches to the spawned-suspended pid before letting it run)
3. **RPC agent** (rpc.exports shape, not one-shot send/recv probe)

## Remaining tuning issue (NOT a wall) — FIXED cycle 692
The `waitforready` RPC returned "Java not available". With attach-before-resume, the agent is injected before the JVM initializes, so `Java.available` is false at agent-load time. The current `waitforready` logic ran out of polling iterations before Java came up.

**Fix applied** (cert-ptrace cycle 692): `analysis/frida/nmss_live_rpc_agent_2026-05-02.js` `waitforready` now polls until Java becomes available instead of erroring immediately. Next spawn should pass through this stage.

## Implication for the campaign
With the script-load wall broken, the path to per-challenge cert capture is now:
1. Tune `waitforready` to actually catch JVM init (cert-ptrace's next task)
2. Run `--init` (calls NmssSa.init) once the title gate clears
3. Run `--getall` to collect 5 challenge → native_cert pairs
4. Save to `analysis/checkpoints/native_cert_<challenge>_clean_session_2026-05-02.json`
5. Set fact `live_native_cert_ground_truth_2026-05-02 = full`

## Sister wall now narrower
`walls/frida-spawn-probe-load.md` should be re-scoped: the wall affects **hot-reload of probe scripts** and **post-startup script load** but does NOT affect **spawn-owned + attach-before-resume + RPC-agent** shape. Update that wall accordingly.
