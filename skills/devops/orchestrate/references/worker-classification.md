# Worker classification rules

Extracted from `orchestrate.md` — Worker handling section. These rules govern how to read a pane and decide what to do.

## Classification

| Status | Detection |
|---|---|
| **dead** | Pane terminated, OR HTTP 404 on screen fetch. |
| **working** | `idle_seconds == 0`, OR last 20 rows contain `Working (` (Codex's working indicator), OR spinner / processing keywords (`thinking`, `analyzing`, `Hatching`, `running`) in the last 20 rows. |
| **stuck** | Last 20 rows contain an error pattern (`traceback`, `exception`, `error:`, `permission denied`, `segmentation fault`, `command not found`) AND no working indicator. |
| **idle** | `idle_seconds > 0`, last non-empty row looks like a prompt (`❯` or `›`), no working indicator. |
| **stuck (stale)** | No output change for 10+ minutes across cycles. |

**Codex caveat:** Codex shows `›` at the bottom of the screen even while working. NEVER classify a Codex pane as idle from the prompt alone — check `idle_seconds` and look for `Working (` in the rows *above* the prompt.

## Routing rules

- **idle + on a progressing path** — send `Continue.` (or `Continue. <one-line summary of their own stated next step>` if their last output names a clear next step). Do not re-explain what they already said.
- **idle + on a stalled path** — do not nudge. Wait for the divergence rule to retire the path.
- **stuck** — read the error; if cross-pollinatable from a sibling path's fact, redirect with the fact; otherwise treat as non-progress and let the path's stall counter advance.
- **dead on a live path** — rewrite briefing, restart from briefing-pointer prompt.
- **low context (~20% remaining) AND on a progressing path** — perform a context refresh (below). Otherwise let auto-compaction handle it; low context % does not mean exhausted.

## Context refresh (use sparingly)

1. Send: `Summarize your current goal, what you've accomplished, and the exact next 2-3 tasks. Be concise.`
2. Read the worker's response from the pane screen.
3. Rewrite `/home/sdancer/orchestrator/briefings/<agent>.md` using the worker's summary + the latest facts and episode context.
4. Send `/clear`.
5. Send the canonical briefing-pointer prompt (see [worker-briefings.md](worker-briefings.md)).

Never paste the summary inline. Always drive workers off the briefing file so the briefing-pointer prompt works for both fresh spawns and post-clear restarts.
