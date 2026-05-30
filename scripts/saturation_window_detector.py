#!/usr/bin/env python3
"""
Post-saturation window detector — cycle-76 backlog row ship.

Reads /home/sdancer/orchestrator/analysis/paths.json, computes hours since
each goal's metric last moved, classifies saturation, and surfaces a
recommendation. Intended to be called from each /orchestrate tick during
SENSE to produce a programmatic signal for the orchestrator's brainstorm
step.

Per [[no-time-based-llm-discipline]]: this is NOT a wall-clock gate. It's a
post-hoc detector that summarizes a state already implicit in paths.json,
making the saturation signal explicit so the planner / orchestrator can
brainstorm-pivot when warranted.

Outputs:
  - JSON to stdout
  - /tmp/saturation_window_status.json (durable single-file mirror)
  - Exit code 1 if ≥1 goal is long-saturated (≥24h), else 0
"""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

PATHS_JSON = Path("/home/sdancer/orchestrator/analysis/paths.json")
STATUS_OUT = Path("/tmp/saturation_window_status.json")

THRESHOLDS = [
    (6.0,  "healthy"),
    (12.0, "warming"),
    (24.0, "saturated"),
]
LONG = "long-saturated"

ACTIVE_PATH_STATUSES = {"progressing", "active", "stalled", "at-risk", "saturated-hold",
                       "paused-mid-progress", "mechanism-armed-awaiting-natural-event",
                       "infra-blocked", "infra-blocked-being-bypassed"}


def hours_since(iso_str):
    if not iso_str:
        return None
    s = iso_str.replace("Z", "+00:00")
    try:
        dt = datetime.fromisoformat(s)
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return (datetime.now(timezone.utc) - dt).total_seconds() / 3600.0


def classify(hours):
    if hours is None:
        return "unknown"
    for thresh, label in THRESHOLDS:
        if hours < thresh:
            return label
    return LONG


def count_active(paths):
    n = 0
    for p in paths or []:
        s = (p.get("status") or "").lower()
        st = (p.get("status_tag") or "").lower()
        # "done" / "retired" / "path-dropped" / "falsified" => inactive
        if s in {"done", "retired", "path-dropped", "falsified", "mechanism-dropped"}:
            continue
        # otherwise count as active candidate
        n += 1
    return n


def main():
    if not PATHS_JSON.exists():
        print(json.dumps({"error": f"missing {PATHS_JSON}"}), file=sys.stderr)
        return 2

    data = json.loads(PATHS_JSON.read_text())
    goals = data.get("goals", {})

    per_goal = []
    long_count = 0
    saturated_count = 0
    done_count = 0
    user_gate_count = 0
    for gkey, g in goals.items():
        cur = g.get("current")
        tgt = g.get("target")
        # Done filter: current met/exceeded target → not actionable saturation
        is_done = (isinstance(cur, (int, float)) and isinstance(tgt, (int, float))
                   and cur >= tgt)
        # User-gate filter: closure_kind=done-with-caveat AND/OR explicit user-gate flag
        # means the saturation is expected (waiting on user-controlled input that
        # the orchestrator cannot supply). Suppress alert classification but keep
        # the row in the output for transparency.
        is_user_gated = bool(g.get("gate_resolution_required_at_user_level"))
        hrs = hours_since(g.get("last_move_at"))
        if is_done:
            cls = "done"
            done_count += 1
        elif is_user_gated:
            cls = "user-gate-blocked"
            user_gate_count += 1
        else:
            cls = classify(hrs)
            if cls == LONG:
                long_count += 1
            elif cls == "saturated":
                saturated_count += 1
        per_goal.append({
            "goal": gkey,
            "metric": g.get("metric_name", "?"),
            "current": cur,
            "target": tgt,
            "last_move_at": g.get("last_move_at"),
            "hours_since_move": round(hrs, 2) if hrs is not None else None,
            "classification": cls,
            "active_path_count": count_active(g.get("paths", [])),
            "user_gated": is_user_gated,
        })

    # Sort: actionable (long-sat/sat/warming/healthy) first, then user-gated, then done.
    def sort_key(r):
        cls = r["classification"]
        tier = 0 if cls in (LONG, "saturated", "warming", "healthy") else (1 if cls == "user-gate-blocked" else 2)
        return (tier, -(r["hours_since_move"] or -1.0))
    per_goal.sort(key=sort_key)

    if long_count >= 1:
        rec = (f"PIVOT — {long_count} not-done, not-user-gated goal(s) ≥24h without metric movement; "
               "spawn fresh hypothesis from backlog or 5-paths-ranked rule")
    elif saturated_count >= 1:
        rec = (f"BRAINSTORM — {saturated_count} not-done, not-user-gated goal(s) 12-24h without metric movement; "
               "per-tick blocked-goal brainstorm should consider lane pivot")
    else:
        rec = "STEADY — no actionable goals saturated beyond 12h threshold"

    status = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "goals_total": len(per_goal),
        "goals_done": done_count,
        "goals_user_gate_blocked": user_gate_count,
        "goals_long_saturated": long_count,
        "goals_saturated": saturated_count,
        "alert": long_count >= 1,
        "recommendation": rec,
        "per_goal": per_goal,
    }

    STATUS_OUT.write_text(json.dumps(status, indent=2))
    print(json.dumps(status, indent=2))
    return 1 if status["alert"] else 0


if __name__ == "__main__":
    sys.exit(main())
