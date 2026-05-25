# dashboard-talk-channel-controls — add per-channel delete & clear buttons to /talk

## Role & workdir
Codex worker, workdir `/home/sdancer/orchestrator`. Editing the running Flask app `web/app.py` (untracked, no worktree).

## Current goal / sub-goal
- **goal_key**: `dashboard_talk_panel`
- **sub_goal_key**: `talk-channel-delete-and-clear-buttons`

## User request (verbatim)
> "add a channel delete button and a clear button"

Interpretation: add UI affordances on the `/talk` page to (a) delete a channel entirely (removes the JSONL file from `analysis/talk_channels/`) and (b) clear a channel (truncates the JSONL file to zero bytes but keeps it). Both should be visible per-channel in the sidebar, and ideally also next to the channel header at the top of the thread view.

## Success criteria
- Each channel row in the `/talk` sidebar gains a `[clear]` and `[delete]` link/button (small, unobtrusive — `text-zinc-500 hover:text-red-500` styling fits the dark theme).
- POST `/talk/clear?c=<slug>` truncates `analysis/talk_channels/<slug>.jsonl` to zero bytes and redirects back to `/talk?c=<slug>`.
- POST `/talk/delete?c=<slug>` removes `analysis/talk_channels/<slug>.jsonl` from disk and redirects to `/talk?c=general` (the default channel).
- The `general` channel cannot be deleted (delete on `general` returns 400 or no-op + flash; clear is allowed).
- Confirmation: simple JS `confirm()` dialog before submitting either action.
- `orchestrator-dash.service` restarted; smoke verifies endpoints return 302 on POST + 200 on subsequent GET, and the file system reflects the expected state.
- Set fact `dashboard_talk_channel_controls_live_2026_05_18=true`.
- Verdict at `analysis/dashboard_talk_channel_controls_verdict.md`. Final line `DASHBOARD_TALK_CHANNEL_CONTROLS_DONE`.

## Concrete tasks (do in order)

1. **Read `/home/sdancer/orchestrator/web/app.py`** — locate the existing `/talk` GET/POST handler and the channel sidebar rendering code (added by the prior `dashboard-talk-channels` worker, see verdict `analysis/dashboard_talk_channels_verdict.md`).

2. **Add two new POST routes** before the `if __name__ == "__main__":` line:
   - `POST /talk/clear` — reads `request.form.get('c')` or `request.args.get('c')`, sanitizes via the same channel-slug regex used elsewhere, truncates `analysis/talk_channels/<slug>.jsonl` (open with mode `'w'` then close), redirect 303 to `/talk?c=<slug>`.
   - `POST /talk/delete` — same slug parsing; if slug is `general`, return a 400 with a short message ("cannot delete the default channel"); else `os.remove(...)` the file (guard against FileNotFoundError) and redirect 303 to `/talk?c=general`.
   - Both should fire `notify_orchestrator_pane()` with prefix `[/talk#<channel> ADMIN clear|delete]` so the orchestrator pane sees these admin actions.

3. **Sidebar UI**: in the channel-list rendering, after each `<a href="/talk?c=<slug>">`, append two small forms (inline):
   ```html
   <form method="post" action="/talk/clear" style="display:inline" onsubmit="return confirm('Clear all messages in #<slug>?');">
     <input type="hidden" name="c" value="<slug>">
     <button type="submit" class="text-xs text-zinc-500 hover:text-amber-400 ml-1" title="clear">clr</button>
   </form>
   <form method="post" action="/talk/delete" style="display:inline" onsubmit="return confirm('Delete channel #<slug>?');">
     <input type="hidden" name="c" value="<slug>">
     <button type="submit" class="text-xs text-zinc-500 hover:text-red-500 ml-1" title="delete">×</button>
   </form>
   ```
   Suppress the `×` (delete) button for the `general` channel.

4. **Header buttons** (optional but nice): next to the channel name at the top of the thread, render the same two compact actions.

5. **Restart via systemd**:
   ```bash
   sudo systemctl restart orchestrator-dash.service
   systemctl is-active orchestrator-dash.service
   curl -sS -o /dev/null -w 'HTTP %{http_code}\n' http://127.0.0.1:3030/talk
   ```

6. **End-to-end smoke**:
   ```bash
   # Create a throwaway channel
   curl -sS -X POST -d 'name=ephem-test' http://127.0.0.1:3030/talk/new -L >/dev/null
   ls analysis/talk_channels/ | grep ephem-test
   # Post a message via append (skip the form for speed)
   echo '{"ts":"2026-05-18T18:00:00+02:00","from":"orchestrator","text":"smoke"}' >> analysis/talk_channels/ephem-test.jsonl
   wc -l analysis/talk_channels/ephem-test.jsonl
   # Clear it
   curl -sS -X POST -d 'c=ephem-test' http://127.0.0.1:3030/talk/clear -o /dev/null -w 'clear → HTTP %{http_code}\n'
   wc -c analysis/talk_channels/ephem-test.jsonl  # expect 0
   # Delete it
   curl -sS -X POST -d 'c=ephem-test' http://127.0.0.1:3030/talk/delete -o /dev/null -w 'delete → HTTP %{http_code}\n'
   ls analysis/talk_channels/ | grep -c ephem-test  # expect 0
   # Confirm general is protected
   curl -sS -X POST -d 'c=general' http://127.0.0.1:3030/talk/delete -o /dev/null -w 'delete-general → HTTP %{http_code}\n'  # expect 400
   ```

7. **Set fact + verdict.** `harness fact-set dashboard_talk_channel_controls_live_2026_05_18 "..."` + verdict at `analysis/dashboard_talk_channel_controls_verdict.md`. Final line `DASHBOARD_TALK_CHANNEL_CONTROLS_DONE`.

## Constraints & gotchas

- **`web/app.py` is untracked** — edit in place. No worktree, no git commit.
- **Use the existing channel-slug sanitizer** for parsing `c=<slug>` to avoid path traversal.
- **NEVER delete `general`** — the default channel is protected. Clear on general is allowed.
- **Restart only via systemd** per memory `[[worker-artifact-isolation]]`. Never `nohup`.
- **303 not 302** for POST-redirect-GET (the prior /talk POST handler used 302 which breaks `curl -L`). New endpoints should use 303.
- **fire-and-forget** the biome inject to avoid blocking the POST.
- **No new dependencies.** Flask + stdlib + Tailwind CDN only.

## Relevant files / references
- `/home/sdancer/orchestrator/web/app.py` — the Flask app
- `/home/sdancer/orchestrator/analysis/talk_channels/` — directory containing per-channel JSONL files
- `/home/sdancer/orchestrator/analysis/dashboard_talk_channels_verdict.md` — prior channels-feature verdict (reference for slug regex / form patterns)
- `/etc/systemd/system/orchestrator-dash.service` — durable systemd unit
- Memory: `[[worker-artifact-isolation]]`
