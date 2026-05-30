# Wall: cert_client `\0cert_hook` socket has no listener

**Confirmed**: cycle 649, 2026-05-02

## Symptom
```
$ adb shell '/data/local/tmp/cert_client 7BDA93D2F45D36C0'
connect: Connection refused
```
All 5 challenges return identically.

## Why
`/data/local/tmp/cert_client` is a probe binary that connects to the abstract Unix socket `\0cert_hook` and writes `"%s\n" + challenge`. **The listener `\0cert_hook` is NOT registered by NMSS at runtime** — it must be created by injecting `/data/local/tmp/cert_hook.so` into the game process.

cert_client and cert_hook.so are a **probe pair**, not autonomous tools. Without injecting cert_hook.so first, cert_client always fails.

Verified: `cat /proc/net/unix | grep cert_hook` returned nothing while game + nmcore were alive.

## Reverse-engineered cert_client semantics (preserved)
- argv[1] = challenge hex string (default `DEADBEEF12345678` if absent)
- connects to abstract `\0cert_hook` AF_UNIX
- snprintfs `"%s\n"` and writes
- reads + printfs reply

## What to try instead
- Injecting `cert_hook.so` requires Frida (or LD_PRELOAD via repackaged APK) — same wall as `frida-spawn-probe-load.md` then applies
- Skip this lane entirely; use the RPC agent path
