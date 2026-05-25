# albion-mobile-signup — turn-3: scale to n=2 and n=3 accounts

## Role & workdir
Codex worker (codex_app_server). Workdir: `/home/sdancer/albion-mobile-signup`. Substrate: Waydroid (local Linux host).

## Already achieved (do NOT re-falsify)

| Level | Artifact | What it verifies | Status |
|---|---|---|---|
| 1 | `apk/albion_mobile.apk` (SHA256 `4a38a1b8c427`) | Albion-Mobile APK acquired (`com.albiononline` v1.31.010.331567) | ✅ DONE |
| 2 | `analysis/albion_postload_waydroid_2026-05-24.png` | Split-install + UI renders on Waydroid | ✅ DONE |
| 3 | `analysis/mobile_signup_live_progress_2026-05-24.md` | Full mobile UI signup path mapped: chooser→email→password→terms→final-modal | ✅ DONE |
| 4 | `secrets/account_1.json` (mode 600, addr-sha256 `c9064a0dd400`) | **n=1 fresh account created via mailbox_2, activation link followed** | ✅ DONE |
| 4a | `analysis/mobile_signup_v2_success_2026-05-24.md` | Full verdict: modal-address matched, mail arrived, activation succeeded | ✅ DONE |
| 4b | Mailbox 2 attempt 5 cascade (`mailbox2_attempt5_2026-05-24_*.png`) | Validated UI recipe with per-stage image-wait gates + slow per-character keyboard taps | ✅ DONE |

## Goal
- **goal_key:** `albion_action_loop`
- **sub_goal_key:** `fresh_accounts_via_mobile_app_signup`
- **Success metric (this turn):** `secrets/account_2.json` AND `secrets/account_3.json` both written mode 600 with `{email, password, mailbox_jwt, confirmed_at_iso}`.

## Validated recipe (from n=1 success — REUSE)

The successful path on attempt5 of mailbox_2:

1. Provision fresh `secrets/mailbox_N.json` via mail.tm.
2. Drive in-app wizard: chooser → email page → password page → terms (Accept All @ ~(808,594)) → post-submit modal.
3. **Email entry**: slow per-character on-screen keyboard taps; NEVER `adb shell input text` (mangles `@`/`.`); NEVER batched taps (corrupted address).
4. **Per-stage gates**: wait on actual screen-state reference images, NOT fixed `sleep`. The `attempt5` reference set is in `analysis/`.
5. **Modal verification**: read modal address from `mailbox_N_attemptM_post_submit_raw.png`; compare sha256 prefix vs mailbox address. Must match.
6. **Mail poll**: 180s window on mail.tm after submit timestamp.
7. **Activation**: open confirmation link under `xvfb-run` browser session, follow through.
8. **Persist**: write `secrets/account_N.json` mode 600 with `{email, password, mailbox_jwt, confirmed_at_iso}`.

## Concrete plan (turn-3, ~60min budget)

1. **n=2 attempt** (15–25min):
   - Provision `secrets/mailbox_3.json` (mailbox_2 already consumed by n=1).
   - Re-run the validated recipe end-to-end.
   - On success: `secrets/account_2.json` written. Commit `analysis/mobile_signup_v3_n2_success_2026-05-24.md` with sanitized sha256 prefixes only.

2. **n=3 attempt** (15–25min) — only after n=2 lands:
   - Provision `secrets/mailbox_4.json`.
   - Same recipe. `secrets/account_3.json` written. Commit `analysis/mobile_signup_v3_n3_success_2026-05-24.md`.

3. **If a per-attempt failure happens** (modal-match fail OR 180s mail-poll fail):
   - Capture the post_submit screenshot + raw modal address sha256.
   - If modal-address-matches AND zero mail twice in a row on different mailboxes → commit `analysis/mobile_signup_v3_n<N>_blocked_deliverability_2026-05-24.md`. STOP.
   - If modal-address-doesn't-match: UI recipe regressed; reread `analysis/mobile_signup_live_progress_2026-05-24.md` and tighten per-character cadence.

4. **DO NOT** touch `secrets/account_1.json`, `secrets/mailbox_1.json`, or `secrets/mailbox_2.json`. Those are the n=1 artifact set.

## Falsification (HARD CONTRACT)

Path falsified IFF: 2 attempts on DIFFERENT fresh mailboxes both show correct modal address AND zero mail within 180s. This would be a deliverability-class block specific to the mail.tm / disposable domain. If hit → commit blocked.md + pivot to alt-provider in turn-4 brief.

## Trajectory regulation

- Heartbeat: write something to `analysis/` every ~10 min if no transition.
- Hard 60min budget; if stuck, commit a partial-progress note and stop cleanly.

## Side-channel abort

Each iteration: `test -f /tmp/abort_albion-mobile-signup`.

## Constraints

- **NEVER touch production PID 688424 or `/home/albion/.config/`** — desktop production session, separate substrate (vast.ai).
- **NEVER echo creds, JWTs, tokens, passwords to stdout/logs/chat.** Sanitized SHA256 prefixes only. Secrets mode 600 gitignored.
- **NEVER overwrite `account_1.json` or `mailbox_2.json`** — those are the n=1 verified artifact set.
- The handoff doc `analysis/mobile_signup_live_progress_2026-05-24.md` and the n=1 verdict `analysis/mobile_signup_v2_success_2026-05-24.md` are authoritative — DO NOT re-derive the UI recipe.

## Relevant files / references

- **n=1 verdict (authoritative recipe)**: `analysis/mobile_signup_v2_success_2026-05-24.md`
- **n=1 cascade** (reference images for per-stage gates): `analysis/mailbox2_attempt5_2026-05-24_*.png`
- **UI recipe doc**: `analysis/mobile_signup_live_progress_2026-05-24.md`
- mail.tm helper: `/home/sdancer/albion-fresh-accounts/scripts/create_account.py`
- Dashboard: `https://albion.orch.run/state` — production zone YEKZMHL must remain `name:"YEKZMHL"`
- Memory: `[[albion_waydroid_works]]`, `[[apkpure_skill_fixes]]`, `[[waydroid_vs_thered_substrate]]`
