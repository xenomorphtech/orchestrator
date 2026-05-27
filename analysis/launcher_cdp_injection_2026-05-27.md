# Albion launcher CDP injection probe
Date: 2026-05-27
Branch: `albion-launcher-cdp-injection`
Outcome: `not-electron`
Success fact key: `albion_launcher_cdp_injection_outcome_2026_05_27`

## Scope
This closure only executed the first gate from the briefing: determine whether
the Albion Linux launcher is Electron before attempting `--remote-debugging-port=9222`
and CDP DOM injection.

The probe stopped at step 1 because the launcher bundle and live process tree
resolved to Qt WebEngine, not Electron.

## Prior signal being checked
The c443 closure reported Chromium-style stderr:
- `network_service_instance_impl.cc(286) ERROR`

That signal was real but not Electron-specific. Qt WebEngine embeds Chromium
networking and can emit the same message family, so this pass separated
"Chromium internals present" from "Electron app with CDP surface".

## Path correction
The briefing suggested `/home/albion/albion-launcher/Albion-Online`, but the
live host uses:
- `/home/albion/albion-launcher/data/launcher/Albion-Online`
- `/home/albion/albion-online/Albion-Online`

The active launcher process came from:
- `/home/albion/albion-launcher/data/launcher/Albion-Online --no-sandbox -loglevel 0`

## Evidence
Remote host: `root@ssh6.vast.ai -p 29576`

Directory markers under `/home/albion/albion-launcher/data/launcher`:
- `QtWebEngineProcess`
- `libQt5WebEngine.so.5`
- `libQt5WebEngineCore.so.5`
- `libQt5WebEngineWidgets.so.5`
- `libQt5Qml.so.5`
- `resources/`
- `qt.conf`

Dynamic linkage from `ldd /home/albion/albion-launcher/data/launcher/Albion-Online`:
- `libQt5WebEngine.so.5`
- `libQt5WebEngineWidgets.so.5`
- `libQt5WebEngineCore.so.5`
- `libQt5Qml.so.5`
- `libQt5Core.so.5`
- `libnss3.so`
- `libnssutil3.so`
- `libnspr4.so`

Live process inspection showed:
- `QtWebEngineProcess --type=utility --utility-sub-type=network.mojom.NetworkService ...`

Negative evidence:
- no `app.asar`
- no `resources/app`
- no `chrome-sandbox`
- no Electron-named helper processes

## Classification
`not-electron`

Reasoning:
- Qt WebEngine explains the Chromium stderr from c443.
- Qt WebEngine is Chromium-backed, but it is not Electron.
- The assigned hypothesis specifically required an Electron app that accepts
  `--remote-debugging-port=9222` and exposes a CDP page target.
- With the substrate falsified, forcing a CDP attempt would be mechanism drift.

## Disambiguator screenshots
Not captured:
- `pre_inject.png`
- `post_inject_pre_submit.png`
- `post_180s.png`

Reason:
- the stop condition triggered at the T+4 substrate decision point
- no CDP injection step was executed
- no submit step was executed

## Untried sibling mechanisms
1. Frida or ptrace-based in-process hooks targeting the launcher's Qt input or
   QML/JS bridge layer directly.
2. `LD_PRELOAD` shims around Qt text-entry APIs or WebEngine bridge calls.
3. Accessibility-tree injection through AT-SPI rather than XTest, clipboard, or
   RFB key events.
4. Direct launcher network/auth API replay if the UI keeps filtering all
   input-event-class writes.

## Closure
The launcher is Chromium-backed, but the embedding substrate is Qt WebEngine,
not Electron. This falsifies the CDP-via-Electron hypothesis for this turn.
