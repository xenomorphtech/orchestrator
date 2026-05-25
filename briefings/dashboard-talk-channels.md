# dashboard-talk-channels — split /talk into per-channel threads + Ctrl+Enter send

## Role & workdir
Codex worker, workdir `/home/sdancer/orchestrator`. Editing the running Flask app `web/app.py` (untracked, no worktree).

## Current goal / sub-goal
- **goal_key**: `dashboard_talk_panel`
- **sub_goal_key**: `talk-multi-channel-and-ctrl-enter`

## User request (verbatim)
> "split the dashboard talk in different talks, add a hotkey for send, ctrl+enter"

Interpretation: the single `/talk` thread is too coarse — the user wants multiple separate conversation channels (think Slack-channels or Discord-channels), each with its own message history. Also: pressing **Ctrl+Enter** in the textarea should submit the form (today the user has to click the button or wait for Enter handling, but plain Enter inserts newlines in a textarea).

## Success criteria
- `/talk` shows a **sidebar (or top tabs) of channels** and the currently-selected channel's thread.
- A way to create a new channel by name (simple `<form>` posting a `?new_channel=<name>` parameter, or a dedicated `/talk/new` route).
- Each channel persists to its own JSONL file under `analysis/talk_channels/<channel-slug>.jsonl` (or a single JSONL with a `channel` field — either works; per-file is slightly simpler).
- The existing `/talk` route continues to work — default channel is `general` (the old `analysis/talk.jsonl` content stays accessible, either migrated to `talk_channels/general.jsonl` or rendered when no channel is specified).
- **Ctrl+Enter inside the textarea submits the form.** Plain Enter still inserts a newline. Inline JS hook.
- The biome-injection feature (notify_orchestrator_pane) keeps working — pass the channel name through to the inject text so the orchestrator can tell which channel a `/talk` message came from.
- `orchestrator-dash.service` restarted with new code live; smoke verifies.
- Set fact `dashboard_talk_channels_live_2026_05_18=true`.
- Verdict at `analysis/dashboard_talk_channels_verdict.md`. Final line `DASHBOARD_TALK_CHANNELS_DONE`.

## Concrete tasks (do in order)

1. **Inspect existing `/talk` route** in `/home/sdancer/orchestrator/web/app.py` (around line 524). Note the existing helpers: `append_talk_entry(...)`, `notify_orchestrator_pane(...)`, the JSONL store at `analysis/talk.jsonl`, the sessionStorage-draft preservation JS from the prior fix.

2. **Add channel storage.** Create directory `analysis/talk_channels/` if missing. Channel store: each channel is a file `analysis/talk_channels/<slug>.jsonl`. Slug = lowercase, ascii alnum + `-`, max 32 chars. Default channel = `general` — if it doesn't exist yet, migrate the contents of `analysis/talk.jsonl` into it on first request (or just symlink — pick whichever is simpler; migrate-then-leave-old-empty is fine).

3. **Update routes:**
   - `GET /talk?c=<channel>` — show that channel; default to `general`.
   - `POST /talk?c=<channel>` — append to that channel; same `from=user` semantics; `notify_orchestrator_pane` called with prefix `[/talk#<channel> @ <ts>] <text>` so the orchestrator can tell channels apart.
   - `POST /talk/new` (or `GET /talk?new=<name>`) — create a new channel (just touches the JSONL file) and redirect to it.
   - Optional `GET /talk/api/channels` returning the list as JSON if helpful for navigation.

4. **UI changes:**
   - Sidebar (left, ~12rem wide) listing channels with a link `<a href="/talk?c=<slug>">`. Active channel highlighted.
   - At top of sidebar, a tiny form: `<form action="/talk/new" method="post"><input name="name" placeholder="new channel"><button>+</button></form>`.
   - Main pane: same chat thread as before but filtered to the current channel.
   - **Ctrl+Enter submit:** add a small `<script>` that listens for `keydown` on the textarea; if `e.ctrlKey && e.key === 'Enter'`, call `e.target.form.requestSubmit()` (or `submit()`).
   - Keep the existing sessionStorage draft preservation — but key it per channel: `talk_draft_v2:<channel>` so switching channels doesn't lose drafts.

5. **Restart `orchestrator-dash.service`** via `sudo systemctl restart orchestrator-dash.service`. Verify `is-active`. Smoke:
   ```bash
   curl -sS http://127.0.0.1:3030/talk | grep -cE 'ctrlKey.*Enter|requestSubmit'
   curl -sS http://127.0.0.1:3030/talk | grep -c 'talk_draft_v2'
   curl -sS http://127.0.0.1:3030/talk?c=general | grep -c 'channel'
   curl -sS -X POST -d 'name=test1' http://127.0.0.1:3030/talk/new -L >/dev/null
   ls analysis/talk_channels/
   ```

6. **Set fact + verdict** at `analysis/dashboard_talk_channels_verdict.md`. Final line `DASHBOARD_TALK_CHANNELS_DONE`.

## Constraints & gotchas
- **`web/app.py` is untracked** — edit in place. No worktree, no git commit.
- **No new dependencies.** Flask + stdlib + Tailwind CDN + inline JS only.
- **Don't break `notify_orchestrator_pane`** — keep it firing on `from=user` POSTs; just include the channel name in the prefix so the orchestrator can route.
- **Restart only via systemd**, never `nohup` from inside the worker — per memory `[[worker-artifact-isolation]]`.
- **Sanitize channel slugs.** Strip everything except `[a-z0-9-]`, lowercase, cap at 32 chars. Reject empty after sanitization.
- **Preserve the existing draft-restore JS** from the prior fix (sessionStorage `talk_draft_v1`), but per-channel — rename to `talk_draft_v2:<slug>`.

## Relevant files / references
- `/home/sdancer/orchestrator/web/app.py` — the Flask app
- `/home/sdancer/orchestrator/analysis/talk.jsonl` — existing single-thread log (migrate or keep alongside)
- `/home/sdancer/orchestrator/analysis/talk_channels/` — NEW directory to create
- `/etc/systemd/system/orchestrator-dash.service` — durable systemd unit
- Memory: `[[worker-artifact-isolation]]`
- Prior verdict: `analysis/dashboard_talk_preserve_draft_verdict.md` (sessionStorage pattern reference)
