# Wall: adb shell input tap doesn't dismiss Vampir title screen

**Confirmed**: cycle 677, 2026-05-02

## Symptom
```
$ adb -s 127.0.0.1:5558 shell input tap 960 540
$ # screenshot taken after — still shows 請輕觸畫面。 on Vampir title
```

Screen size is 1920×1080, so 960,540 is dead center.

## Why (likely)
Unreal-engine apps frequently filter OS-level synthesized input events and only accept genuine touch events from `/dev/input/eventN`. The `input tap` command goes through the InputManager service, which Unreal can detect and discard.

## What to try instead
- **Physical/manual tap from user** (currently the only known path)
- Possibly: write directly to `/dev/input/event*` via low-level uinput injection (not yet tried, may also be filtered)
- Possibly: a `swipe` gesture instead of tap
- Possibly: tap a specific UI element (the "tap to start" button) rather than screen center — coordinates not yet known

## Forbidden
- Do NOT loop `input tap` retries — confirmed ineffective and just adds noise
