# Harness

This module ports the SQLite state layer from [`harness.py`](/home/sdancer/orchestrator/harness.py) to SpacetimeDB while keeping the same core entities:

- `agents`
- `goals`
- `sub_goals`
- `facts`
- `observations`
- `artifacts`
- `actions`
- `events`

It also keeps the main state-management operations as reducers:

- `seed_agents`
- `bootstrap_known_goals`
- `agent_add`, `agent_remove`
- `goal_add`, `goal_update`, `goal_remove`
- `sub_goal_add`, `sub_goal_update`, `sub_goal_remove`
- `fact_set`
- `observation_add`
- `artifact_upsert`
- `action_queue`, `action_complete`, `queue_prompt`
- `agent_poll_record`
- `resolve_goal_states`, `resolve_sub_goal_states`, `resolve_active_sub_goals`
- `decide_actions`

Biome host integration is now in Rust procedures:

- `poll_agents_biome`
- `send_prompt_biome`
- `execute_pending_actions_biome`
- `run_once_biome`

## Build

```bash
cargo check --manifest-path harness-rs/Cargo.toml
spacetime build
```

## Local publish

```bash
spacetime start
spacetime publish orchestrator-harness
```

## Example calls

```bash
spacetime call orchestrator-harness seed_agents

spacetime call orchestrator-harness goal_add \
  '{"goal_key":"orchestrator.collect_more_sessions","title":"Collect more session captures","detail":"Gather additional live captures for validation","status":"pending","priority":15,"depends_on_goal_key":null,"success_fact_key":"agent_c.more_sessions_collected","metadata_json":"{}"}'

spacetime call orchestrator-harness sub_goal_add \
  '{"sub_goal_key":"agent_c.collect_more_sessions","goal_key":"orchestrator.collect_more_sessions","owner_agent":"agent_c","title":"Capture more sessions","detail":"","status":"pending","priority":10,"depends_on_sub_goal_key":null,"success_fact_key":"agent_c.more_sessions_collected","instruction_text":"Collect three fresh session captures and save the artifacts.","stuck_guidance_text":"If a dependency is unavailable, stub it and validate the path.","metadata_json":"{}"}'

spacetime call orchestrator-harness fact_set \
  '{"fact_key":"oracle.tested","value_json":"true","confidence":1.0,"source_type":"manual","source_ref":null,"metadata_json":"{}"}'

spacetime call orchestrator-harness resolve_active_sub_goals

spacetime call orchestrator-harness run_once_biome \
  'http://localhost:3000' \
  20 \
  true \
  10
```

## Notes

This Rust module now owns the biome polling and biome prompt-dispatch path. The remaining host-side gap is filesystem walking and external command indexing; those can be ported next if you want the artifact scan/index loop inside the module too.
