# dashboard-talk-notify — inject /talk user messages into orchestrator pane

## Role & workdir
Codex worker, workdir `/home/sdancer/orchestrator`. Editing the running Flask app `web/app.py` (untracked, no worktree).

## Current goal / sub-goal
- **goal_key**: `dashboard_talk_panel`
- **sub_goal_key**: `talk-to-orchestrator-pane-bridge`

## User request (verbatim)
> "http://term.orch.run:3030/talk < when a new message is appended, notify it here via biome injection"

Interpretation: when a `from=user` message is POSTed to `/talk`, push the text into the live orchestrator session's biome_term pane so the running orchestrator Claude sees it as input. This makes `/talk` a live channel into the running `/orchestrate` loop.

## Success criteria
- POST `/talk` with `text=hello` → message appended to `analysis/talk.jsonl` AND a corresponding line appears in the **orchestrator pane** (biome_term pane id `d277ed7a-3b9d-4cfa-8cb0-c7baa32150f9`, name `orchestrator`).
- Only `from=user` messages are forwarded (no echo loop — orchestrator's own future replies posted via the harness CLI must NOT trigger re-injection).
- The forwarded text is plainly visible — formatted like `[/talk @ ISO-TS] <message>` so the orchestrator session can distinguish it from other input.
- The `orchestrator-dash.service` systemd unit is restarted with the new code live; smoke test: POST a test message and confirm via `harness screen orchestrator --lines 5` that it appears.
- Set fact `dashboard_talk_orchestrator_injection_live_2026_05_18=true`.
- Single short verdict at `analysis/dashboard_talk_notify_verdict.md`. Final line `DASHBOARD_TALK_NOTIFY_DONE`.

## Concrete tasks (do in order)

1. **Read `/home/sdancer/orchestrator/web/app.py`** — locate the existing `talk_view()` route (around line 524) and the `append_talk_entry()` helper (around line 166). The POST handler at `/talk` currently calls `append_talk_entry("user", text)` then redirects.

2. **Add the biome injection helper.** New function `notify_orchestrator_pane(text: str)`:
   - Target pane id: `d277ed7a-3b9d-4cfa-8cb0-c7baa32150f9` (the `orchestrator` pane).
   - Use the harness CLI directly to avoid HTTP/API-key plumbing: `subprocess.Popen([HARNESS, "send", "orchestrator", f"[/talk @ {ts}] {text}"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)`. **Fire-and-forget** (don't block the HTTP response on the send — `harness send` can take several seconds).
   - Catch exceptions silently — the inject failing must not break the POST.

3. **Hook the POST handler.** Right after `append_talk_entry("user", text)` succeeds, call `notify_orchestrator_pane(text)`. Wrap in try/except so a biome failure doesn't propagate to the user.

4. **Anti-loop guard.** Only do the inject when `from == "user"` in the post handler. Orchestrator-side replies (which would land via a separate path like `/api/talk/reply` or future direct `append_talk_entry("orchestrator", …)` calls) must not be re-injected.

5. **Restart the systemd unit.** `sudo systemctl restart orchestrator-dash.service` — verify it comes back up with `is-active`, smoke-test GET `/talk` → 200.

6. **End-to-end smoke test.**
   ```bash
   curl -sS -X POST -d 'text=cycle-1500 notify smoke test' http://127.0.0.1:3030/talk -L --max-redirs 1 >/dev/null 2>&1
   sleep 2
   /home/sdancer/orchestrator/harness screen orchestrator --lines 6 | grep -F '[/talk @' || echo 'NO INJECT'
   ```
   Expect to see the line in the screen capture.

7. **Set fact + commit verdict.** `harness fact-set dashboard_talk_orchestrator_injection_live_2026_05_18 "..."` + short markdown at `analysis/dashboard_talk_notify_verdict.md`.

## Constraints & gotchas

- **`web/app.py` is untracked in git** — no worktree, edit in place. Don't try `git add web/app.py` unless asked.
- **Restart via systemd**: `orchestrator-dash.service` (NOT the cgroup-doomed `harness-worker@dashboard-talk` — that one died after the original task). The service was created in cycle 1475 to fix the cgroup-death pattern documented in memory `feedback_worker_artifact_isolation`.
- **Do NOT spawn the dashboard via `nohup &` from your worker** — same cgroup-death hazard. Use `sudo systemctl restart orchestrator-dash.service`.
- **Fire-and-forget the harness send** — Python's `subprocess.Popen([...], close_fds=True)` then return immediately. Blocking the POST handler for ~3s of `harness send` work would be unacceptable UX.
- **HARNESS path**: `/home/sdancer/orchestrator/harness` (the existing app already imports as `HARNESS`).

## Relevant files / references
- `/home/sdancer/orchestrator/web/app.py` — the Flask app
- `/home/sdancer/orchestrator/analysis/talk.jsonl` — append-only chat log
- `/etc/systemd/system/orchestrator-dash.service` — durable systemd unit (User=sdancer, WorkingDirectory=/home/sdancer/orchestrator, Restart=on-failure)
- biome pane: id `d277ed7a-3b9d-4cfa-8cb0-c7baa32150f9` name `orchestrator`
- Memory: `[[worker-artifact-isolation]]` (why we use systemd, not nohup)
- Existing verdict pattern: `analysis/dashboard_talk_verdict.md` (from cycle 1475 worker — though that file was lost when talk.jsonl was clobbered, the route code remains)
