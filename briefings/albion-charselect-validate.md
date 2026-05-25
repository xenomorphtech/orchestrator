# albion-charselect-validate — turn-12: relogin acct_3 to in-zone state

## Role & workdir
Codex worker. Workdir: `/home/sdancer/albion-charselect-validate`. Substrate: **local `sdancer` host only**, Xtigervnc :3.

## Already achieved (do NOT re-falsify)

| Level | Artifact | What it verifies | Substrate | Status |
|---|---|---|---|---|
| L1 acct_3 created | fact `albion_fresh_account_3_created` | mobile signup | mobile | ✅ DONE |
| L2 acct_3 charselect | prior verdict | reached CREATE CHARACTER | local :3 | ✅ DONE |
| L3 acct_3 inworld earlier today | `acct3_t10_inworld.png` sha256 `29163675ad79`, verdict `acct3_charcreate_verdict_2026-05-25.md` | Veldra1203 The Lighthouse, mechanism = clipboard paste + xdotool active-window | local Xtigervnc :3 | ✅ DONE (~02:00) |
| L4 channel proven for actions | fact `albion_action_emitter_local_dispatch_verified_2026-05-25` + verdict `/home/sdancer/albion-action-emitter-local/analysis/ae_local_verdict_2026-05-25.md` | R key dispatch produced visible activation ring on Veldra1203 | local Xtigervnc :3 | ✅ DONE (02:23) |

## Current substrate state (PRE-VERIFIED — do NOT re-test)

- `acct3-xtigervnc.service` and `acct3-albion.service` both ACTIVE
- Albion window on Xtigervnc :3 is currently showing the **login screen with Server Selection modal** (NOT in-zone anymore) per blocker artifact `/home/sdancer/albion-action-emitter-local/analysis/t5_blocker_login_modal.png` at ~03:15Z
- Veldra1203 character ALREADY EXISTS server-side (not a fresh signup); this is just a re-login
- The action-emitter-local worker COMPLETED click+key palette implementation (fact `albion_action_emitter_local_click_palette_impl_complete`) but cannot live-verify until acct_3 is back in-zone

## Goal (turn-12) — drive acct_3 from login modal back to in-zone

- **goal_key:** `albion_action_loop`
- **success_fact_key:** `albion_acct_3_back_in_zone_2026-05-25_T2`
- **success metric:** screenshot of Albion window on :3 showing in-game HUD (zone name visible, NOT login UI), sha256 prefix logged in verdict

## Hypothesis
The same clipboard+active-window mechanism that worked at ~02:00 (when acct_3 first reached in-zone via the acct_1 fallback pattern) will work again for relogin. Server-side state persists — character Veldra1203 still exists; we just need to dismiss server-select, type creds (via clipboard), handle 2FA if it fires, click Enter World.

## Mechanisms proven on this substrate
- **xdotool active-window** for button clicks works on Xtigervnc :3 (per `[[albion-login-substrate]]`)
- **xclip + xdotool ctrl+v** for password field works on Unity TMP_InputField (per `[[unity-password-clipboard-paste]]`)
- **vncdotool RFB** for pointer clicks on :3
- **mail.tm-style poll** for 2FA codes (existing infra at `secrets/mailbox_4_2fa_poll.json`)

## Falsification (mechanism-scoped — read [[falsify-mechanism-not-path]])
- **Mechanism**: xdotool active-window button clicks + xclip clipboard paste against Albion login surface on Xtigervnc :3.
- Falsified iff: 5 consecutive paste attempts produce no character entry in the password field AND no visual change after Enter key.
- Untried siblings if mechanism fails: vncdotool RFB type-replay, evdev /dev/uinput, AT-SPI accessibility events, USB HID gadget. List ≥3 before any closure.

## Next 2-3 concrete tasks (~90min total)

### Task 1 (~5min) — orient
- `ss -tlnp 2>/dev/null | grep -E ':590[0-9]'` to confirm :3 VNC port (likely 5903)
- `DISPLAY=:3 XAUTHORITY=/home/sdancer/.Xauthority import -window root /home/sdancer/albion-charselect-validate/analysis/acct3_t12_orient.png` → confirm visible state
- `ls /home/sdancer/albion-charselect-validate/secrets/account_3_postlogin/` to see what creds/state are available

### Task 2 (~60min) — drive the login flow
Acct_3 creds are in `secrets/account_3_postlogin/` (read the file structure first; DO NOT log values to chat/jsonl).
1. Dismiss server-selection modal (likely Click "Continue" or pick the server then Continue) — use xdotool active-window click on the visible button
2. If login form appears: focus email field (real-press mousedown-hold-mouseup at field center), xclip clipboard paste email, Tab to password, xclip clipboard paste password, click Sign In
3. If 2FA prompt appears: poll the inbox at `secrets/mailbox_4_2fa_poll.json` (use existing `/home/sdancer/albion-prod-login/scripts/inbox_poll.py` or write a small poller); extract 6-digit code; type into 2FA field with xdotool (codes are public-OK to type since they're single-use)
4. After Sign In success: character-select screen → click Veldra1203 → Enter World
5. Verify in-zone via screenshot: `DISPLAY=:3 import -window root analysis/acct3_t12_back_inzone.png`; HUD should show zone name and minimap, NO login UI

### Task 3 (~10min) — verdict + fact + handoff
- Commit verdict `analysis/acct3_t12_relogin_verdict_2026-05-25.md` with sha256 prefix of the in-zone screenshot
- Set fact: `/home/sdancer/orchestrator/harness fact-set albion_acct_3_back_in_zone_2026-05-25_T2 "Acct_3 Veldra1203 back in-zone on Xtigervnc :3; mechanism: <which mechanism worked>; artifact: <path + sha256>"`
- This handoff fact triggers the action-emitter-local worker to resume live click+key dispatch verification

## Commit-or-falsify contract
- 90min hard cap. If not back in-zone by then, commit `analysis/acct3_t12_relogin_blocked.md` with mechanism-scoped block + ≥3 untried siblings.
- 15min heartbeat: append a one-liner to `analysis/heartbeat.log`
- `/tmp/abort_albion-charselect-validate` → commit partial verdict + exit

## Constraints & gotchas
- **NEVER log/print/jsonl creds.** Read directly from `secrets/account_3_postlogin/`. If pasting via xclip, clear clipboard with `xclip -selection clipboard -i /dev/null` immediately after.
- **NEVER restart acct3-albion or acct3-xtigervnc services** — they hold the substrate. If Albion died, that's terminal; commit blocker.
- **NEVER touch other VNC/Albion sessions** (:4 acct_1, :5 acct_2 — those are P2 territory, dormant).
- **NEVER spam Escape** (per `[[xdotool_unity_albion_blocked]]` retracted — Escape toggles quit-dialog).
- 2FA toll is normal for this account (memory `[[albion-2fa-container-rotation]]`); polling infra exists.

## Files / endpoints
- This briefing: `/home/sdancer/orchestrator/briefings/albion-charselect-validate.md`
- Creds: `/home/sdancer/albion-charselect-validate/secrets/account_3_postlogin/`
- 2FA poll config: `/home/sdancer/albion-charselect-validate/secrets/mailbox_4_2fa_poll.json`
- Reference 2FA poller: `/home/sdancer/albion-prod-login/scripts/inbox_poll.py`
- Reference acct_1 successful login mechanism: `/home/sdancer/albion-charselect-validate/analysis/acct1_t6_post_login.png`
- Harness binary: `/home/sdancer/orchestrator/harness`

## Memory references
- `[[unity-password-clipboard-paste]]` — clipboard mechanism proven for Unity TMP_InputField password fields
- `[[unity-real-press-required]]` — Unity requires mousedown-hold-mouseup, not synthetic click
- `[[albion-login-substrate]]` — Xtigervnc :3 accepts XTEST keysyms for buttons
- `[[albion-2fa-container-rotation]]` — Albion's 2FA prompt is device-trust based, fires on substrate changes
- `[[falsify-mechanism-not-path]]` — mechanism-scoped closure, ≥3 untried siblings before path close
- `[[macromanage-workers]]` — you handle lookups; orchestrator names the goal + verification artifact
