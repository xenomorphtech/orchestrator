# Parallelism — substrate sharding & fan-out

Serial path management (one worker per path, divergence on stall) is the default and is correct while a capability is still being *discovered*. But once a capability is **reproducible**, the remaining work is frequently embarrassingly parallel, and continuing single-threaded is the *circling* failure mode wearing a disguise.

## Triggers (run this whenever ANY is true)

- A path just produced a reproducible unit-of-work generator (codified skill/script + verified ≥1 success) — e.g. "account → in-game character" is now a one-command recipe.
- An active goal's remaining work is a *loop over independent items* (N accounts, N zones, N files, N challenges) rather than a single unsolved unknown.
- A goal has sat at parallelism=1 for ≥2 ticks while its work is independent and substrate is free.

## The fan-out decision (4 questions)

1. **Decomposable?** Can the remaining work split into N units with no cross-unit ordering dependency? (If units must run in sequence, parallelism=1 is correct — stop here.)
2. **Substrate per unit?** What *exclusive* substrate does one unit need (display, client, account, device, container)?
3. **Shardable + capacity?** Can K independent instances be provisioned now, and does the box have the headroom (GPU/CPU/RAM per client, display count, account supply)? K = min(units, instances-provisionable, resource-cap). **Declare K and the binding constraint** in the cycle report.
4. **Worth it?** Fan-out cost (provisioning + K workers' tokens) < serial cost of the remaining units. For long loops it almost always is.

## Execution (per shard)

- Provision substrate instance `i` (e.g. `Xtigervnc :N+i` + its own client/account/container) — script this; it is the reproducible generator's job.
- One **worktree per shard** and one **worker per shard** (the one-worker-per-path invariant applies per shard). Name shards `<goal>-shard-<i>`.
- Each shard's briefing points at *its* substrate instance explicitly (display number, account, device serial) — shards must never address each other's substrate.
- Record each shard as its own DB `paths` row under the goal; the goal's metric becomes `Σ shard progress` (e.g. `accounts_through_tutorial / K`).

## Substrate-sharding invariants

- **One exclusive substrate instance per worker.** Two workers on one display/device/client is corruption, not parallelism — the higher-numbered shard re-provisions its own instance or is retired.
- **Width is capped by real capacity, declared, and never silently exceeded.** If only 1 instance fits, the goal is parallelism-capped at 1 — state it; don't pretend otherwise.
- **Shared *read-only* substrate** (a pcap, a model, a corpus) is NOT exclusive — fan out freely; only mutating/stateful substrate forces sharding.

## Don't fan out when

- The unit work is still unproven (discover serially first — fan-out multiplies a broken recipe).
- Units are sequentially dependent.
- Substrate is a true singleton.

This is the standing parallelism doctrine; the K_aux=12 backlog-benchmark and the Parallelization-pass DECIDE rule are its triggers.
