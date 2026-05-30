# `/data/local/tmp/nmss_probe.sh`

Stock probe script that sends 16 different payload shapes to the cert UNIX socket and saves replies.

## What it does
- Targets `/data/user/0/com.netmarble.thered/files/nmss` (LISTEN, owned by nmcore)
- Tries 16 payload shapes:
  - 1–6: short payloads (8-byte challenge with various length prefixes; immediate zero-byte response observed)
  - 7–16: longer payloads (length=16449 / `0x4041` etc; timeout)
- Logs to `/data/local/tmp/nmss_probes/log.txt`

## Result on pre-tap session (cycle 650)
**16/16 empty bytes returned.** See `walls/nmss-probe-empty.md`.

## When this might become useful
After the title-screen tap clears AND any in-game login completes, retry — nmcore may then respond to one of the 16 shapes (or to a shape we haven't tried). Currently unknown.
