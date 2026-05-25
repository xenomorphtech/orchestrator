# albion-tutorial-adv-enum — Adversarial-pair enumeration for tutorial-advance closure

## Role & workdir
Codex worker (adversarial-pair, per orchestrate skill). Workdir: **`/home/sdancer/albion-tutorial-adv-enum`** (branch `tutorial-adv-enum`).

## Mandate (do NOT execute mechanisms)
Path `albion-tutorial-advance` was just marked `mechanism-dropped` after 5 XTest-class probes failed to advance Veldra's "Talk to the survivor" quest. The mechanism class `xdotool XTest input synthesis` (LEFT button + keyboard) is exhausted.

Your job: **enumerate ≥3 untried mechanisms in the same class (input-injection to Albion NPC)**, evaluate each, rank by expected value. DO NOT execute the alternatives — just produce the enumeration document.

## Deliverable
Write `/home/sdancer/albion-tutorial-adv-enum/analysis/tutorial-advance_adversarial_alternatives.md` with the following format per mechanism:

```
## Mechanism N: <name>
- **One-line recipe**: <how to invoke it, with exact command shape>
- **Why it might work where XTest failed**: <hypothesis about why this class differs from XTest>
- **Fastest probe**: <single command + verification check, ≤30s wall time>
- **Cost**: <install/setup cost: minutes or impossible>
- **Risk**: <anti-cheat / instability concerns; reference [[no-frida-libunreal]] if relevant>
- **EV-rank score** (subjective 1–10): _
```

Minimum 5 mechanisms. Rank them by EV at the end. Include a "Recommendations" section naming the top-2 for next-tick spawning.

## Starter candidates (you may add more)
1. **vncdo / vncdotool RFB pointer click** — `pip install vncdotool && vncdo -s localhost:3 click left X Y`. Bypasses XTest, sends raw RFB protocol.
2. **ydotool / `/dev/uinput` direct write** — `ydotool click 0xC0` after `systemctl start ydotoold`. Kernel-side, indistinguishable from real HID.
3. **AT-SPI / dbus accessibility events** — `pyatspi` or `dbus-send` to invoke "click" action on the accessible NPC widget (if Unity exposes AT-SPI tree).
4. **Frida hook on Unity UI raycaster / OnPointerClick** — RISK per [[no-frida-libunreal]] — anti-cheat may detect.
5. **xdotool `--clearmodifiers` + XSendEvent** — alternative X protocol path that may not look like XTest.
6. **evdev raw write to /dev/input/event* nodes** — fully bypasses X, kernel input layer.
7. **Mouse-warp to NPC then xdotool key Return** — focus-then-confirm pattern.
8. **wlrctl** (Wayland but Xtigervnc is X — likely not applicable; verify availability and dismiss if no).
9. **LD_PRELOAD libX11 shim** — intercept Albion's own X calls (anti-cheat risk).
10. **Network-layer Photon packet injection** — bypass input entirely, send the dialog-open packet to the Albion server. RISK: [[albion-send-hooks-break-client]].

## Files / references
- Path closure document (READ THIS FIRST): `/home/sdancer/albion-tutorial-advance/analysis/tutorial_advance_partial_2026-05-25.md`
- Original briefing for context: `/home/sdancer/orchestrator/briefings/albion-tutorial-advance.md`
- /state endpoint: `http://127.0.0.1:8765/state`
- Substrate: Veldra1203 in The Lighthouse, world (7.75, -44.25), NPC visible adjacent

## Hard constraints
- **DO NOT execute any mechanism.** Only enumerate.
- **DO NOT touch any worktree other than your own** (`/home/sdancer/albion-tutorial-adv-enum/`).
- **DO NOT touch the 4 daemons** (watchdog, gamestate-local-{capture,service}, action-loop).
- **30-minute hard cap**: commit deliverable by minute 28 or write `partial.md` with whatever you have.
- **DO NOT need heartbeat file** — this is a research/enumeration task; just write the deliverable.

## Memory references
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure
- `[[adversarial-enumeration-on-blocked-claim]]` — your purpose
- `[[no-frida-libunreal]]` — anti-cheat caveat for any Frida-class mechanism
- `[[albion-send-hooks-break-client]]` — caveat for network-level injection
