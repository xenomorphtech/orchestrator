# albion-tutorial-dodge-adv-enum — adversarial-pair enumeration for H10 mechanism-dropped

## URGENT — 30-min HARD CAP, ENUMERATION ONLY (no execution)

Path `albion-tutorial-dodge` was just marked **mechanism-dropped** by worker `albion-tutorial-dodge` (verdict: `/home/sdancer/albion-tutorial-dodge/analysis/tutorial_dodge_blocked_2026-05-25.md`). Citing failure of input-injection mechanisms (xdotool key F, xdotool keydown+keyup with focus, ydotool uinput).

Your job: **enumerate ≥3 untried alternative mechanisms** that could fire/cast the Albion "Dodge" tutorial step quest. Do NOT execute any of them — just enumerate, score, and write the enumeration file.

## Role & workdir

Codex worker. Workdir: **`/home/sdancer/albion-tutorial-dodge-adv-enum`** (branch `tutorial-dodge-adv-enum`).

## Mandate

Read the original worker's verdict at `/home/sdancer/albion-tutorial-dodge/analysis/tutorial_dodge_blocked_2026-05-25.md` first. Then enumerate untried mechanisms across THREE classes:

### Class A: Simple keypress siblings (within input-injection class)
The worker already listed 5 untried siblings: `xdotool key --repeat 2 f`, `xdotool key F1`, `xdotool key Shift+f`, `vncdo -s localhost:3 key f`, `/dev/uinput KEY_F direct injector`. Validate these are real (not already exhausted) and add ≥2 more.

### Class B: Compound mechanism (F-arms-target, then fires)
**CRITICAL HYPOTHESIS** — the briefing said "F at NPC armed a combat ability earlier (per H8 verdict)". The Albion Dodge spell is a **directional ability on boots**, not a passive — pressing F may ARM a directional targeter requiring a second action to commit:
- F then LEFT-click ground (fire dodge toward click point)
- F then RIGHT-click ground (cancel-fire?)
- F then arrow-key (directional dodge with movement keys)
- F then F-tap-again (some games fire on second tap)
- F + immediately move (locomotion-coupled dodge)
- F while character is already moving (right-click ground move, then F mid-motion)

Enumerate ≥3 compound recipes.

### Class C: Higher-substrate mechanism (escape input-injection entirely)
- Frida-hook UnityEngine.Input.GetKeyDown / GetKey to inject directly
- AT-SPI/dbus a11y events
- LD_PRELOAD libX11 shim for XQueryKeymap
- Raw `XSendEvent` (non-XTest, real KeyPress/KeyRelease)
- `evemu-event` synthesized HID via /dev/input/event*
- Game-config remap (change Dodge to a key the worker DID send)
- Skip-tutorial chat command (`/leave`, `/exit-tutorial`, `/skip`)

Enumerate ≥3 from Class C with cost estimates.

## Output format (REQUIRED)

Write `/home/sdancer/albion-tutorial-dodge-adv-enum/analysis/albion-tutorial-dodge_adversarial_alternatives.md` with this structure:

```markdown
# Adversarial enumeration for H10 albion-tutorial-dodge

## Class A — Untried keypress siblings (input-injection class)
| # | Recipe (one line) | Fastest probe to validate | Cost estimate |
|---|---|---|---|
| 1 | `xdotool key --repeat 2 f` | ... | 1min |
... ≥7 rows total (5 worker-named + ≥2 fresh) ...

## Class B — Compound mechanism (F-armed, second action fires)
| # | Recipe (one line) | Fastest probe to validate | Cost estimate |
| 1 | `xdotool key f; sleep 0.2; mousemove 960 600; click 1` (fire dodge toward screen center via L-click) | screenshot + /state diff | 2min |
... ≥3 rows ...

## Class C — Higher-substrate mechanism (escape input-injection)
... ≥3 rows ...

## Verdict
- Net-new mechanisms identified: <count>
- Most-recommended next-path: <recipe>
- Recommendation: spawn new path under name `<albion-tutorial-dodge-{class+letter}>`
```

## Constraints (HARD)

- **NO EXECUTION.** Don't run xdotool, ydotool, screenshot. Just read + write the enumeration file.
- NEVER touch acct3-albion.service / acct3-xtigervnc.service / albion-acct3-watchdog.service / albion-gamestate-local-*.
- 30-min hard cap (your unit will be killed at the cap).
- Output MUST be the enumeration file. No analysis-only narration.

## Memory references
- `[[falsify-mechanism-not-path]]`
- `[[albion-tutorial-step-advance-recipe]]`
- `[[unity-password-clipboard-paste]]` — case study of clipboard-paste reviving a "blocked" path

## Files

- Original verdict: `/home/sdancer/albion-tutorial-dodge/analysis/tutorial_dodge_blocked_2026-05-25.md`
- Worktree: `/home/sdancer/albion-tutorial-dodge-adv-enum/`
- Output: `/home/sdancer/albion-tutorial-dodge-adv-enum/analysis/albion-tutorial-dodge_adversarial_alternatives.md`
