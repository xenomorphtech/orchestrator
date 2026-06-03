#!/usr/bin/env python3
"""Import the legacy orchestrator analysis/paths.json into harness DB tables."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


DEFAULT_PATHS_JSON = Path("/home/sdancer/orchestrator/analysis/paths.json")
DEFAULT_HARNESS = Path("/home/sdancer/orch-harness-schema/harness")

PATH_FIELDS = {
    "name",
    "worker",
    "worktree",
    "hypothesis",
    "falsification",
    "status",
    "stall_counter",
    "last_metric_move_at",
    "predicted_delta",
    "substrate",
    "notes",
}


def text(value: Any) -> str:
    if value is None:
        return ""
    return str(value)


def optional(args: list[str], flag: str, value: Any) -> None:
    if value is not None and value != "":
        args.extend([flag, str(value)])


def numeric(value: Any) -> float | None:
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str) and value.strip():
        try:
            return float(value)
        except ValueError:
            return None
    return None


def run(cmd: list[str], dry_run: bool) -> None:
    if dry_run:
        print(" ".join(cmd))
        return
    subprocess.run(cmd, check=True)


def import_paths(harness: Path, paths_json: Path, dry_run: bool) -> tuple[int, int]:
    data = json.loads(paths_json.read_text())
    goals = data.get("goals", {})
    path_count = 0
    anchor_count = 0
    for goal_key, goal in goals.items():
        metric_current = numeric(goal.get("current"))
        metric_target = numeric(goal.get("target"))
        metric_name = text(goal.get("metric_name")) or "metric"
        understanding_parts = [
            text(goal.get("title")),
            f"status={text(goal.get('status'))}" if goal.get("status") is not None else "",
            text(goal.get("notes")),
            text(goal.get("completion")),
        ]
        current_understanding = "\n\n".join(part for part in understanding_parts if part)
        invariant_parts = [
            text(goal.get("decisions_locked")),
            text(goal.get("metric_semantics_note")),
            text(goal.get("metric_depth_note")),
            text(goal.get("metric_honesty_note")),
        ]
        invariants = "\n\n".join(part for part in invariant_parts if part)
        if metric_current is not None and metric_target is not None and current_understanding:
            cmd = [
                str(harness),
                "anchor-set",
                "--goal",
                goal_key,
                "--current-understanding",
                current_understanding,
                "--metric-name",
                metric_name,
                "--metric-current",
                str(metric_current),
                "--metric-target",
                str(metric_target),
            ]
            optional(cmd, "--invariants", invariants)
            run(cmd, dry_run)
            anchor_count += 1

        for path in goal.get("paths", []) or []:
            path_name = text(path.get("name"))
            if not path_name:
                raise ValueError(f"goal {goal_key} has a path without name")
            metadata = {
                key: value
                for key, value in path.items()
                if key not in PATH_FIELDS and value is not None
            }
            metadata["source"] = str(paths_json)
            cmd = [
                str(harness),
                "path-add",
                path_name,
                "--goal",
                goal_key,
                "--worker",
                text(path.get("worker")),
                "--worktree",
                text(path.get("worktree")),
                "--hypothesis",
                text(path.get("hypothesis")),
                "--falsification",
                text(path.get("falsification")),
                "--status",
                text(path.get("status")) or "active",
                "--stall-counter",
                str(int(path.get("stall_counter") or 0)),
                "--metadata",
                json.dumps(metadata, separators=(",", ":"), sort_keys=True),
            ]
            optional(cmd, "--last-metric-move-at", path.get("last_metric_move_at"))
            predicted_delta = numeric(path.get("predicted_delta"))
            if predicted_delta is not None:
                optional(cmd, "--predicted-delta", predicted_delta)
            optional(cmd, "--substrate", path.get("substrate"))
            optional(cmd, "--notes", path.get("notes"))
            run(cmd, dry_run)
            path_count += 1
    return path_count, anchor_count


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--paths-json", type=Path, default=DEFAULT_PATHS_JSON)
    parser.add_argument("--harness", type=Path, default=DEFAULT_HARNESS)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    path_count, anchor_count = import_paths(args.harness, args.paths_json, args.dry_run)
    print(f"imported paths={path_count} anchors={anchor_count}")


if __name__ == "__main__":
    main()
