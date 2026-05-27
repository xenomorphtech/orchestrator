# Orchestrator

Harness infrastructure for running multiple coordinated coding-agent workers
against a project: scheduler, dashboard, biome terminal layout, talk channels,
and the shared `orchestrate` skill.

## Scope

This repo is for the **orchestration harness itself** — code that runs and
coordinates workers, not the things workers happen to be working on.

### Do not contaminate this repo with goal-specific data

Workspace artifacts from any individual campaign — captures, dumps, decoded
streams, per-target wikis, per-target Rust crates, debugging notes, agent
briefings, verdicts, hypotheses, traces — belong in **their own repos**, not
here. The harness must remain reusable across goals.

Examples of work that lives elsewhere:

- `~/nmss-wiki/` — NMSS cert / harness research
- `briefings/` — per-agent working notes (local-only, gitignored)
- `analysis/` — per-campaign verdicts / hypotheses (local-only, gitignored)
- `minimap-sniffer/`, `darkdecember/` — dark december campaign artifacts
  (local-only, gitignored)

If you find yourself adding a directory with a campaign name or target name to
the root of this repo, that's the signal: it belongs somewhere else.

## What lives here

- `harness-rs/` — Rust harness (scheduler, services, agent reducer)
- `web/` — dashboard (auth gate, talk channels, screenshots)
- `orchestrate.md`, `orchestrate-scheduler.sh`, `send.sh` — orchestration tooling
- `.claude/` skills and settings shared across workers
