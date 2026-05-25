# clientless-login-stage-gap — Stage-by-stage gap analysis for clientless fresh-account login

## Role & workdir
Synthesis/analysis worker. Workdir: `/home/sdancer/nmss-emu-clientless-login-stage-gap`. **NO outbound HTTPS to apis.netmarble.com or any Netmarble host this turn.** Desk-research + file-system survey only.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login` (opened cycle 922, refined cycle 926)
- **sub_goal_key**: `clientless-login-stage-gap` (first path; ranks the next attacks)

## Terminal success criterion (cycle 926 refinement — read this carefully)
**GS (game server) accepts a security packet** on the GS TCP connection. Read fact `nmss_clientless_login_goal_refined_2026_05_17` via `harness fact-get` for full text. The refined 5-stage metric:
1. Adapt sibling-game `vampir.cpp` clientless login pipeline for thered's gameCode → NetmarbleSToken JWT + I_PID (= MAGIC32)
2. Open GS TCP connection using existing transport-encoder (174/174 byte-exact, fact `nmss_transport_encoder_167_round_tripped`)
3. Receive challenge frame from GS
4. Compute cert via cert-rust-repro pure-Rust pipeline using fresh MAGIC32
5. Send security packet (cert in transport framing) — **GS accepts** = success

## Hypothesis
A stage-by-stage decomposition of the clientless-login chain maps cleanly onto the existing fact corpus, and at least one stage has an unattacked path with bounded cost (<6h, no user resource ask). The likely top recommendation is "adapt-vampir-to-thered" since the sibling pipeline already shipped end-to-end (facts `vampir.clientless_pipeline_complete` 2026-03-29, `clientless_login_complete` 2026-03-30).

## Falsification (3 outcomes)
- (a) **Stage map produced + ≥1 unattacked stage identified with concrete bounded-cost path proposal** → SUCCESS. Fact: `clientless_login_stage_gap_complete_<path_name>_proposed`. Orchestrator spawns that path on the next cycle.
- (b) **Stage map produced but every stage is either CLOSED-final or gated on a user resource ask** → `stalled-meta` shape. Fact: `clientless_login_all_stages_blocked_or_closed`.
- (c) **The 5-stage decomposition itself is wrong** (missing stage, stages don't compose). Fact: `clientless_login_stage_decomposition_invalid`. Refine.

## Progress so far (turn 1 complete, this is turn 2)
- **Turn 1 DONE** (2026-05-17 ~14:22Z): `analysis/fact_index.md` (143 lines, 11.4 KB) catalogues the harness-side fact corpus. Surfaced sibling-game `vampir.cpp` pipeline body of work + correct cpp-auth routes `/cpp-auth/v1/fetch-player` and `/cpp-auth/v1/sign-in` (cycle 215's bare `/cpp-auth/` probe missed the `/v1/*` sub-path — that's a key falsifier-was-wrong-substrate observation).
- **Turn 2 STARTS NOW.** The synthesis doc has NOT been written yet. The Codex agent on turn-2-attempt-1 stopped early after concluding "task 1 is already done" — the briefing was rewritten (this version) to make the synthesis doc the explicit task 1.

## Task 1 (THIS turn — the only task for now)

**Produce `/home/sdancer/nmss-emu-clientless-login-stage-gap/analysis/stage_gap_2026-05-17.md`.**

Inputs to read first:
- `/home/sdancer/nmss-emu-clientless-login-stage-gap/analysis/fact_index.md` (your own prior output)
- `/home/sdancer/orchestrator/analysis/hypotheses.md` lines 100–180 (cycle 200–228 corpus)
- `/home/sdancer/nmss-emu/WIKI.md` (cycles 200–228 narrative)
- `harness fact-get nmss_clientless_login_goal_refined_2026_05_17`
- `harness fact-get vampir.cpp_auth_binary_format`
- `harness fact-get vampir.token_exchange_endpoints`
- `harness fact-get vampir.clientless_pipeline_complete`
- `harness fact-get clientless_login_complete`

Locate vampir artifacts on disk (one or more of these will likely hit):
- `find /home/sdancer -maxdepth 3 -type d \( -name '*vampir*' -o -name '*clientless*' \) 2>/dev/null`
- `ls /home/sdancer/games/autoproto/ 2>/dev/null` (per `feedback_inapp_webview_cdp` memory, c670 artifacts landed there)
- `find /home/sdancer -maxdepth 4 -name 'netmarble_*.json' -o -name 'cpp_auth*' 2>/dev/null | head`
- `rg -l 'cpp-auth/v1' /home/sdancer/ --type-add 'py:*.py' -t py 2>/dev/null | head`

Doc sections (must include all):

1. **Refined 5-stage table** with columns: `stage | knowns | known-falsifications | unknowns | bounded-cost-attacks | user-resource-asks`.
2. **Fact survey**: per stage, cite the relevant harness facts and on-disk artifacts (with absolute paths if found).
3. **Critical clarification**: cycle 215 falsified `/cpp-auth/` (bare path) under JSON, NOT `/cpp-auth/v1/fetch-player` or `/cpp-auth/v1/sign-in` under protobuf. Make this point explicit so a future planner doesn't re-falsify-by-association.
4. **Ranked next-path table**: 3 candidates with bounded cost. For each: predicted Δstages, cost in h, user-resource ask y/n/conditional, distinct-from-falsified justification. At least one MUST be the "adapt-vampir-to-thered" path (the sibling-pipeline transplant).
5. **Top recommendation**: ONE sentence naming the next worker to spawn, with the gameCode swap + cert-pipeline glue + GS-packet send chain as the deliverable.
6. **Closing fact**: `harness fact-set clientless_login_stage_gap_complete_<top_path_name>_proposed "<one-line summary>"`.
7. **Final line of the doc**: `CLIENTLESS_LOGIN_STAGE_GAP_DONE`.

## Constraints & gotchas
- **NO HTTPS to Netmarble.** Stage-3+ probes happen in the *spawned* path (next cycle), not here.
- **Memory budget**: 256 MB RSS. Per `[[feedback_bulk_enumeration_memory_budget]]`.
- **Don't re-falsify-by-association**: `cycle 215 falsified bare /cpp-auth/ JSON` does NOT close `/cpp-auth/v1/fetch-player` protobuf — different route, different content-type, different sub-path. Honor `[[feedback_planner_advisory_overridable]]`: the 6-in-a-row closure at cycle 228 is *advisory*.
- **Cap path proposals at 3.** Bound the work.
- **DO NOT redo task 1 from the prior briefing**. The fact_index.md is already written; just cite it.

## Relevant files / references
- Worker's own prior output: `/home/sdancer/nmss-emu-clientless-login-stage-gap/analysis/fact_index.md`
- Hypotheses ledger: `/home/sdancer/orchestrator/analysis/hypotheses.md`
- Wiki: `/home/sdancer/nmss-emu/WIKI.md`
- Memory rules: `[[feedback_impossibility_caution]]`, `[[feedback_inapp_webview_cdp]]`, `[[feedback_planner_advisory_overridable]]`, `[[feedback_bulk_enumeration_memory_budget]]`, `[[feedback_check_existing_decoder_before_re]]`
- Harness binary: `/home/sdancer/orchestrator/harness`
