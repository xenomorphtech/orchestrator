# codex-app-server agent mode

A non-pane backend for Codex agents in the orchestrator harness, backed by `codex app-server` JSON-RPC over stdio. Pane mode (`biome_term`) crashes on long pasted prompts, queues paste content out of order, and loses prompts during init. This mode replaces stdin-driven panes with a structured JSON-RPC client and persists threads server-side so subprocess crashes do not lose conversation state.

## When to use which kind

- `biome_term`: pane is required for human-in-the-loop debugging in tmux/biome, or you want a visible scrollback. Existing biome_term agents are unchanged and continue to work.
- `codex_app_server`: default for new Codex agents going forward. Use it when the agent runs unattended, takes long prompts, or has been crashing in pane mode.

## CLI surface

Verified against `/home/sdancer/orchestrator/harness-rs/src/main.rs`.

### Register an agent

```
harness agent-add <name> --kind codex-app-server --workdir <abs path> [--default-task '<text>']
```

- `--kind` accepts `codex_app_server`, `codex-app-server`, `codex_app`, or `codex-app`. All normalize to `codex_app_server`.
- `--biome-pane-id` is NOT required for this kind. The harness writes a synthetic placeholder `codex-app-server:<name>` because the underlying column is non-null.
- `--metadata` is optional; if supplied it must be a JSON object. The harness merges `kind` into it before persisting. Re-running `agent-add` on an existing name is treated as upsert (this is what the resume-id persistence path relies on).

### Send a prompt

```
harness send <name> '<prompt>' [--wait | --no-wait] [--timeout <secs>] [--follow]
```

- Default `--wait` is `true`; the call blocks until `turn/completed`.
- `--no-wait` returns immediately after `turn/start`. Output is `{agent, thread_id, turn_id, events_jsonl, status:"started"}`.
- `--timeout` defaults to `300` seconds. On expiry the harness sends `turn/interrupt` and exits non-zero.
- `--follow` streams each event method/id to stderr while waiting.
- `--delay` is biome_term-only and ignored here.

The first send against a `codex_app_server` agent calls `thread/start`. Subsequent sends call `thread/resume` with the persisted `codex_thread_id`. If resume fails (server-side thread is gone) the harness falls back to `thread/start` and overwrites the stored id.

## What gets persisted, where

- `agents.metadata_json.kind = "codex_app_server"` — written on `agent-add` so `send` can dispatch correctly. No schema migration; this lives inside the existing `metadata_json` blob.
- `agents.metadata_json.codex_thread_id = "<uuid>"` — written after the first successful `thread/start` (or after a successful `thread/resume`), so the next send picks the same thread.
- Per-send event log: `analysis/codex-sessions/<agent>/<run_id>/events.jsonl` (`run_id` = UTC `%Y%m%dT%H%M%SZ`). Override the root with `HARNESS_CODEX_SESSION_ROOT`.
- Per-send Codex stderr: `analysis/codex-sessions/<agent>/<run_id>/codex.stderr.log`.
- Fact ledger: `codex_app_server_handshake_verified` was recorded as a fact on 2026-05-03 (status PASS).

## Lifetime semantics (D2: per-call subprocess + thread/resume)

- Each `harness send` spawns a fresh `codex app-server` subprocess. The subprocess runs `initialize` -> `thread/start` (first time) or `thread/resume` (subsequent) -> `turn/start` -> drains events to JSONL -> exits.
- The OS process is ephemeral. The THREAD lives server-side and is identified by `codex_thread_id`.
- A Codex crash mid-conversation does NOT lose the thread. Verified end-to-end on 2026-05-03 (sacrificial migration, Phase D): a third send arrived after the prior subprocess had fully exited and recalled the run-1 uuid from memory.
- There is no `thread/resumed` server-side notification. `thread/resume` returns metadata in the JSON-RPC response only. The stderr line `[codex-send] resumed thread <id>` is the canonical resume marker — grep that, not the event stream.

## Latency guidance

Observed in the sacrificial smoke (`analysis/codex-sessions/codex-app-server-smoke/`):

- Cold-start (first send / first send after subprocess exit): ~22s. About half is MCP server startup chatter (`mcpServer/startupStatus/updated` x8, `configWarning` x1).
- Resume (subsequent sends): ~3.5s.

Don't tune cold-start unless this becomes a hot path. If you ever migrate hot-loop agents (e.g. orchestrator pollers) we'd want to pre-warm.

## Approval / sandbox config

Production setting (in `cmd_codex_send` -> `thread/resume` and `Session::turn_start`):

- `approvalPolicy: "never"`
- `sandboxPolicy: { "type": "dangerFullAccess" }` for `turn/start`.
- `sandbox: "danger-full-access"` for `thread/start` (kebab-case enum, NOT the tagged-union object — they share a name but are different types).

This matches existing pane Codex agents that run with `--dangerously-bypass-approvals-and-sandbox`.

The auto-reply path for approvals exists and was verified end-to-end on 2026-05-03 against `approvalPolicy: "on-request"` (one `item/commandExecution/requestApproval` round trip, decision `acceptForSession`, server unblocked the call). Under `"never"` the path is bypassed by the server entirely (correct).

### Latent bug to fix before relying on a non-`never` policy

`Session::auto_reply` for `item/tool/requestUserInput` returns `{contentItems: [...]}`, but the schema requires `{answers: {<qid>: {answers: [<label>]}}}`. Currently dormant under `"never"`. Fix as a follow-up if a non-default policy is ever used in production.

## Migrating an existing biome_term agent

Hard rule: do NOT migrate `cert-rust-reimpl`, `cert-ptrace`, or `cert-re` while they are mid-task on a live RE campaign.

1. Confirm the agent is between tasks or has a checkpoint to roll back to.
2. Either spawn a parallel app-server-based copy in a separate worktree (preferred for risky migrations) or update the agent in-place. `agent_add` is upsert; running it on an existing name will rewrite `metadata_json.kind` — verify on a non-live agent first.
3. Run `harness agent-add <existing-name> --kind codex-app-server --workdir <path>`.
4. Send a smoke prompt that produces a checkable artifact (write a file, return a uuid).
5. Send a continuity prompt that requires recall from the first turn — verifies `thread/resume` actually carried history.
6. Compare artifacts to what the pane-based version was producing.

## Known issues / gotchas

- Bubblewrap warning fires on every `initialize`. It only matters under sandboxed modes; we use `dangerFullAccess` so it's ignored. Surface to the operator if it ever blocks.
- `availableDecisions` on approval requests is a UI hint, not enforcement. The harness sends `acceptForSession` regardless of what the server lists.
- `pgrep -f 'codex app-server'` matches its own zsh command line and produces a false positive. Use `pgrep -f '/codex .*app-server'` instead. A naive `pkill -f 'codex app-server'` is safe in practice — `cert-*` agents run `codex --dangerously-bypass-...` which does not contain `app-server` — but use the precise pattern anyway.
- `harness.toml` may point at `:3001` while only `:3000` is listening. Set `HARNESS_SERVER=http://127.0.0.1:3000` until that's fixed.

## Source-of-truth artifacts

- Pinned protocol JSON schema: `analysis/codex-protocol/codex-app-server-schema.json`
- TypeScript types: `analysis/codex-protocol/codex-app-server.d.ts`
- Symphony deep-read: `analysis/checkpoints/codex_app_server_step1_symphony_read_2026-05-03.json`
- Protocol pin checkpoint: `analysis/checkpoints/codex_app_server_step2_protocol_pinned_2026-05-03.json`
- Minimal client checkpoint: `analysis/checkpoints/codex_app_server_step3_minimal_client_2026-05-03.json`
- Approval handshake checkpoint: `analysis/checkpoints/codex_app_server_step3_approval_handshake_2026-05-03.json`
- Harness wiring checkpoint: `analysis/checkpoints/codex_app_server_step4_harness_wiring_2026-05-03.json`
- Sacrificial migration checkpoint: `analysis/checkpoints/codex_app_server_step5_sacrificial_migration_2026-05-03.json`
- Client module: `harness-rs/src/codex_app_server.rs`
- Send dispatch and CLI surface: `harness-rs/src/main.rs` (search `// ── codex_app_server agent kind plumbing`)
- Smoke driver: `harness-rs/examples/codex_app_server_smoke.rs`
- Approval probe: `harness-rs/examples/codex_app_server_approval_probe.rs`
- Sacrificial migration replay: `analysis/codex-sessions/codex-app-server-smoke/`

## Reproduction recipe

This reproduces the sacrificial migration (Phase B + Phase C, the parts that proved end-to-end PASS).

```
RND=5cc86135   # any random suffix
mkdir -p /tmp/codex-smoke-$RND/

HARNESS_SERVER=http://127.0.0.1:3000 \
  ./harness agent-add codex-app-server-smoke \
    --kind codex-app-server \
    --workdir /tmp/codex-smoke-$RND/ \
    --default-task 'Codex app-server smoke test agent.'

HARNESS_SERVER=http://127.0.0.1:3000 \
  ./harness send codex-app-server-smoke \
    'Create a file named hello.txt in the current directory with these exact two lines:
line one: hello from codex
line two: <random uuid you generate now>
Then print the file contents and tell me the uuid you generated. Do not write anything else.' \
    --wait --timeout 120

# Capture the uuid from the JSON output's last_assistant_text. Then:

HARNESS_SERVER=http://127.0.0.1:3000 \
  ./harness send codex-app-server-smoke \
    'What was the uuid you generated in your previous turn? Reply with just the uuid and nothing else. Do not look at the file.' \
    --wait --timeout 120
# Expect [codex-send] resumed thread <id> and the same uuid back.
```

A successful run prints `[codex-send] started new thread <id>` on the first send and `[codex-send] resumed thread <id>` on the second, with identical thread ids in both events.jsonl files under `analysis/codex-sessions/codex-app-server-smoke/`.
