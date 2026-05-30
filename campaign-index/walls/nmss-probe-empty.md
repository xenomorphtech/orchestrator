# Wall: nmss_probe.sh against /files/nmss returns empty for all 16 payloads

**Confirmed**: cycle 650, 2026-05-02

## Symptom
```
$ adb shell /data/local/tmp/nmss_probe.sh
# results saved to /data/local/tmp/nmss_probes/log.txt
# all 16 payload shapes:
#   payloads 1..6: immediate zero-byte response
#   payloads 7..16: timeout, zero-byte response
```

## Why
The Unix socket `/data/user/0/com.netmarble.thered/files/nmss` (owned by nmcore) is in LISTEN state, but nmcore does not respond to any of the 16 stock payload shapes when the game has not advanced past the title screen / not been authenticated.

Possible causes:
1. nmcore is gated on game-side state (auth, title-tap dismissal, login completion)
2. Payload shapes are wrong format/protocol; correct shape is unknown
3. Specific session context (cookie, key) needs to be present

## What to try instead
- Re-test **after** the title-screen tap is cleared and any in-game login completes
- If still empty post-login: the socket protocol needs to be RE'd from nmcore's binary

## Forbidden
- Do NOT retry the 16-payload sweep on a pre-title-tap session — confirmed unproductive
