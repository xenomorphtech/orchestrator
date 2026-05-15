#!/usr/bin/env python3
"""
dd_autoenter.py — Drive Dark December through title → world → char → enter-game.

The device is at the title screen ("Press the screen to login"); this script
clicks through to in-game using template matching against saved screen crops,
falling back to hardcoded coordinates when a template is missing.

Workflow:
    1. First time, just run it with `--run`. It will tap center of screen
       (works for the title), wait, screenshot, and try again. While at each
       new screen, in another shell save a template:
            python dd_autoenter.py --capture region_select
            python dd_autoenter.py --capture char_select
            python dd_autoenter.py --capture enter_game
    2. After that, each `--run` invocation matches the template, prints which
       state was matched, taps the configured coordinate, and proceeds.

Templates live in ./templates/<state>.png. You can edit STATES below to tune
coordinates and wait times.

Usage:
    python dd_autoenter.py --run                  # default: full sequence
    python dd_autoenter.py --run --state title    # run only one named state
    python dd_autoenter.py --capture <state>      # save current screen for that state
    python dd_autoenter.py --show                 # save current screen to /tmp/dd_now.png
    python dd_autoenter.py --tap X Y              # one-shot manual tap
    python dd_autoenter.py --probe                # screenshot + match against ALL templates

Requires: cv2 (opencv-python-headless), numpy, adb on PATH.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

import cv2
import numpy as np

ADB_DEVICE = "localhost:5558"
SCRIPT_DIR = Path(__file__).resolve().parent
TEMPLATES_DIR = SCRIPT_DIR / "templates"
TEMPLATES_DIR.mkdir(exist_ok=True)


def adb(*args: str, check: bool = True, capture: bool = True) -> subprocess.CompletedProcess:
    cmd = ["adb", "-s", ADB_DEVICE, *args]
    return subprocess.run(
        cmd,
        capture_output=capture,
        check=check,
    )


def screencap() -> np.ndarray:
    """Return current Waydroid screen as BGR numpy array."""
    p = subprocess.run(
        ["adb", "-s", ADB_DEVICE, "exec-out", "screencap", "-p"],
        capture_output=True,
        check=True,
    )
    arr = np.frombuffer(p.stdout, dtype=np.uint8)
    img = cv2.imdecode(arr, cv2.IMREAD_COLOR)
    if img is None:
        raise RuntimeError("screencap returned no image data")
    return img


def tap(x: int, y: int) -> None:
    print(f"[tap] ({x}, {y})", flush=True)
    adb("shell", "input", "tap", str(x), str(y))


def match_template(scene: np.ndarray, template: np.ndarray, threshold: float = 0.85):
    """Return (score, center_x, center_y) or None if best < threshold."""
    if template.shape[0] > scene.shape[0] or template.shape[1] > scene.shape[1]:
        return None
    result = cv2.matchTemplate(scene, template, cv2.TM_CCOEFF_NORMED)
    _, max_val, _, max_loc = cv2.minMaxLoc(result)
    if max_val < threshold:
        return (max_val, None, None)
    h, w = template.shape[:2]
    return (max_val, max_loc[0] + w // 2, max_loc[1] + h // 2)


@dataclass
class State:
    name: str
    tap_xy: tuple[int, int]
    wait_after: float = 2.0
    threshold: float = 0.85
    # If template missing AND fallback=True, will tap tap_xy without confirmation.
    fallback: bool = False
    description: str = ""

    @property
    def template_path(self) -> Path:
        return TEMPLATES_DIR / f"{self.name}.png"

    def load_template(self) -> np.ndarray | None:
        if not self.template_path.exists():
            return None
        return cv2.imread(str(self.template_path))


# ====== Edit these to tune the sequence ======
# (1920x1080 landscape)
STATES: list[State] = [
    State(
        name="title",
        tap_xy=(960, 540),
        wait_after=3.0,
        fallback=True,
        description='Title screen with "Press the screen to login". Tap center.',
    ),
    State(
        name="region_select",
        tap_xy=(960, 760),
        wait_after=3.0,
        fallback=False,
        description='Region/server screen. Tap "Last Region" or "EUROPE1" button.',
    ),
    State(
        name="char_select",
        tap_xy=(960, 540),
        wait_after=2.0,
        fallback=False,
        description="Character select screen. Tap the character slot.",
    ),
    State(
        name="enter_game",
        tap_xy=(1700, 1000),
        wait_after=8.0,
        fallback=False,
        description='"Enter Game" / "Start" button (typically bottom-right).',
    ),
]


def find_state(scene: np.ndarray) -> State | None:
    """Return the first state whose template matches scene, else None."""
    for s in STATES:
        tmpl = s.load_template()
        if tmpl is None:
            continue
        r = match_template(scene, tmpl, threshold=s.threshold)
        if r is None:
            continue
        score, cx, cy = r
        if cx is None:
            print(f"  ? {s.name}: best score {score:.3f} (below {s.threshold})")
            continue
        print(f"  ✓ matched {s.name} (score {score:.3f} at ({cx},{cy}))")
        return s
    return None


def run_sequence(max_iterations: int = 20, between_states: float = 1.0) -> int:
    """
    Walk through STATES, tapping each in order. After each tap, sleep wait_after,
    re-screencap, and decide whether to advance.

    Strategy: at each iteration, screencap, try to match against ALL state
    templates. If matched, tap that state's coord. If none match, advance to
    next state in list (if fallback=True) or stop.
    """
    idx = 0
    for it in range(max_iterations):
        print(f"\n=== iter {it+1}/{max_iterations} (cursor=state[{idx}]) ===")
        scene = screencap()
        matched = find_state(scene)
        if matched is not None:
            tap(*matched.tap_xy)
            time.sleep(matched.wait_after)
            # try to advance idx past this state
            try:
                idx = max(idx, STATES.index(matched) + 1)
            except ValueError:
                pass
            continue

        # No template matched. Use the cursor state if fallback allowed.
        if idx >= len(STATES):
            print("  -> done (all states past cursor exhausted)")
            return 0
        s = STATES[idx]
        if s.fallback:
            print(f"  no template match; FALLBACK tapping {s.name} at {s.tap_xy}")
            tap(*s.tap_xy)
            time.sleep(s.wait_after)
            idx += 1
            continue

        print(f"  no template match for state[{idx}]={s.name} and fallback=False")
        print(f"  -> stopping. capture this screen as a template:")
        print(f"     python {sys.argv[0]} --capture {s.name}")
        return 1
    print("\nmax_iterations hit")
    return 2


def cmd_run(args):
    if args.state:
        # run just one state
        s = next((x for x in STATES if x.name == args.state), None)
        if s is None:
            print(f"Unknown state: {args.state}. Known: {[x.name for x in STATES]}")
            return 1
        scene = screencap()
        m = find_state(scene)
        if m is None or m.name != s.name:
            if not s.fallback:
                print(f"State {s.name} not matched on current scene and fallback=False")
                return 1
        tap(*s.tap_xy)
        time.sleep(s.wait_after)
        return 0
    return run_sequence(max_iterations=args.max_iterations)


def cmd_capture(args):
    state_name = args.capture
    if state_name not in [s.name for s in STATES]:
        print(f"Warning: '{state_name}' is not in STATES. Saving anyway.")
    scene = screencap()
    # Save the full screen as the template. (You can crop it later to a unique
    # region with a separate command, but full-screen matching tends to work
    # well at high resolutions with a strong threshold.)
    if args.crop:
        x0, y0, x1, y1 = [int(v) for v in args.crop.split(",")]
        crop = scene[y0:y1, x0:x1]
    else:
        crop = scene
    path = TEMPLATES_DIR / f"{state_name}.png"
    cv2.imwrite(str(path), crop)
    print(f"saved {path} ({crop.shape[1]}x{crop.shape[0]})")
    return 0


def cmd_show(args):
    scene = screencap()
    out = args.output or "/tmp/dd_now.png"
    cv2.imwrite(out, scene)
    print(f"saved screenshot to {out}  dims={scene.shape[1]}x{scene.shape[0]}")
    return 0


def cmd_tap(args):
    tap(args.tap_x, args.tap_y)
    return 0


def cmd_probe(args):
    scene = screencap()
    print(f"scene: {scene.shape[1]}x{scene.shape[0]}")
    for s in STATES:
        tmpl = s.load_template()
        if tmpl is None:
            print(f"  {s.name}: <no template>")
            continue
        r = match_template(scene, tmpl, threshold=0.0)
        if r is None:
            print(f"  {s.name}: template too large for scene")
            continue
        score, cx, cy = r
        marker = "✓" if score >= s.threshold else " "
        print(f"  {marker} {s.name}: score={score:.3f} at ({cx},{cy}) threshold={s.threshold}")
    return 0


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--run", action="store_true", help="run the full state sequence")
    p.add_argument("--state", help="only run this single named state (with --run)")
    p.add_argument("--capture", metavar="NAME", help="save current screen as templates/NAME.png")
    p.add_argument("--crop", help="for --capture, crop region 'x0,y0,x1,y1' before saving")
    p.add_argument("--show", action="store_true", help="save current screen and exit")
    p.add_argument("--output", help="for --show, write to this path instead of /tmp/dd_now.png")
    p.add_argument("--tap", nargs=2, type=int, metavar=("X", "Y"), help="one-shot tap")
    p.add_argument("--probe", action="store_true", help="screencap + score against every template")
    p.add_argument("--max-iterations", type=int, default=20, help="upper bound for --run loop")
    a = p.parse_args()

    if a.tap:
        a.tap_x, a.tap_y = a.tap
        return cmd_tap(a)
    if a.capture:
        return cmd_capture(a)
    if a.show:
        return cmd_show(a)
    if a.probe:
        return cmd_probe(a)
    if a.run:
        return cmd_run(a)
    p.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
