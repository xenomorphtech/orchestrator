# codex-app-server — Integrate Codex app-server JSON-RPC into the orchestrator

## Role & workdir
You design and implement a **terminal-free Codex worker mode** for the orchestration harness, so we can spawn / steer Codex agents without biome_term panes (which have been crashing, hanging on pasted-content prompts, and dropping context). Workdir: `/home/sdancer/orchestrator/`.

## Goal / sub-goal
- Goal key: `codex_app_server_integration`
- Success fact: `codex_app_server_integration_complete`
- Concrete done: harness can launch a Codex agent that exposes JSON-RPC over stdio (via `codex app-server`), drive its turns, capture tool-call streams, and surface them through the same `agent` interface that biome_term panes use today. At least ONE existing agent (e.g. `cert-rust-reimpl`) must be runnable in the new mode and produce the same artifacts as the pane-based version.

## Why this matters
- biome_term panes have failed three times in the last hour for `cert-ptrace` (Codex crashes, paste-mode queue confusion, lost prompts requiring manual Enter).
- Pane mode is fragile: aeon MCP timeouts → pane death; long pasted prompts get queued instead of submitted; trust prompts on first launch require human keystrokes.
- An app-server-based worker is durable: clean JSON in, clean JSON out, no terminal state.

## Reference implementation
- **Symphony** (OpenAI's open-source orchestrator): https://github.com/openai/symphony
- Key file: `elixir/lib/symphony_elixir/codex/app_server.ex` (already validated). Read this carefully — it's the canonical client.
- Protocol summary (extracted from Symphony):
  - Codex is launched via `codex app-server` (NOT `mcp-server`; both exist but app-server is the right one).
  - Communication is JSON-RPC 2.0, line-delimited, over stdio.
  - Initial handshake: send `{"method":"initialize","id":1,"params":{"capabilities":{"experimentalApi":true},"clientInfo":{"name":"...","version":"..."}}}`, then `{"method":"initialized","params":{}}`.
  - Start a thread: `{"method":"thread/start","id":2,"params":{"approvalPolicy":"never","sandbox":"...","cwd":"/path/to/workspace","dynamicTools":[...]}}` → returns `{"thread":{"id":"<thread_id>"}}`.
  - Start a turn: `{"method":"turn/start","id":3,"params":{"threadId":"<thread_id>","input":[{"type":"text","text":"<prompt>"}],"cwd":"...","title":"...","approvalPolicy":"never","sandboxPolicy":{...}}}`.
  - Stream events: `turn/completed`, `turn/failed`, `turn/cancelled`, plus tool-call events with method `<name>` (assistant message, tool-call request, etc.).
- Useful CLI helpers:
  - `codex app-server generate-json-schema` — emits the protocol schema (use it!).
  - `codex app-server generate-ts` — emits TS types if you want them.
  - `codex app-server proxy` — proxies stdio to a running app-server (useful for tests).

## Approach (suggested, refine in plan)

1. **Read the Symphony Elixir file end-to-end** (don't just skim). Understand the message envelope, approval flow, tool-input handling, dynamic tools, sandbox policy.
2. **Pin the protocol**: run `codex app-server generate-json-schema > codex-app-server-schema.json` in your worktree. Use it as the source of truth, not assumptions.
3. **Pick the implementation language** to match the existing harness (`/home/sdancer/orchestrator/harness` is a Rust binary). Adding a Rust module is the lowest-friction path; a Python sidecar is acceptable if it ships faster but be explicit about the tradeoff.
4. **Build a minimal client**: spawn `codex app-server` as a child, perform the initialize/thread/start handshake, start a turn with a single text input, stream the JSON-RPC events to a file or to harness episodes.
5. **Wire into harness**: add an agent kind alongside the current biome_term kind. `harness agent-add my-agent --kind codex-app-server --workdir <path> --default-task "..."` should spawn the subprocess on demand. `harness send my-agent "<prompt>"` should start a new turn.
6. **Migration validation**: pick `cert-rust-reimpl` (currently a Codex agent on a fragile pane), spawn a parallel app-server-based copy in a separate worktree, verify it can run a non-trivial Rust task end-to-end (compile, test, write checkpoint).
7. **Document** in `/home/sdancer/orchestrator/AGENTS.md` (or a new `codex-app-server-mode.md`) so future cycles can use it without re-deriving.

## Constraints & gotchas
- `/home/sdancer/orchestrator/harness` is the Rust harness CLI — work in a sibling repo or fresh module; don't break existing biome_term mode.
- The harness DB is SpacetimeDB; new agent kind probably needs a column on the `agent` table. Check existing schema before adding migrations.
- Authorization: codex app-server inherits `~/.codex/config.toml`. Don't reset auth, and don't commit any credentials.
- Sandbox policy: Symphony uses `approvalPolicy="never"` plus a sandbox profile. For our use, mirror what existing pane Codex agents have (i.e. unrestricted / `--dangerously-bypass-approvals-and-sandbox` equivalent, since that's what current pane mode gives them).
- Don't move `cert-ptrace` / `cert-rust-reimpl` / `cert-re` over until you've validated end-to-end on a sacrificial agent — those are mid-task on a live RE campaign and disrupting them costs us.

## Reporting cadence
Write a status checkpoint after each milestone (schema dumped / first handshake works / first turn round-trip / harness integration / first migrated agent). Save under `analysis/checkpoints/codex_app_server_<step>_2026-05-03.json` or similar in your workdir.

## Relevant files / references
- Symphony reference: https://github.com/openai/symphony/blob/main/elixir/lib/symphony_elixir/codex/app_server.ex
- Symphony tree: https://github.com/openai/symphony/tree/main/elixir/lib/symphony_elixir/codex/ (sibling files: `dynamic_tool.ex`, `path_safety.ex`, `ssh.ex`, etc. — read what's relevant)
- Codex CLI help: `codex app-server --help`, `codex app-server generate-json-schema`
- Existing harness Rust source (probably under `/home/sdancer/orchestrator/harness-src/` or similar — locate it)
- Current biome_term integration for reference: `/home/sdancer/orchestrator/.claude/commands/orchestrate.md` describes the existing pane-based agent flow
