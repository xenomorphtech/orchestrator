# Worker briefings — required sections + SQL-backed source of truth

Every worker is paired with a briefing at `/home/sdancer/orchestrator/briefings/<agent>.md`. Required sections (in this order):

1. **Role & workdir** — one sentence + absolute path of the worktree.
2. **Current goal / sub-goal** — `goal_key` and `sub_goal_key` with one-line titles.
3. **Success criteria** — the fact key(s) that complete the path, and what "done" looks like concretely.
4. **Progress so far** — bullets summarising prior cycles: what was tried, what worked, what failed, artifacts produced (with absolute paths).
5. **Next 2–3 concrete tasks** — ordered, specific, actionable. No vague "keep going".
6. **Constraints & gotchas** — device assignments, non-obvious rules from memory, things the agent must NOT do, known pitfalls from prior episodes.
7. **Relevant files / references** — paths, URLs, fact keys.

Keep briefings under ~150 lines. Briefings are working documents — **rewrite, don't append**.

For input-injection / scraping / instrumentation paths, the briefing MUST list 3+ candidate mechanisms up front. Workers default to the easiest tool; the orchestrator counters by surfacing alternatives as first-class options.

## Briefings are SQL-backed (SoT = harness DB; .md = mirror)

Briefings live in the SpacetimeDB `briefings` table (body markdown + metadata: `goal_key`, `category`, `tags`, `archived`, `created_at`, `updated_at`). The flat `briefings/<name>.md` file is a **mirror** the CLI maintains so workers' "Read briefings/<name>.md" keeps working — but the **DB row is the source of truth**.

- **Persist a briefing:** after writing/rewriting `briefings/<name>.md`, run
  ```
  harness briefing-set <name> --body-file briefings/<name>.md --goal <goal_key> --category <cat> --tags <csv>
  ```
  (writes the SQL row + metadata AND re-mirrors the file; un-archives it). You can also pass `--body "<text>"` instead of `--body-file`. Always set `--goal` + `--category` (e.g. `live-driver` / `offline-prep` / `infra`) so the portfolio stays queryable.
- **Update metadata only:** `harness briefing-set-meta <name> --goal <k> --category <c> --tags <csv>`.
- **Read/list:** `harness briefing-get <name>` (body from SQL; `--materialize` re-writes the .md), `harness briefing-list [--archived|--only-archived] [--goal k] [--category c]`.
- **Archive / restore:** `harness briefing-archive <name>` (moves the .md to `briefings/_archived/`); `--restore` brings it back. Archived briefings are KEPT (history), just hidden from the default list + active dir.
- **Periodic archiving (housekeeping):** `harness briefing-archive-unused` archives every briefing with **no live agent**. Run it **every K=6 cycles** (alongside the planner) to keep the active set == the briefings actually driving live work. Non-destructive + reversible.

## Canonical briefing-pointer prompt

Use for every spawn, restart, and post-`/clear` refresh; also the `--default-task` value on `agent-add`:

```
Read /home/sdancer/orchestrator/briefings/<agent>.md — that is your full briefing. Then continue with task 1.
```
