# `/data/local/tmp/cert_client` (size 7232 B)

**Status: dead end** — see `walls/cert-client-listener.md`.

## Reverse-engineered semantics
- argv[1] = challenge hex string (default `DEADBEEF12345678`)
- Connects to abstract Unix socket `\0cert_hook` (AF_UNIX)
- snprintfs `"%s\n"` and writes the challenge
- read()s + printf()s the reply

Format strings extracted from binary:
- 0x728: `'%s\n'`
- 0x72f: `'DEADBEEF12345678'`
- 0x740: `'socket'`
- 0x751: `'connect'`
- 0x759: `'ERROR: no response'`

## Why it doesn't work standalone
The `\0cert_hook` listener is **NOT** registered by NMSS at runtime. It must be created by injecting the companion `/data/local/tmp/cert_hook.so` into the game process. Verified: `cat /proc/net/unix | grep cert_hook` returns nothing while game + nmcore are alive.

cert_client + cert_hook.so form a **probe pair**, not autonomous tools.

## Symbol surface (from `strings -a`)
socket, connect, write, read, perror, _DYNAMIC, snprintf, ERROR: no response (etc.)
