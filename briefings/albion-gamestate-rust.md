# albion-gamestate-rust — Rust port of gamestate service + dashboard tab redesign

## Role & workdir
Codex worker (codex_app_server) at `/home/sdancer/albion-gamestate-rust` (git worktree, branch `albion-gamestate-rust`, off vastai-albion main commit 14b7ace).

## Goal
Two coupled deliverables, in order:

1. **Port `gamestate_service.py` (1199 lines, at `albion-gamestate-service-snapshot/gamestate_service.py`) to a Rust workspace** of multiple crates. Anything that does NOT strictly need Python goes to Rust. You decide what stays Python (likely nothing — the existing Python is pure stdlib + JSONL tailing + Photon decode + HTTP).
2. **Redesign the embedded dashboard** with these tabs: **Combat Log · Packets Log · HTN · Entities Around · Inventory · Quests**. Focus on visual design and information architecture, not just slapping divs.

## Verification artifacts (the ONLY things that mean "done")

**For (1) — Rust port:**
- A `cargo run -p <bin>` invocation that drop-in replaces `python3 gamestate_service.py --bind 0.0.0.0 --port 8765 --source frida --session-dir /var/log/albion-frida-sessions --session-glob 'session-*.jsonl' --memory-mb 1024`.
- Side-by-side diff: spin the Rust binary at port 8766 and the live Python at 8765 (already running on vast.ai 37014838), poll `/state` for ~60s, assert the entity counts and self/zone fields agree within tolerance. Commit a short comparison script in `tests/` plus the diff log proving parity.
- Resident set < 200 MB under the same session-log workload (Python is currently ~36 MB — Rust should easily beat or match).

**For (2) — Dashboard tabs:**
- Live screenshot via the deployed instance at https://albion.orch.run/ (or a sibling tunnel if you deploy alt-port). Each tab navigates and renders. Tabs whose data sources don't exist yet show a clean "no data yet" empty state — not a broken UI.
- At least one tab (your choice — Entities Around is the easiest since `/state` already provides it) is fully wired with live data.

## Operating doctrine (macromanage)
- **You choose every tactic**: workspace layout, crate names, async runtime (tokio/async-std/blocking), HTTP framework (axum/hyper/tiny_http/…), JSONL parser, packet decoder reuse vs. fresh, dashboard tooling (vanilla JS / lit / htmx / pick one and justify in a one-paragraph DESIGN.md). The only hard constraint: **multiple crates** in a workspace.
- **Reuse `photon-decoder-rs`** if it covers the Photon parsing you need; if it's missing pieces, extend it rather than re-implement.
- **Iterate to parity, not perfection.** Ship the port at functional parity first, then the dashboard rewrite. Don't gold-plate.
- **License hygiene.** No GPL/AGPL community decoder code copied verbatim — read as reference, write your own.

## Substrate facts
- vast.ai SSH: `ssh -p 14838 -i /home/sdancer/.ssh/id_ed25519 root@ssh8.vast.ai`
- Live Python service: `:8765` on the box (PID 24844, ~36 MB), tailing `/var/log/albion-frida-sessions/session-*.jsonl`. Currently active — there's live gameplay landing in session-3 right now.
- Ingester writes JSONL frames at `/var/log/albion-frida-sessions/session-20260521-020029-00003.jsonl` (~28 MB, ~180 KB/min).
- Dashboard tunnel: https://albion.orch.run/ → `:8765` on vast.ai.
- vast.ai container has **no systemd** — long-running daemons go via `nohup setsid` + PID file.
- Cloudflared token: `/home/sdancer/.cloudflared_albion_token` (don't commit).
- Albion creds: `/home/albion/.albion_credentials.txt` on remote (don't commit).

## Anti-patterns (don't)
- Don't draft a 500-line DESIGN.md before writing any Rust. One short DESIGN.md (≤100 lines) is enough — bias to code.
- Don't redirect production `:8765` to the Rust binary until parity is proven and the user explicitly says so. Deploy on an alt port until then.
- Don't ask the orchestrator which crate names / frameworks to use — pick and go.
- Don't break the existing live Python service while porting.

## Workspace seeds (suggestions — you can override)
- `crates/albion-gamestate-store` — entity store / decay / TTL
- `crates/albion-photon-decode` — wrapper around / extension of `photon-decoder-rs`
- `crates/albion-session-tail` — JSONL tailer with rotation handling
- `crates/albion-gamestate-http` — axum or hyper handlers, dashboard static asset
- `bin/albion-gamestate` — binary wiring it all together

Or any other carving you prefer. Just keep it ≥3 crates.

## Tab hints (worker resolves the rest)
- **Combat Log** — needs combat events from Photon packets. Worker decides: surface event types in the store, expose `/combat_log?since=…`.
- **Packets Log** — raw Photon packets seen recently (capped buffer). Tail of decoded frames with op-code names. Useful for live RE.
- **HTN** — Hierarchical Task Network bot behavior surface; for now a stub tab with a placeholder ("HTN planner not yet wired") is fine, but design the panel layout to accept later wiring.
- **Entities Around** — already in `/state.players / mounts / buildings`. Render a sortable distance-from-self table.
- **Inventory** — Photon packet type unknown to current Python; stub OK, design the grid layout properly.
- **Quests** — same; stub OK.

You don't have to wire all six tabs to live data. You DO have to design the IA so they all coexist cleanly and the empty states don't look like bugs.

## Reporting cadence
Append milestone events (not heartbeats) to `/home/sdancer/orchestrator/analysis/talk_channels/vastai-albion-web.jsonl` as
`{"ts":"<utc>","from":"albion-gamestate-rust","text":"<event>"}`.

Report-worthy events: parity reached, dashboard tabs live, deploy successful.

## Memory pointers
- `[[macromanage-workers]]`
- `[[worker-artifact-isolation]]` — vast.ai daemons survive turn-end via nohup setsid (no Hetzner cgroup death)
- `[[albion-substrate]]` — vast.ai is the Albion substrate (not RK3588)
