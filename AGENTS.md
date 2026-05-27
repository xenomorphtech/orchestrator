# AGENTS.md

Orientation for AI agents working in this orchestrator repo.

This repo runs a multi-agent orchestration loop. The orchestrator harness lives in `harness-rs/` (Rust binary, entry `harness-rs/src/main.rs`). Per-agent role briefings live in `briefings/`. Campaign output (checkpoints, codex sessions, protocol notes) lives in `analysis/`. Top-level Markdown docs (`orchestrate.md`, `codex-app-server-mode.md`, `design.md`) describe the model, the new agent kind, and the original design respectively.

This file is an index. It does not duplicate the operator docs — it points at them.

## Agent kinds

Two agent backends are registered with the harness today.

### biome_term

The original Codex agent kind. Each agent owns a tmux/biome pane; the pane is the I/O surface. Spawn-time the pane is expected to already exist (it is created out-of-band or by the orchestrator skill). `harness send` writes the prompt as a paste into the pane and returns. Idle/active state is inferred from pane scrollback. Pane mode is fragile in practice: long pasted prompts crash Codex, paste queue can reorder, init prompts get lost.

For the broader scheduler model (5-minute control cycles, briefings, worktree-per-path), see `orchestrate.md`.

### codex_app_server

Non-pane Codex agent backed by `codex app-server` JSON-RPC over stdio. Default for new Codex agents going forward. Each `harness send` spawns a fresh subprocess that runs `initialize` -> `thread/start` or `thread/resume` -> `turn/start` and drains events to JSONL. The thread lives server-side, identified by `codex_thread_id` persisted in `agents.metadata_json`. A subprocess crash does not lose conversation state.

Full operator doc: `codex-app-server-mode.md`. Read it before adding, sending to, or debugging a `codex_app_server` agent.

## Conventions

### Briefings

Every agent has a Markdown brief at `briefings/<agent-name>.md`. The brief sets role, workdir, goal key, success fact, references, and approach. The orchestrator skill writes/refreshes a brief before spawning or restarting an agent, and the worker's first prompt instructs it to read that file. For an example template see `briefings/codex-app-server.md`.

### Checkpoints

Every milestone writes a JSON checkpoint to `analysis/checkpoints/<campaign>_<step>_<YYYY-MM-DD>.json`. Naming is consistent across campaigns; this is how status, fact ledgers, and hand-offs reconstruct what was verified and when. Existing examples include `codex_app_server_step1_symphony_read_2026-05-03.json` through `codex_app_server_step6_documentation_2026-05-03.json`.

### Fact ledger

Cross-campaign signals (verified milestones, ready-for-migration flags, kill-switches) are recorded with `harness fact-set` and inspected with `harness facts`. Facts are the right primitive for "this milestone is verified" or "this gate is open"; per-conversation notes belong in checkpoints, not the ledger. Concrete examples set 2026-05-03:

- `codex_app_server_handshake_verified` — JSON-RPC handshake passed end-to-end.
- `codex_app_server_ready_for_real_agent_migration` — the new kind is cleared for migrating a real agent off panes.

## Useful CLI primitives

The harness binary lives at `/home/sdancer/orchestrator/harness` (Rust, built from `harness-rs/`).

- `harness agents` — list all agents.
- `harness agent-get <name> [--json]` — single agent's full record (added 2026-05-03 for clean diffing).
- `harness agent-add <name> --kind {biome_term|codex_app_server} --workdir <path>` — register or upsert.
- `harness send <name> '<prompt>' [--wait|--no-wait] [--timeout SECS] [--follow]` — drive a turn.
- `harness facts` — list the fact ledger.
- `harness fact-set <key> <value> [--source-type ...] [--source-ref ...] [--metadata ...]` — record a fact.

Endpoint: `HARNESS_SERVER=http://127.0.0.1:3000` is the working endpoint in the current environment. `harness.toml` may point at `:3001` — that is a known pre-existing infra mismatch, not something to "fix" in passing.

## See also

- `codex-app-server-mode.md` — codex app-server agent kind, full operator doc.
- `orchestrate.md` — orchestrator skill / control-cycle model / scheduler.
- `design.md` — original design notes.
- `briefings/` — per-agent briefings; look here when you are assigned to an agent name.
- `analysis/checkpoints/` — campaign checkpoints (naming convention above).
- `analysis/codex-sessions/` — per-send Codex event logs and stderr for `codex_app_server` agents.
- `harness-rs/src/main.rs` — orchestrator harness entry point.
- `harness-rs/src/codex_app_server.rs` — JSON-RPC client implementation for the `codex_app_server` kind.
