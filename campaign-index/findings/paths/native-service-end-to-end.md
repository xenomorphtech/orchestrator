---
name: Native cert service end-to-end working
description: cycle 730 breakthrough — Frida CLI spawn + top-level loader + Java.scheduleOnMainThread reaches NmssSa.singleton(); native TCP service on tcp:7781 replies STATUS/INIT/GETCERT
type: finding
---

# Native cert service end-to-end working (cycle 730 → 731 full success)

**Confirmed**: 2026-05-02, cert-ptrace agent, baseline checkpoint
`analysis/checkpoints/nmss_live_cert_service_baseline_2026-05-02.json`

## What works

- **Lane**: plain `frida -U -f com.netmarble.thered -l <loader>.js` (Frida CLI, NOT Python API)
- **Loader shape**: top-level `setInterval` polling, NOT `rpc.exports`
- **Java access**: every NmssSa call is wrapped in `Java.scheduleOnMainThread(...)` — touching it off-main-thread poisons class init
- **Native service**: native .so loaded into game; pthread TCP server on `tcp:7781` inside game process
- **Replies on fresh spawn**:
  - `STATUS` → `{"ok":true,"inst":true,"activity":true,"version":"5.17.26001.2601 "}`
  - `INIT` → `{"ok":true,"cr_path":"/data/local/tmp/nmsscr.dec","loadCr":true}`
  - `GETCERT 7BDA93D2F45D36C0` → `{"ok":true,"challenge":"7BDA93D2F45D36C0","public_cert":"","native_cert":""}`

## Full success update

On a later fresh spawned session, the remaining session-state wall was bypassed by sending an **in-process synthetic touch gesture** from the same top-level Frida loader:

- main-thread `Activity.dispatchTouchEvent(MotionEvent)` at screen center
- followed by the same native service `STATUS/INIT/GETCERT` flow

After that touch:
- memory jumped into the earlier "in-game world" range (`~839 MB RSS` on this device build; enough to flip cert state)
- `GETCERT 7BDA93D2F45D36C0` became fully non-empty
- all 5 challenge certs were captured successfully

Saved captures:
- `analysis/checkpoints/native_cert_0000000000000000_clean_session_2026-05-02.json`
- `analysis/checkpoints/native_cert_0123456789ABCDEF_clean_session_2026-05-02.json`
- `analysis/checkpoints/native_cert_1111111111111111_clean_session_2026-05-02.json`
- `analysis/checkpoints/native_cert_7BDA93D2F45D36C0_clean_session_2026-05-02.json`
- `analysis/checkpoints/native_cert_AABBCCDDEEFF0011_clean_session_2026-05-02.json`
- summary: `analysis/checkpoints/native_cert_all5_clean_session_2026-05-02.json`

Ground truth captured from the live session:

| Challenge | Native cert |
|---|---|
| `0000000000000000` | `4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47` |
| `0123456789ABCDEF` | `3083EFBA67F78ADBBD44EA727AF751E1F7380A33CC78B030` |
| `1111111111111111` | `3933F258F2EDF72D3CDD7E31D075D1F34AE73DF72D60A16B` |
| `7BDA93D2F45D36C0` | `90237F0E03DF6993A54669AA7CF27E36304273143AD6A030` |
| `AABBCCDDEEFF0011` | `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C` |

## Follow-up on the first live service-backed session

- Session `18737/18844` stayed alive with the native TCP service up and replying.
- Cheap OS-input progression attempts were tried on that same live session:
  - `adb shell input swipe 960 540 961 540 50`
  - `adb shell input keyevent 66`
  - `adb shell input keyevent 23`
  - `adb shell cmd input motionevent DOWN 960 540; ... UP 960 540`
- These inputs **did change the framebuffer** (screenshots diverged materially from the earlier baseline), but they did **not** make `GETCERT 7BDA93D2F45D36C0` go non-empty.
- Memory stayed in the ~`760 MB RSS` range, far below the earlier ~`1.7 GB` "in-game world" session marker. Current read: the session progressed somewhat, but not into the fully authenticated/in-world cert-valid state.
- Practical implication (confirmed correct): **in-process UI driving** via `Activity.dispatchTouchEvent(MotionEvent)` on the main thread is the right branch; blind `adb input` retries are not.

## Two implementation bugs that mattered

1. **Python Frida API spawn path keeps Java invisible.** Plain `frida -f` does not. This is independent of `--no-pause` — it's about Python's spawn/attach/resume sequencing differing from the CLI's. Use the CLI lane for any spawn-mode capture.
2. **Touching `NmssSa` off-main-thread poisons class init.** Symptom: `NoClassDefFoundError` even after `Java.available=true`. Fix: wrap every `Java.use("nmss.app.NmssSa")` and singleton call in `Java.scheduleOnMainThread(...)`.

## Files

- `nmss-emu/scripts/nmss_live_cert_service.c` + `build_nmss_live_cert_service.sh` + `.so`
- `nmss-emu/analysis/frida/nmss_live_cert_service_loader_2026-05-02.js` (final top-level shape)
- `nmss-emu/analysis/checkpoints/nmss_live_cert_service_baseline_2026-05-02.json`
- `nmss-emu/analysis/checkpoints/nmss_live_cert_service_toplevel_cli_spawn_v6_2026-05-02.log`

## Implication for campaign

- Injection / loader / service startup is **no longer the blocker** — that wall is gone.
- The live ground-truth capture goal is **complete** on the native-service lane.
- The cert RE campaign can now pivot from "how do we get live certs?" to "how do we reproduce/validate the actual live path against this ground truth?".
