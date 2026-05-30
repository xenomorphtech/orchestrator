# Wall: frida -p <pid> attach → instant kill

**Confirmed**: many cycles, baseline known since cycle 583

## Symptom
```
$ frida -p 31790
Process terminated
```
Both `com.netmarble.thered` and `nmcore` then absent from `pidof`.

## Why
NMSS anti-debug actively monitors for ptrace attach. Frida's CLI client uses ptrace under the hood to attach to a running pid. Detection is immediate — the process self-terminates.

**Cycle 690 update**: confirmed Frida-attach kills the process **even after** the title-tap gate has been dismissed and the game is fully in the in-game world. The post-title state does NOT relax anti-debug. Attaching the new spawn-owned-design RPC agent (`nmss_live_rpc_agent_2026-05-02.js`) to a running pid 31790 in attach mode still killed both game + nmcore on first RPC call (Java guard bug in agent surfaced as `ReferenceError: 'Java' is not defined`, but the kill itself was anti-debug, not the bug). Bug fixed afterward in the agent.

## What to try instead
- Use **spawn mode** instead of attach: `frida -U -f com.netmarble.thered ...` — the agent is injected before anti-debug initializes (but see `frida-spawn-probe-load.md` for the script-load wall on probe scripts)
- For the new RPC agent specifically: drive it through `nmss_live_rpc_5558.py` in **spawn mode + attach-before-resume**, NOT attach-to-existing — see `open/rpc-agent-build.md`
- Avoid Frida entirely; use non-injection probes

## Forbidden
- Do NOT `frida -p <any-pid-of-com.netmarble.thered-or-nmcore>` ever
- Do NOT `gdb attach <pid>` either — same anti-debug
