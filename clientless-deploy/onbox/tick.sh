#!/usr/bin/env bash
export IS_SANDBOX=1
# One orchestrate-lean tick for the clientless campaign. Run from $HOME/clientless.
set -u
cd "$HOME/clientless" || exit 1
mkdir -p analysis
CODEX_MODEL="${HARNESS_CODEX_MODEL:-${CODEX_MODEL:-gpt-5.5}}"
exec 9>analysis/tick.lock
if ! flock -n 9; then
  echo "tick skipped: another tick is active $(date -u +%FT%TZ)" >> analysis/loop.log
  exit 0
fi

EPISODES_BEFORE=$(wc -l < analysis/episodes.log 2>/dev/null || echo 0)
TS=$(date -u +%H:%MZ)
PROMPT="You are Codex running with model ${CODEX_MODEL}. You are the self-driving orchestrator on this box - a goal/subgoal control loop. Your full operating \
instructions are in ~/.claude/commands/orchestrate-lean.md. The loop is GOAL-DRIVEN: FIRST run \
'\$HOME/orchestrator/harness goals' to see the active campaign goal(s) and DRIVE EVERY active goal (do NOT work a \
hardcoded one). Then read analysis/goal_tree.json (your working decomposition: a goal + PARALLEL workstreams) and \
analysis/STATE.md. Execute EXACTLY ONE tick (SENSE->EVALUATE->DECIDE->ACTUATE->RECORD): pick the highest-leverage \
workstream that can move THIS tick. LOGIN/credential is just ONE workstream, NOT the whole goal - obtain the credential \
by RUNNING the proven in-client tools in facts[0] (albion_e2e.py / albion_ingame_register_login.py) on the REAL client; \
the clientless from-scratch login is gated for the CLIENTLESS surface ONLY, never a reason to stall the whole bot. Do \
real work (read/edit/run/test the crates + tooling; verify the metric from ground truth). Then rewrite \
analysis/goal_tree.json (update the worked workstream's status/stall/blocker/next_substep and tick+=1) and append ONE \
line 'tick <N> ${TS} <workstream> - <=10-word headline' to analysis/episodes.log. If (new tick %% 10 == 0): also \
CONDENSE analysis/STATE.md (distill, drop superseded) and note the condense in the episode. Keep it to one concrete \
committed sub-step. Be terse."
codex exec \
  --model "$CODEX_MODEL" \
  --dangerously-bypass-approvals-and-sandbox \
  --skip-git-repo-check \
  -C "$HOME/clientless" \
  "$PROMPT" >> analysis/tick_out.log 2>&1
SUMMARY_LINE=$(
  tail -n +"$((EPISODES_BEFORE + 1))" analysis/episodes.log 2>/dev/null \
    | grep -E '^tick [0-9]+ ' \
    | tail -n 1 || true
)
if [ -n "$SUMMARY_LINE" ]; then
  if ! python3 - "$SUMMARY_LINE" <<'PY' >> analysis/db_mirror.log 2>&1
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


def parse_stall(value):
    if value is None or value == "":
        return 0, None
    if isinstance(value, bool):
        return int(value), None
    if isinstance(value, (int, float)):
        return int(value), None
    text = str(value).strip()
    try:
        return int(text), None
    except ValueError:
        return 0, text


def join_note(*parts):
    return " | ".join(str(p).strip() for p in parts if str(p or "").strip())


try:
    tree_path = clientless / "analysis" / "goal_tree.json"
    tree = json.loads(tree_path.read_text())
    goal = tree.get("goal") or {}
    # broadened frame uses `workstreams`; older serial frame used `subgoals` - accept either
    items = tree.get("workstreams") or tree.get("subgoals") or []
    frontier = next(
        (it for it in sorted(items, key=lambda it: it.get("order", 9999))
         if it.get("status") in {"pending", "active"}),
        items[0] if items else {},
    )

    goal_key = goal.get("key") or "albion_bot"
    goal_title = goal.get("title") or goal_key
    frontier_id = frontier.get("id") or "WS1"
    frontier_title = frontier.get("title") or frontier_id
    status = frontier.get("status") or "active"
    owner = frontier.get("worker") or "lean-loop"
    stall, frontier_stall_text = parse_stall(frontier.get("stall"))
    frontier_blocker = join_note(frontier.get("blocker"), frontier_stall_text)

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
            "blocker": frontier_blocker or None,
            "stall_text": frontier_stall_text,
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
    # Push EVERY workstream as a sub-goal + path so the FULL decomposition is visible in the dashboard
    # (not just the frontier). This is the goal tree, materialised into the DB the dashboard reads.
    for idx, it in enumerate(items):
        wid = it.get("id") or f"WS{idx+1}"
        wtitle = it.get("title") or wid
        wstatus = it.get("status") or "active"
        wworker = it.get("worker") or owner
        wstall, wstall_text = parse_stall(it.get("stall"))
        wblocker = join_note(it.get("blocker"), wstall_text)
        run([
            "ws-set",
            goal_key,
            wid,
            "--title", wtitle,
            "--status", wstatus,
            "--stall", str(wstall),
            "--blocker", wblocker,
            "--next-substep", it.get("next_substep") or "",
            "--metric", it.get("metric") or "",
            "--done-when", it.get("done_when") or "",
            "--falsification", it.get("falsification") or "",
            "--worker", wworker,
            "--ord", str(int(it.get("order", idx) or idx)),
            "--metadata", compact({
                "source": "goal_tree.json",
                "order": it.get("order", idx),
                "stall_text": wstall_text or it.get("stall_text"),
            }),
        ])
        run([
            "sub-goal-add",
            wid,
            goal_key,
            wworker,
            wtitle,
            "--detail", it.get("done_when") or "",
            "--status", wstatus,
            "--priority", str(max(1, 100 - idx)),
            "--instruction-text", it.get("next_substep") or "",
            "--stuck-guidance-text", wblocker,
            "--metadata", compact({
                "source": "goal_tree.json",
                "order": it.get("order", idx),
                "metric": it.get("metric"),
                "stall": wstall,
                "stall_text": wstall_text,
            }),
        ])
        run([
            "path-add",
            f"{wid} {wtitle}",
            "--goal", goal_key,
            "--sub-goal", wid,
            "--worker", wworker,
            "--hypothesis", it.get("next_substep") or "",
            "--falsification", it.get("falsification") or "",
            "--status", wstatus,
            "--stall-counter", str(wstall),
            "--notes", wblocker,
            "--metadata", compact({
                "source": "goal_tree.json",
                "tick": goal.get("tick"),
                "metric": it.get("metric"),
                "done_when": it.get("done_when"),
                "stall_text": wstall_text,
            }),
        ])
    print(f"mirrored {len(items)} workstreams to SpacetimeDB: {summary}")
except Exception:
    print(f"failed to mirror episode to SpacetimeDB: {summary}", file=sys.stderr)
    traceback.print_exc()
    sys.exit(1)
PY
  then
    echo "db mirror failed $(date -u +%FT%TZ): $SUMMARY_LINE" >> analysis/loop.log
  fi
fi
echo "tick done $(date -u +%FT%TZ)" >> analysis/loop.log
