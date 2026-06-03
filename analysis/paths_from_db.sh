#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
harness="${HARNESS:-$repo_root/harness}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

"$harness" dump >"$tmpdir/dump.jsonl"
"$harness" path-list --json >"$tmpdir/paths.json"

jq -s '[.[] | select(.type == "goal")] | sort_by(.goal_key)' \
  "$tmpdir/dump.jsonl" >"$tmpdir/goals.json"

: >"$tmpdir/anchors.jsonl"
while IFS= read -r goal_key; do
  anchor_json="$("$harness" anchor-get --goal "$goal_key" --json 2>/dev/null || true)"
  if jq -e type >/dev/null 2>&1 <<<"$anchor_json"; then
    printf '%s\n' "$anchor_json" >>"$tmpdir/anchors.jsonl"
  fi
done < <(jq -r '.[].goal_key' "$tmpdir/goals.json")

jq -s '.' "$tmpdir/anchors.jsonl" >"$tmpdir/anchors.json"

jq -n \
  --slurpfile goals "$tmpdir/goals.json" \
  --slurpfile paths "$tmpdir/paths.json" \
  --slurpfile anchors "$tmpdir/anchors.json" '
  def metadata:
    (.metadata_json // "{}" | fromjson? // {});

  def clean:
    with_entries(select(.value != null and .value != ""));

  ($goals[0]) as $goals_arr
  | ($paths[0]) as $paths_arr
  | ($anchors[0] | map({key: .goal_key, value: .}) | from_entries) as $anchor_by_goal
  | {
      schema_version: 1,
      goals: (
        $goals_arr
        | map(
            . as $goal
            | ($goal | metadata) as $goal_meta
            | ($anchor_by_goal[$goal.goal_key] // {}) as $anchor
            | {
                key: $goal.goal_key,
                value: ({
                  title: $goal.title,
                  metric_name: ($anchor.metric_name // $goal_meta.metric),
                  current: $anchor.metric_current,
                  target: $anchor.metric_target,
                  status: $goal.status,
                  success_fact_key: $goal.success_fact_key,
                  priority: $goal.priority,
                  completion: $goal.detail,
                  paths: (
                    $paths_arr
                    | map(select(.goal_key == $goal.goal_key))
                    | sort_by(.path_name)
                    | map(
                        . as $path
                        | ($path | metadata | del(.source)) as $path_meta
                        | ({
                            name: $path.path_name,
                            worker: $path.worker,
                            worktree: $path.worktree,
                            substrate: $path.substrate,
                            hypothesis: $path.hypothesis,
                            falsification: $path.falsification,
                            status: $path.status,
                            stall_counter: $path.stall_counter,
                            last_metric_move_at: $path.last_metric_move_at,
                            notes: $path.notes
                          } + $path_meta
                          | clean)
                      )
                  )
                } | clean)
              }
          )
        | from_entries
      )
    }
'
