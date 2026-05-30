# `/data/local/tmp/xerda-server` (52 MB)

Modified Frida server. Behavior matches stock Frida-server CLI verbatim (`xerda-help.txt` on device).

## Launch
```
adb shell /data/local/tmp/xerda-server -D
```
Listens on TCP `127.0.0.1:27042` by default. Verify with `pidof xerda-server`.

## Use
Acts as the device-side endpoint for `frida -U -f ...` calls from the host. Required when frida-server isn't installed (most production devices).

## Current usefulness
- ✅ Useful for `frida -U -f com.netmarble.thered -l <no-op-script>` clean spawns (the only non-killing Frida path)
- ❌ NOT useful for hot-reloading non-trivial probe scripts — anti-debug fast-kills (see `walls/frida-spawn-probe-load.md`)
- ❌ NOT useful for `frida -p <pid>` attach — anti-debug fast-kills (see `walls/frida-attach.md`)

## Currently running
pid 10640 (cycle 687), with helper `re.xerda.helper` pid 10646. Started during cycle 631-ish work; persists across game restarts.
