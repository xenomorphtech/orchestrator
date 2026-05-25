# workplace-restriction-investigation — Diagnose pdj8pyp3 workplace restriction; find clearing path

## Role & workdir
Desk-research worker. Workdir: `/home/sdancer/nmss-emu-workplace-restriction-investigation`. **NO HTTPS calls this turn.** Investigate existing artifacts + libUnreal.so strings + vampir protocol docs only.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login`
- **sub_goal_key**: `workplace-restriction-investigation` (parallel path; sibling of `adapt-vampir-to-thered` which is blocked on resource ask)

## Why this turn exists
Sibling path `adapt-vampir-to-thered` got cpp-auth/v1/sign-in to return `errorCode: 0` but `player.status: 2, playerStatus.restriction: ["workplace"]`. The lobby silently drops `PktLobbyLogin`. The same pid worked end-to-end 2026-03-29. **Hypothesis**: the `workplace` restriction is a clearable account-policy gate, and the clearing path may be visible in (a) the cpp-auth response itself (look for `nextStep`, `verificationUrl`, `actionRequired`), (b) libUnreal.so strings describing the restriction-handling client flow, (c) vampir or other Netmarble protocol docs.

## Falsification (3 outcomes)
- (a) **Clearable path identified** (e.g. a verification URL, an API endpoint, a self-service members.netmarble.com flow) → SUCCESS. Fact: `workplace_restriction_clearing_path_<endpoint_or_action>`. Sibling path can attempt clearing.
- (b) **`workplace` is a known anti-abuse flag with no client-side clearing** (e.g., requires Netmarble customer support) → falsifies the "self-serve clearable" hypothesis. Fact: `workplace_restriction_not_clientside_clearable`. Recommends fresh-account path.
- (c) **`workplace` is undocumented in the local corpus** → forces the new-account path by elimination. Fact: `workplace_restriction_undocumented_in_local_corpus`.

## Success criteria — what "done" looks like
**Primary deliverable**: `/home/sdancer/nmss-emu-workplace-restriction-investigation/analysis/workplace_restriction_2026-05-17.md` with:

1. **Full cpp-auth response audit**: re-parse `/home/sdancer/nmss-emu-adapt-vampir-to-thered/analysis/stage_1_3_capture/get_stoken_output.json` for EVERY key/value in `resultData` and `playerInfo`. Look for: `nextStep`, `verificationUrl`, `actionRequired`, `restrictionDetail`, `expiresAt`, anything URL-shaped, anything that looks like a clearing-action hint.
2. **libUnreal.so string survey** (memory-bound, ≤3 min wall time, ≤500 MB RSS): grep `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so` (158 MB ELF) for:
   - `workplace` (case-insensitive)
   - `restriction`, `playerStatus`, `restrictionType`
   - `clearRestriction`, `verifyRestriction`, `unblockAccount`
   - Any `member.*Url` literal that takes an action verb
3. **vampir / autoproto docs scan**: `rg -i 'workplace|restriction|playerStatus' /home/sdancer/games/vampir/ /home/sdancer/games/autoproto/` (no recursion past 3 levels).
4. **Verdict** matched to (a)/(b)/(c) above + recommended fact + recommended next path.

Print `WORKPLACE_RESTRICTION_INVESTIGATION_DONE` on the final line.

## Constraints
- **No HTTPS this turn.** Pure desk research.
- **Memory budget**: 512 MB hard.
- **Time budget**: 30 min wall time max.
- **DO NOT touch the pdj8pyp3 account credentials file** for any action that would mutate state (this turn is read-only on accounts).
- Honor `[[feedback_check_existing_decoder_before_re]]`: before grepping libUnreal.so, check if there's an existing strings dump anywhere under `/home/sdancer/nmss-emu*/` so we don't re-grep 158 MB.

## Progress so far
None — this is turn 1 of this parallel path. Sibling-path findings:
- `adapt-vampir-to-thered/analysis/stage_1_3_capture/get_stoken_output.json` — cpp-auth resultData with `restriction: ["workplace"]`
- `adapt-vampir-to-thered/analysis/stage_1_3_capture/validate_gameserver_failure.json` — lobby_login timeout
- `adapt-vampir-to-thered/analysis/stage_1_3_capture/lobby_diagnosis.md` — verdict: account-scoped gate

## Next 1 concrete task (THIS turn)
1. Produce `analysis/workplace_restriction_2026-05-17.md` per success criteria. Close with fact-set + `WORKPLACE_RESTRICTION_INVESTIGATION_DONE`.

## Relevant files
- Captured cpp-auth response: `/home/sdancer/nmss-emu-adapt-vampir-to-thered/analysis/stage_1_3_capture/get_stoken_output.json`
- Lobby diagnosis: `/home/sdancer/nmss-emu-adapt-vampir-to-thered/analysis/stage_1_3_capture/lobby_diagnosis.md`
- libUnreal.so: `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so`
- Vampir signup automation: `/home/sdancer/games/vampir/create_account/`
- Autoproto account store: `/home/sdancer/games/autoproto/accounts/`
- Harness binary: `/home/sdancer/orchestrator/harness`
