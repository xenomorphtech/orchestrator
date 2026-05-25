# dashboard-talk-preserve-draft — keep textarea content across /talk auto-refresh

## Role & workdir
Codex worker, workdir `/home/sdancer/orchestrator`. Editing the running Flask app `web/app.py` (untracked, no worktree).

## Current goal / sub-goal
- **goal_key**: `dashboard_talk_panel`
- **sub_goal_key**: `talk-textarea-survives-refresh`

## User request (verbatim)
> "refresh of the dashboard talk is cleaning the chat box, can fix?"

Interpretation: the `/talk` page meta-refreshes every 10s. Mid-typing, the user's textarea content is wiped on each refresh. Fix so the in-progress draft survives auto-refresh.

## Success criteria
- Open `http://127.0.0.1:3030/talk`, type into the textarea, wait through one auto-refresh (≥10s), and the typed text is **still in the textarea after refresh**.
- The auto-refresh itself still works (replies from orchestrator continue to surface).
- No new server-side state required — the persistence is in the browser only (sessionStorage or localStorage).
- Bonus (preferred): if the textarea is non-empty when the meta-refresh would fire, **defer the refresh** (extend / postpone) so the user isn't interrupted mid-thought. Easiest: replace the meta-refresh with a small JS timer that skips when `document.activeElement === textarea` OR textarea has content.
- `orchestrator-dash.service` restarted with new code live; smoke test verifies behavior.
- Set fact `dashboard_talk_draft_preserved_2026_05_18=true`.
- Short verdict at `analysis/dashboard_talk_preserve_draft_verdict.md`. Final line `DASHBOARD_TALK_PRESERVE_DRAFT_DONE`.

## Concrete tasks (do in order)

1. **Locate the `/talk` route handler in `web/app.py`** (around line 524 per prior briefings). Inspect the HTML it emits — find the `<textarea name=text ...>` and the `refresh=10` parameter (or `<meta http-equiv=refresh ...>` tag). Keep changes localized to this route's HTML.

2. **Add tiny inline JS** to the `/talk` page body. Implementation suggestion (pick one):
   - **Approach A — sessionStorage + pause refresh while typing**:
     ```html
     <script>
       (function () {
         const ta = document.querySelector('textarea[name="text"]');
         if (!ta) return;
         const KEY = 'talk_draft_v1';
         // Restore.
         const saved = sessionStorage.getItem(KEY);
         if (saved && !ta.value) ta.value = saved;
         // Persist on input.
         ta.addEventListener('input', () => sessionStorage.setItem(KEY, ta.value));
         // Clear on submit so the next page load doesn't re-fill.
         ta.form && ta.form.addEventListener('submit', () => sessionStorage.removeItem(KEY));
         // Pause auto-refresh while the user is actively typing.
         //   Remove the <meta http-equiv=refresh ...> and replace with a JS timer
         //   that defers itself if textarea has focus OR non-empty content.
         document.querySelectorAll('meta[http-equiv="refresh"]').forEach(m => m.remove());
         function tick() {
           const busy = document.activeElement === ta || ta.value.trim().length > 0;
           if (!busy) { location.reload(); }
           else { setTimeout(tick, 5000); }  // re-check every 5s while busy
         }
         setTimeout(tick, 10000);
       })();
     </script>
     ```
   - The above is the recommended approach. Tweak as needed to fit the existing template style.

3. **Render the HTML**: if the `body` is composed as an HTML string passed to `render(...)`, append the `<script>` block inline. Verify the new JS is reachable via curl (`curl -s http://127.0.0.1:3030/talk | grep -c 'talk_draft_v1'` ≥ 1).

4. **Restart the dashboard via systemd** (NOT `nohup` from inside the worker — see [[worker-artifact-isolation]]):
   ```bash
   sudo systemctl restart orchestrator-dash.service
   systemctl is-active orchestrator-dash.service
   curl -sS -o /dev/null -w 'HTTP %{http_code}\n' http://127.0.0.1:3030/talk
   ```

5. **End-to-end smoke**:
   - GET /talk, confirm the new JS is present.
   - Simulate browser: confirm via inline check that the meta-refresh tag is gone (or, if you kept the tag, that the JS overrides it) and the JS timer logic is in place.
   - Visual: not strictly required, but if you can use a headless browser smoke (puppeteer/playwright would be overkill — just curl + grep is enough), do it.

6. **Set fact + write verdict**: `harness fact-set dashboard_talk_draft_preserved_2026_05_18 "..."` + short markdown at `analysis/dashboard_talk_preserve_draft_verdict.md`. Final line `DASHBOARD_TALK_PRESERVE_DRAFT_DONE`.

## Constraints & gotchas

- **`web/app.py` is untracked** — edit in place, no worktree, no git commit.
- **NO new dependencies.** Flask + stdlib + Tailwind CDN only. The script tag is plain JS, no framework.
- **Don't break the existing `body|safe` rendering pattern** — the `<script>` should be inside the body HTML string that gets `|safe`'d, NOT inside Jinja-escaped user content.
- **Restart only via systemd**, never nohup — per memory `worker-artifact-isolation` (c1475 reference).
- **Anti-loop guard from notify feature must remain intact** — only modify the rendered HTML, do NOT touch the POST handler or `notify_orchestrator_pane()`.
- **Don't remove the `refresh` parameter from the `render()` call** unless you also keep some form of auto-refresh working (a JS reload is fine substitute). The orchestrator's replies still need to surface promptly when the user isn't typing.

## Relevant files / references
- `/home/sdancer/orchestrator/web/app.py` — Flask app (untracked)
- `/etc/systemd/system/orchestrator-dash.service` — durable systemd unit
- `/home/sdancer/orchestrator/analysis/talk.jsonl` — chat store (read-only for this task)
- Prior briefings: `briefings/dashboard-talk.md`, `briefings/dashboard-talk-notify.md`
- Memory: `[[worker-artifact-isolation]]`
