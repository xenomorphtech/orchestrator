#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -n "${HARNESS:-}" ]]; then
  harness="$HARNESS"
elif [[ -x /home/sdancer/orchestrator/harness ]]; then
  harness="/home/sdancer/orchestrator/harness"
else
  harness="$repo_root/harness"
fi
source_json="${PATHS_JSON:-/home/sdancer/orchestrator/analysis/paths.json}"

if [[ ! -f "$source_json" ]]; then
  echo "paths_to_db: missing source JSON: $source_json" >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

clean_filter='
  def clean:
    with_entries(select(.value != null and .value != ""));
'

"$harness" dump >"$tmpdir/dump.jsonl"
"$harness" path-list --json >"$tmpdir/current_paths.json"

jq -s '[.[] | select(.type == "goal")]' "$tmpdir/dump.jsonl" >"$tmpdir/current_goals.json"

jq -c --arg source "$source_json" "$clean_filter"'
  .goals
  | to_entries[]
  | . as $entry
  | $entry.value as $goal
  | {
      key: $entry.key,
      title: ($goal.title // $entry.key),
      detail: ($goal.completion // ""),
      has_status: ($goal | has("status")),
      status: ($goal.status // ""),
      has_priority: ($goal | has("priority")),
      priority: ($goal.priority // 50),
      success_fact_key: ($goal.success_fact_key // ""),
      metadata: (
        ($goal
          | del(
              .title,
              .metric_name,
              .current,
              .target,
              .status,
              .success_fact_key,
              .priority,
              .completion,
              .paths
            )
          | clean)
        + (if ($goal.metric_name // "") != "" then {metric: $goal.metric_name} else {} end)
        + {source: $source}
      ),
      anchor: (
        if (($goal.metric_name // "") != "" and ($goal.current != null) and ($goal.target != null))
        then {
          metric_name: $goal.metric_name,
          metric_current: $goal.current,
          metric_target: $goal.target
        }
        else null
        end
      )
    }
' "$source_json" >"$tmpdir/desired_goals.jsonl"

jq -c --arg source "$source_json" "$clean_filter"'
  .goals
  | to_entries[]
  | . as $goal_entry
  | $goal_entry.value.paths[]?
  | . as $path
  | {
      name: $path.name,
      goal: $goal_entry.key,
      worker: ($path.worker // ""),
      worktree: ($path.worktree // ""),
      hypothesis: ($path.hypothesis // ""),
      falsification: ($path.falsification // ""),
      status: ($path.status // "active"),
      stall_counter: ($path.stall_counter // 0),
      last_metric_move_at: ($path.last_metric_move_at // null),
      substrate: ($path.substrate // null),
      notes: ($path.notes // null),
      metadata: (
        ($path
          | del(
              .name,
              .worker,
              .worktree,
              .substrate,
              .hypothesis,
              .falsification,
              .status,
              .stall_counter,
              .last_metric_move_at,
              .notes
            )
          | clean)
        + {source: $source}
      )
    }
' "$source_json" >"$tmpdir/desired_paths.jsonl"

goal_exists() {
  jq -e --arg key "$1" 'any(.[]; .goal_key == $key)' "$tmpdir/current_goals.json" >/dev/null
}

path_exists() {
  jq -e --arg name "$1" 'any(.[]; .path_name == $name)' "$tmpdir/current_paths.json" >/dev/null
}

path_differs() {
  local desired="$1"
  jq -e --argjson desired "$desired" '
    def metadata:
      (.metadata_json // {}
       | if type == "string" then (fromjson? // {}) else . end);

    first(.[] | select(.path_name == $desired.name)) as $current
    | if $current == null then true
      else
        {
          goal: $current.goal_key,
          worker: ($current.worker // ""),
          worktree: ($current.worktree // ""),
          hypothesis: ($current.hypothesis // ""),
          falsification: ($current.falsification // ""),
          status: ($current.status // "active"),
          stall_counter: ($current.stall_counter // 0),
          last_metric_move_at: ($current.last_metric_move_at // null),
          substrate: ($current.substrate // null),
          notes: ($current.notes // null),
          metadata: ($current | metadata)
        } != {
          goal: $desired.goal,
          worker: $desired.worker,
          worktree: $desired.worktree,
          hypothesis: $desired.hypothesis,
          falsification: $desired.falsification,
          status: $desired.status,
          stall_counter: $desired.stall_counter,
          last_metric_move_at: $desired.last_metric_move_at,
          substrate: $desired.substrate,
          notes: $desired.notes,
          metadata: $desired.metadata
        }
      end
  ' "$tmpdir/current_paths.json" >/dev/null
}

anchor_differs() {
  local goal_key="$1"
  local desired_anchor="$2"
  local current_anchor

  current_anchor="$("$harness" anchor-get --goal "$goal_key" --json 2>/dev/null || true)"
  jq -n -e --argjson desired "$desired_anchor" --argjson current "${current_anchor:-null}" '
    ($current == null)
    or (($current.metric_name // "") != $desired.metric_name)
    or (($current.metric_current // null) != $desired.metric_current)
    or (($current.metric_target // null) != $desired.metric_target)
  ' >/dev/null
}

json_field() {
  local json="$1"
  local filter="$2"
  jq -r "$filter" <<<"$json"
}

goals_added=0
anchors_updated=0
paths_added=0
paths_updated=0
paths_unchanged=0

while IFS= read -r goal; do
  key="$(json_field "$goal" '.key')"
  title="$(json_field "$goal" '.title')"
  detail="$(json_field "$goal" '.detail')"
  status="$(json_field "$goal" '.status')"
  priority="$(json_field "$goal" '.priority')"
  success_fact_key="$(json_field "$goal" '.success_fact_key')"
  metadata="$(jq -c '.metadata' <<<"$goal")"

  if ! goal_exists "$key"; then
    args=(goal-add "$key" "$title" --detail "$detail" --status "${status:-pending}" --priority "$priority" --metadata "$metadata")
    if [[ -n "$success_fact_key" ]]; then
      args+=(--success-fact-key "$success_fact_key")
    fi
    "$harness" "${args[@]}" >/dev/null
    goals_added=$((goals_added + 1))
  fi

  anchor="$(jq -c '.anchor' <<<"$goal")"
  if [[ "$anchor" != "null" ]] && anchor_differs "$key" "$anchor"; then
    "$harness" anchor-set \
      --goal "$key" \
      --current-understanding "$title" \
      --metric-name "$(json_field "$anchor" '.metric_name')" \
      --metric-current "$(json_field "$anchor" '.metric_current')" \
      --metric-target "$(json_field "$anchor" '.metric_target')" >/dev/null
    anchors_updated=$((anchors_updated + 1))
  fi
done <"$tmpdir/desired_goals.jsonl"

while IFS= read -r path; do
  name="$(json_field "$path" '.name')"
  goal="$(json_field "$path" '.goal')"
  worker="$(json_field "$path" '.worker')"
  worktree="$(json_field "$path" '.worktree')"
  hypothesis="$(json_field "$path" '.hypothesis')"
  falsification="$(json_field "$path" '.falsification')"
  status="$(json_field "$path" '.status')"
  stall_counter="$(json_field "$path" '.stall_counter')"
  metadata="$(jq -c '.metadata' <<<"$path")"

  if path_exists "$name"; then
    if ! path_differs "$path"; then
      paths_unchanged=$((paths_unchanged + 1))
      continue
    fi

    args=(
      path-set "$name"
      --goal "$goal"
      --worker "$worker"
      --worktree "$worktree"
      --hypothesis "$hypothesis"
      --falsification "$falsification"
      --status "$status"
      --stall-counter "$stall_counter"
      --metadata "$metadata"
    )
    if [[ "$(jq -r '.last_metric_move_at == null' <<<"$path")" == "true" ]]; then
      args+=(--clear-last-metric-move-at)
    else
      args+=(--last-metric-move-at "$(json_field "$path" '.last_metric_move_at')")
    fi
    if [[ "$(jq -r '.substrate == null' <<<"$path")" == "true" ]]; then
      args+=(--clear-substrate)
    else
      args+=(--substrate "$(json_field "$path" '.substrate')")
    fi
    if [[ "$(jq -r '.notes == null' <<<"$path")" == "true" ]]; then
      args+=(--clear-notes)
    else
      args+=(--notes "$(json_field "$path" '.notes')")
    fi
    "$harness" "${args[@]}" >/dev/null
    paths_updated=$((paths_updated + 1))
  else
    args=(
      path-add "$name"
      --goal "$goal"
      --worker "$worker"
      --worktree "$worktree"
      --hypothesis "$hypothesis"
      --falsification "$falsification"
      --status "$status"
      --stall-counter "$stall_counter"
      --metadata "$metadata"
    )
    if [[ "$(jq -r '.last_metric_move_at == null' <<<"$path")" != "true" ]]; then
      args+=(--last-metric-move-at "$(json_field "$path" '.last_metric_move_at')")
    fi
    if [[ "$(jq -r '.substrate == null' <<<"$path")" != "true" ]]; then
      args+=(--substrate "$(json_field "$path" '.substrate')")
    fi
    if [[ "$(jq -r '.notes == null' <<<"$path")" != "true" ]]; then
      args+=(--notes "$(json_field "$path" '.notes')")
    fi
    "$harness" "${args[@]}" >/dev/null
    paths_added=$((paths_added + 1))
  fi
done <"$tmpdir/desired_paths.jsonl"

canonical_goals="$(jq '.goals | length' "$source_json")"
canonical_paths="$(jq '[.goals | to_entries[] | .value.paths | length] | add' "$source_json")"

printf 'paths_to_db: canonical=%s goals/%s paths; goals added=%s; anchors updated=%s; paths added=%s updated=%s unchanged=%s\n' \
  "$canonical_goals" \
  "$canonical_paths" \
  "$goals_added" \
  "$anchors_updated" \
  "$paths_added" \
  "$paths_updated" \
  "$paths_unchanged"
