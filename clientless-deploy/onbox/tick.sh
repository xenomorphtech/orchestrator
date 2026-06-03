#!/usr/bin/env bash
export IS_SANDBOX=1
# One orchestrate-lean tick for the clientless campaign. Run from $HOME/clientless.
set -u
cd "$HOME/clientless" || exit 1
mkdir -p analysis
exec 9>analysis/tick.lock
if ! flock -n 9; then
  echo "tick skipped: another tick is active $(date -u +%FT%TZ)" >> analysis/loop.log
  exit 0
fi

EPISODES_BEFORE=$(wc -l < analysis/episodes.log 2>/dev/null || echo 0)
TS=$(date -u +%H:%MZ)
PROMPT="You are the self-driving CLIENTLESS-ALBION orchestrator on this box. Your full operating \
instructions are in ~/.claude/commands/orchestrate-lean.md (a goal/subgoal control loop). Read it, then read \
analysis/goal_tree.json and analysis/STATE.md. Execute EXACTLY ONE tick (SENSE->EVALUATE->DECIDE->ACTUATE->RECORD) \
on the FRONTIER subgoal: do real work on the clientless codebase here in \$HOME/clientless (read/edit/run/test the \
existing crates + clientless-bot tooling; verify the frontier metric from ground truth). Then rewrite \
analysis/goal_tree.json (update the frontier subgoal status/stall/blocker/next_substep and tick+=1) and append ONE \
line 'tick <N> ${TS} <frontier> - <=10-word headline' to analysis/episodes.log. If (new tick %% 10 == 0): also \
CONDENSE analysis/STATE.md (distill, drop superseded) and note the condense in the episode. Respect the op5-login-CLOSED \
scope. Keep it to one concrete committed sub-step. Be terse."
claude --dangerously-skip-permissions -p "$PROMPT" >> analysis/tick_out.log 2>&1
SUMMARY_LINE=$(
  tail -n +"$((EPISODES_BEFORE + 1))" analysis/episodes.log 2>/dev/null \
    | grep -E '^tick [0-9]+ ' \
    | tail -n 1 || true
)
if [ -n "$SUMMARY_LINE" ]; then
  python3 - "$SUMMARY_LINE" <<'PY' >> analysis/db_mirror.log 2>&1
import json
import os
import pathlib
import subprocess
import sys
import traceback

summary = sys.argv[1]
home = pathlib.Path(os.environ.get("HOME", "/home/sdanced"))
clientless = home / "clientless"
harness = os.environ.get("HARNESS", str(home / "orchestrator" / "harness"))
server = os.environ.get("HARNESS_SERVER", "http://127.0.0.1:3001")
database = os.environ.get("HARNESS_DATABASE", "orchestrator-box")


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def run(args):
    cmd = [harness, "--server", server, "--database", database, *args]
    result = subprocess.run(cmd, text=True, capture_output=True)
    if result.stdout:
        print(result.stdout.rstrip())
    if result.stderr:
        print(result.stderr.rstrip(), file=sys.stderr)
    if result.returncode != 0:
        raise RuntimeError(f"{cmd[:5]} ... exited {result.returncode}")


try:
    tree_path = clientless / "analysis" / "goal_tree.json"
    tree = json.loads(tree_path.read_text())
    goal = tree.get("goal") or {}
    subgoals = tree.get("subgoals") or []
    frontier = next(
        (sg for sg in sorted(subgoals, key=lambda sg: sg.get("order", 9999))
         if sg.get("status") in {"pending", "active"}),
        subgoals[0] if subgoals else {},
    )

    goal_key = goal.get("key") or "clientless_albion_bot"
    goal_title = goal.get("title") or goal_key
    frontier_id = frontier.get("id") or "S1"
    frontier_title = frontier.get("title") or frontier_id
    status = frontier.get("status") or "active"
    owner = frontier.get("worker") or "clientless-lean-loop"
    stall = int(frontier.get("stall") or 0)

    progress = {
        "source": "clientless/onbox/tick.sh",
        "summary": summary,
        "goal_key": goal_key,
        "tick": goal.get("tick"),
        "metric": goal.get("metric"),
        "frontier": {
            "id": frontier_id,
            "title": frontier_title,
            "status": status,
            "stall": stall,
            "blocker": frontier.get("blocker"),
            "next_substep": frontier.get("next_substep"),
        },
    }

    run([
        "episode-add",
        "--agent-statuses", compact({"lean_loop": "tick_complete", "frontier": frontier_id}),
        "--actions-taken", compact({"mirrored_episode_line": summary, "db_mirror": "tick.sh"}),
        "--goal-progress", compact(progress),
        summary,
    ])
    run([
        "goal-add",
        goal_key,
        goal_title,
        "--detail", goal.get("done_when") or goal.get("scope_note") or "",
        "--status", "active",
        "--priority", "100",
        "--metadata", compact({
            "source": "goal_tree.json",
            "tick": goal.get("tick"),
            "metric": goal.get("metric"),
            "scope_note": goal.get("scope_note"),
        }),
    ])
    run([
        "sub-goal-add",
        frontier_id,
        goal_key,
        owner,
        frontier_title,
        "--detail", frontier.get("done_when") or "",
        "--status", status,
        "--priority", str(max(1, 100 - int(frontier.get("order") or 1))),
        "--instruction-text", frontier.get("next_substep") or "",
        "--stuck-guidance-text", frontier.get("blocker") or "",
        "--metadata", compact({
            "source": "goal_tree.json",
            "order": frontier.get("order"),
            "metric": frontier.get("metric"),
            "stall": stall,
        }),
    ])
    run([
        "path-add",
        f"{frontier_id} {frontier_title}",
        "--goal", goal_key,
        "--sub-goal", frontier_id,
        "--worker", owner,
        "--hypothesis", frontier.get("next_substep") or "",
        "--falsification", frontier.get("falsification") or "",
        "--status", status,
        "--stall-counter", str(stall),
        "--notes", frontier.get("blocker") or "",
        "--metadata", compact({
            "source": "goal_tree.json",
            "tick": goal.get("tick"),
            "done_when": frontier.get("done_when"),
        }),
    ])
    print(f"mirrored episode to SpacetimeDB: {summary}")
except Exception:
    print(f"failed to mirror episode to SpacetimeDB: {summary}", file=sys.stderr)
    traceback.print_exc()
    sys.exit(1)
PY
  if [ "$?" != 0 ]; then
    echo "db mirror failed $(date -u +%FT%TZ): $SUMMARY_LINE" >> analysis/loop.log
  fi
fi
echo "tick done $(date -u +%FT%TZ)" >> analysis/loop.log
