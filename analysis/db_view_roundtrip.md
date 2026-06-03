# DB View Round-Trip

Date: 2026-06-03

Command run:

```sh
./analysis/paths_from_db.sh > /tmp/paths_from_db.json
```

Read surface inspected:

- `./harness goals` lists 6 goals from the central DB.
- `./harness goal-tree --help` fails in this worktree: `goal-tree` is not registered in the current `./harness` binary.
- `./harness path-list --help` exposes filters `--goal`, `--sub-goal`, `--status`, and `--json`.
- `./harness path-list --json` is the path read surface. It returns `goal_key`, `path_name`, `hypothesis`, `falsification`, `status`, `stall_counter`, `last_metric_move_at`, `worker`, `worktree`, `substrate`, `notes`, and `metadata_json`.
- `./harness anchor-get --goal <goal> --json` returns goal metric/current/target where an anchor exists.

## Result

Not clean.

The generator reproduced a DB-derived JSON view with 6 goals and 18 paths. The canonical `/home/sdancer/orchestrator/analysis/paths.json` has 5 goals and 27 paths.

Common paths round-trip on `hypothesis`, `falsification`, `stall_counter`, and path identity. One common path has a status mismatch:

| Goal | Path | Canonical status | DB status |
| --- | --- | --- | --- |
| `dgm_self_improving_orchestrator` | `rust-dashboard-1to1` | `dev-done-ops-residual` | `active` |

Paths present in canonical `paths.json` but missing from the DB:

| Goal | Path |
| --- | --- |
| `dgm_self_improving_orchestrator` | `albion-dash-svc` |
| `dgm_self_improving_orchestrator` | `albion-orch-restore` |
| `dgm_self_improving_orchestrator` | `deploy-harden-reprovision` |
| `dgm_self_improving_orchestrator` | `e2e-verify-prune` |
| `harness_db_first_coherence` | `box-loop-db-first` |
| `harness_db_first_coherence` | `central-paths-db-first` |
| `harness_db_first_coherence` | `dbview` |
| `harness_db_first_coherence` | `interface-a-harness-cli` |
| `harness_db_first_coherence` | `interface-b-dashboard-http` |

Goals/anchors:

| Goal | Finding |
| --- | --- |
| `albion_gamedata_corpus` | Present in DB, absent from canonical `paths.json`; no paths; no anchor current/target. |
| `harness_db_first_coherence` | Present in DB and canonical, but DB has 0 paths and no anchor current/target. Canonical has 5 paths and current/target `4/5`. |
| `albion_tutorial_bot` | DB goal status is `active`; canonical has no top-level `status` field, only status notes. |

## Verdict

`analysis/paths_from_db.sh` proves the available DB read surfaces can reconstruct the rows that are actually in the DB, including the required path fields. It does not prove DB-as-source for the full portfolio yet because the central DB is missing 9 canonical paths, has one stale path status, and lacks the `harness_db_first_coherence` goal anchor.

Fact `harness_db_view_roundtrip_proven` was not set.

## Re-sync (dbsync c863)

Date: 2026-06-03

Commands run:

```sh
./analysis/paths_to_db.sh
./analysis/paths_to_db.sh
./analysis/paths_from_db.sh > /tmp/paths_rt.json
/home/sdancer/orchestrator/harness path-list --json | python3 -c "import json,sys;print(len(json.load(sys.stdin)))"
```

Result: clean for the canonical path set.

- First sync exited 0: canonical source had 5 goals / 28 paths; goals added `0`; anchors updated `1`; paths added `10`; paths updated `2`; paths unchanged `16`.
- Second sync exited 0 and was idempotent: goals added `0`; anchors updated `0`; paths added `0`; paths updated `0`; paths unchanged `28`.
- The requested `path-list` count command printed `28`. The earlier 27-path figure is stale; `/home/sdancer/orchestrator/analysis/paths.json` now contains 28 paths.
- `paths_from_db` read-back has 28 paths under the 5 canonical goals.
- Goal set caveat: additive-only sync leaves the pre-existing DB-only goal `albion_gamedata_corpus` in place, with no paths.
- Path set comparison under canonical goals is clean: no missing paths, no DB-only paths, and no mismatches on `hypothesis`, `falsification`, or `status`.
- Remaining goal-field caveat: `albion_tutorial_bot` has DB goal status `active`; canonical `paths.json` has no top-level status for that goal, only status notes.
