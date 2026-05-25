# albion-acct3-watchdog — URGENT v3 regression recovery

## URGENT — substrate currently AT LOGIN, watchdog NOT triggering relogin

**Live observation (08:59 CEST, /tmp/v_1779692406.png):** acct_3 Albion is at LOGIN SCREEN with Server Selection modal visible. But the v3 detector's latest heartbeat at 06:59:42Z says `state=in_zone`, `in_zone_min=6`, `login=34`. Translation: the new detector v3 multi-crop voting is falsely matching the login screen's left-side character art against one of the in_zone refs.

The watchdog daemon is therefore NOT triggering the relogin script. The action-loop daemon was *also* dispatching xdotool against the login screen (now stopped by orchestrator).

## Already achieved (do NOT re-falsify, but DO investigate the regression in #1)

| Level | Artifact | Status |
|---|---|---|
| L1–L5 (MVP) | `analysis/watchdog_verdict_2026-05-25.md` | ✅ DONE |
| L6 (v2) | `analysis/watchdog_detector_v2_verdict_2026-05-25.md` | ✅ DONE |
| L7 (v3) | `analysis/watchdog_detector_v3_verdict_2026-05-25.md` | ⚠️ REGRESSED — login-screen false-match observed |

## This turn — recover substrate + harden v3 (~30min)

### Task 1 (~5min) — Immediate substrate recovery
1. Take a fresh `:3` screenshot and confirm login state visually.
2. Run `bin/relogin_acct3.py --once` directly to drive the login flow. If a Server-Selection modal blocks the email field, the script should already dismiss it (it did during the original L3 success). If not, click the modal close button via xdotool first.
3. After relogin: confirm `bin/detect_state.py --details` on a fresh screenshot returns `state=in_zone` AND the substrate is actually in-zone (visual check).

### Task 2 (~20min) — Diagnose + fix v3 in_zone false-match
1. Diff the login screenshot's per-ref distances vs an actual in_zone screenshot. Specifically: which of `in_zone_v2_{1,2,3,4}` and `zone_charselect_{1..4}` produced the dist=6 false match? That ref is the culprit (probably matches the left-character art that's shared with login splash).
2. Three mitigations (pick one based on data):
   - **Tighten threshold**: lower the in_zone match threshold from 12 to ~5 (i.e. require very tight match, not just "close-ish"). Simplest but may regress on the 35 historical unknown frames the v2→v3 was designed to fix.
   - **Re-crop**: shrink the detection crop region to exclude the left character-art band. Robust but requires recapturing refs.
   - **Veto-by-login**: require `state=in_zone` AND `login_dist > some_floor` (e.g. >25). Cheapest, narrowest fix. Most likely correct.
3. Smoke against: (a) the login screenshot `/tmp/v_1779692406.png` (must classify as `login`), (b) the 35 historical unknown frames (must still classify as `in_zone`), (c) refs/in_zone_v2_*.png (sanity).
4. Restart watchdog daemon. Verify new heartbeat schema correctly tags the live screen.

### Task 3 (~5min) — Verdict + fact

1. Commit `analysis/watchdog_v3_regression_recovery_2026-05-25.md`: what was misclassified, which mitigation chosen, smoke results, post-restart heartbeat.
2. Update fact `albion_acct_3_watchdog_running` with the patched v3.1 description.
3. Add a memory-worthy entry (`feedback_*` style) about this class of regression: *multi-crop voting needs adversarial test inputs from non-target states, not just from target-state variants*.

## Commit-or-falsify contract
- 30min hard cap. Else `analysis/watchdog_v3_regression_partial_2026-05-25.md` with mitigations enumerated.
- 10min heartbeat → `analysis/heartbeat.log`.
- `/tmp/abort_albion-acct3-watchdog` → commit partial + exit.

## Constraints (HARD)
- **action-loop is intentionally stopped** by orchestrator this tick. Do NOT restart it until recovery is verified.
- gamestate-local-* services are running but x/z=null (mask drift; separate backlog row, do NOT touch).
- **NEVER touch** `acct3-albion.service`, `acct3-xtigervnc.service`, or `bin/relogin_acct3.py`'s clipboard-paste logic.
- relogin_acct3.py imports `detect_state.classify_image()`; whatever fix you ship MUST keep that 2-tuple unpack working.

## Memory references
- `[[falsify-mechanism-not-path]]` — closure mechanism-scoped.
- `[[audit-before-falsify]]` — verify before closing v3 as "broken".
- `[[macromanage-workers]]` — you choose which of the 3 mitigations.

## Files / endpoints
- Briefing: `/home/sdancer/orchestrator/briefings/albion-acct3-watchdog.md`
- Worktree: `/home/sdancer/albion-acct3-watchdog/`
- Live login screenshot: `/tmp/v_1779692406.png`
- v3 code: `bin/detect_state.py`
- Relogin script: `bin/relogin_acct3.py`
- Latest heartbeat (showing false-match): `var/log/heartbeat.jsonl`
- Fact to update: `albion_acct_3_watchdog_running`
