# aeon-memory-investigation — root-cause aeon MCP server memory exhaustion

## Role & workdir
Performance/memory analyst on the **aeon MCP server** that the magic32-disasm worker uses for structured binary analysis (datalog queries, get_function_at, scan_pointers, etc). Workdir: `/home/sdancer/nmss-emu-aeon-memory-investigation` (git worktree of nmss-emu, branch `aeon-memory-investigation`).

## Current goal / sub-goal
Diagnose why repeated orchestrator-Claude-Code sessions on this box keep crashing. **Working hypothesis:** the aeon MCP server, when driven by long codex_app_server turns (especially the magic32-disasm worker doing datalog over the 158 MB libUnreal.so), balloons memory and the kernel OOM-killer reaps the entire user-1000.slice cgroup — taking the orchestrator, harness send wrappers, and even SpacetimeDB with it.

## Success criteria
Closing fact: `aeon_memory_exhaustion_diagnosed_2026_05_15`. Concrete deliverable:
- **Quantified memory footprint** of aeon under a representative magic32-disasm workload (RSS over time, peak RSS, growth rate).
- **Root-cause classification**: (a) genuine leak in aeon, (b) cache that never evicts, (c) per-query memory not freed, (d) external pressure from libUnreal.so's own size, or (e) the user-slice cgroup itself being too small.
- **Concrete fix proposals** with tradeoffs — examples: cgroup MemoryMax on aeon, periodic aeon restart, smaller analysis windows, or moving aeon under root systemd alongside the harness workers.

## Progress so far (cross-pollination from magic32-disasm path)

The magic32-disasm worker pivoted to aeon MCP usage during turn 2c (around 2026-05-14T22:51Z). Tools observed in its event stream:
- `get_function_at` — disasm fetch via aeon
- `get_data` — data section access
- `scan_pointers` — pointer-table scans
- `get_function_pointers`
- `search_analysis_names`
- `execute_datalog` — datalog queries against the binary's analysis state

After this pivot, raw codex event rates dropped but agentMessage delta counts stayed high (deep MCP-driven reasoning). Around the same time, the orchestrator Claude-Code session experienced repeated crashes. **Correlation is suggestive but not yet causal.**

Substrate evidence we already have:
- `libUnreal.so` size: **158 MB** (aarch64 ELF at `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so`).
- Box has **124 GB RAM, 3 GB swap** (see prior orchestrator episodes).
- biome-term-server lives in **system.slice** (root systemd-managed). orchestrator Claude-Code session lives in **user-1000.slice** (or whatever the runtime puts it in).
- Workers spawned via `setsid nohup` reparent to PID 1 but **stay in the user slice's cgroup** — when that slice is reaped, all workers die.

## Next 2–3 concrete tasks

1. **Find the aeon MCP server.** Where does codex actually run it? It's an MCP server spec — look at codex's config (`~/.config/codex/`), startup logs, or the codex events.jsonl streams for the codex agents (`/home/sdancer/orchestrator/analysis/codex-sessions/magic32-disasm/*/events.jsonl` — grep for `aeon` + the `mcpServer/startupStatus/updated` events). Identify the binary path, command line, and whether it runs as a codex subprocess or a long-lived process. Record findings in `<workdir>/analysis/aeon_topology.md`.

2. **Profile memory under load.**
   - If aeon is a subprocess of codex: it dies with the codex turn, so per-turn peak RSS is the relevant number. Instrument with `/usr/bin/time -v <codex turn>` if possible, or `ps -o rss --no-headers <pid>` polling.
   - If aeon is a long-lived daemon: poll `/proc/<pid>/status` (VmRSS, VmPeak, VmHWM) every 5s during a representative codex turn.
   - Use the **already-archived event streams** at `/home/sdancer/orchestrator/analysis/codex-sessions/magic32-disasm/20260513T223746Z/events.jsonl` (post-crash turn 2c, the one that triggered the suspected memory issue) to see how many aeon mcpToolCalls fired and what their cadence was — that's a proxy for memory pressure if you can correlate with `dmesg`/`journalctl` timestamps.
   - Cross-reference: was there an OOM-kill event in `journalctl -k` around the crash timestamps? Use `sudo journalctl -k --since "2026-05-13 08:00" --until "2026-05-13 09:00" | rg -i 'oom|kill|memory'` and similar.

3. **Propose a fix.** Based on the root cause, write `<workdir>/analysis/aeon_memory_close.md` with:
   - The exact mechanism by which aeon (or whatever else) drives the user slice into OOM.
   - 2–3 concrete mitigation options ranked by ease + effectiveness. Examples:
     - **Move aeon under root systemd** with a hard MemoryMax that triggers per-process OOM (kills aeon only, not the user slice).
     - **Periodic aeon restart** after N turns or N MB ingested.
     - **Smaller query windows** in the worker's briefing — explicit instruction to query libUnreal.so by section/range rather than whole-binary datalog passes.
     - **DefaultMemoryAccounting=yes + per-cgroup limits** in `/etc/systemd/user.conf` or a drop-in.

## Constraints & gotchas

- **Read-only diagnosis** unless the user explicitly authorizes a system change. You can install diagnostic tools (perf, smem, bcc tools) via apt but pause for confirmation before that.
- Do NOT run aeon directly with rude flags. The goal is to understand its behavior, not to break it.
- The orchestrator has authorization to install systemd units (just did so for `harness-worker@.service`) so a similar pattern for aeon is on the table — propose it explicitly in your closing artifact.
- Memory snapshots in `/proc/<pid>/smaps_rollup` are cheaper than full `smaps` if aeon is a large process.
- The `worker_isolation_setsid_insufficient_2026_05_14` fact and the `magic32-disasm` analysis dir document prior context.

## Relevant files / references

- `~/.config/codex/` — codex CLI config (look for MCP server registrations).
- `/home/sdancer/orchestrator/analysis/codex-sessions/magic32-disasm/*/events.jsonl` — event streams (look for `mcpServer/startupStatus/updated` early in each turn; `mcpToolCall` items with `server=aeon`).
- `/home/sdancer/orchestrator/analysis/codex-sessions/magic32-disasm/20260513T223746Z/events.jsonl` — the canonical crash-precipitating turn.
- `/home/sdancer/.local/share/spacetime/data/logs/spacetime-standalone.log` — SpacetimeDB log; check for restart timestamps that correlate with orchestrator crashes.
- `journalctl -k` (with sudo) — kernel logs for OOM events.
- `/proc/<pid>/status`, `/proc/<pid>/smaps_rollup`, `/proc/<pid>/oom_score`.
- `systemctl status user.slice user@1000.service` — current user-slice resource state.
- `/etc/systemd/system/harness-worker@.service` — the worker isolation unit just installed; reference architecture for any aeon proposal.

## Falsification

This investigation closes (one way or another) when:
- A **specific mechanism** is named with quantitative evidence (RSS peak, OOM-kill log entry, growth pattern), AND
- A **fix proposal** is written with tradeoffs.

If 3 cycles produce no quantitative evidence of memory pressure from aeon specifically, retire this path with falsified conclusion: "orchestrator crashes have a different root cause — recommend further investigation [direction]."
