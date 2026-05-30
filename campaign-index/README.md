# NMSS Cert RE Campaign — Index

This tree is the canonical state for the NMSS cert reverse-engineering campaign. Worker briefings reference this; they do NOT duplicate. When a finding lands, update the relevant leaf here.

**Working repo**: `/home/sdancer/nmss-emu`
**Goal**: capture per-challenge `nmssNativeGetCertValue` outputs from a live authenticated `com.netmarble.thered` session, then validate the Phase 1 algorithm reproducer.

## Operating principle: blockers don't exist
Every "wall" is a path that's currently closed under one set of assumptions. The campaign treats walls as **branch points for thought experiments**, not stop signs. When a worker reports a block:
1. Document the wall in `walls/` (what the symptom is, what conditions produced it).
2. Generate 3–5 lateral approaches in `lateral/<wall-slug>.md` (different tooling, different mechanism, different layer of the stack).
3. Pick the highest-value lateral and try it. Standing down is never the answer.

The fact-checker agent enforces this: every block-claim verdict it returns includes a `LATERAL EXPERIMENTS` section.

## How to use this index
1. Need current device/session state? → `state/current-session.md`
2. About to try a probe path? → check `walls/` first to avoid retrying a confirmed dead end
3. Looking for a tool already built? → `tools/built/INDEX.md` or `tools/on-device/INDEX.md`
4. Need algorithmic detail? → `findings/algorithm/`
5. Need to know what's blocking right now? → `state/blockers.md`

## Tree

### state/ — live operational state (rewrite as it changes)
- [current-session.md](state/current-session.md) — device pids, launch method, screen state
- [blockers.md](state/blockers.md) — what's stopping progress right now

### tools/built/ — host-side tools we've built or adapted
- [INDEX.md](tools/built/INDEX.md) — list with one-liner each
- (leaf files added as tools are built)

### tools/on-device/ — binaries/scripts shipped on /data/local/tmp
- [INDEX.md](tools/on-device/INDEX.md) — list with one-liner each
- [cert-client.md](tools/on-device/cert-client.md) — reverse-engineered, dead end
- [nmss-probe.md](tools/on-device/nmss-probe.md) — 16-payload probe, dead end
- [xerda-server.md](tools/on-device/xerda-server.md) — modified Frida server
- [arm-ptrace-helpers.md](tools/on-device/arm-ptrace-helpers.md) — for snapshot replay only

### findings/ — confirmed campaign facts
- [INDEX.md](findings/INDEX.md) — list with one-liner each
- algorithm/
  - [ed25519-sha512.md](findings/algorithm/ed25519-sha512.md) — crypto identification
  - [serializer-bridge.md](findings/algorithm/serializer-bridge.md) — 0x2d7284 spec
  - [writer-cluster.md](findings/algorithm/writer-cluster.md) — sp+0x968 PCs (snapshot path only)
- paths/
  - [live-vs-snapshot.md](findings/paths/live-vs-snapshot.md) — two distinct cert code paths

### walls/ — confirmed dead ends with reasons (NEVER retry blindly)
- [INDEX.md](walls/INDEX.md) — list with one-liner each
- [frida-attach.md](walls/frida-attach.md)
- [frida-spawn-probe-load.md](walls/frida-spawn-probe-load.md)
- [cert-client-listener.md](walls/cert-client-listener.md)
- [nmss-probe-empty.md](walls/nmss-probe-empty.md)
- [adb-input-tap.md](walls/adb-input-tap.md)

### lateral/ — thought-experiment workarounds for every wall
- [INDEX.md](lateral/INDEX.md) — what's tried, what's untried, what's worth picking up next
- One file per wall: `lateral/<wall-slug>.md` lists 3–5 concrete bypass experiments at varied effort levels

### open/ — pending questions, in-flight work
- [INDEX.md](open/INDEX.md) — list with one-liner each
- [rpc-agent-build.md](open/rpc-agent-build.md) — current cert-ptrace direction
- [title-tap-gate.md](open/title-tap-gate.md) — needs manual user tap

## Update protocol
- When a wall is confirmed: add leaf in `walls/`, update `walls/INDEX.md`, set harness fact for cross-pollination
- When a tool is built/adapted: add leaf in `tools/built/`, update `tools/built/INDEX.md`
- When a finding lands: add leaf in `findings/`, update `findings/INDEX.md`
- When session state changes (pids, launch method, screen): rewrite `state/current-session.md`

## Briefings
Per-agent briefings at `/home/sdancer/orchestrator/briefings/<agent>.md` should be ≤30 lines: role, immediate task, current sub-goal — and link into this index for everything else.

## Fact-checker watchdog
A Sonnet-4.6 agent named `fact-checker` (pane id `6c9be5d2-03c1-4c92-994c-1cbdb49e599b`) is registered to verify worker block claims against this index.

**When the orchestrator should invoke it**: any cycle where a worker's last 20 lines contain a block signal — verbatim phrases like `Process terminated`, `Connection refused`, "I'm stuck", "doesn't work", "dead end", "wall", "can't get past", or a tool/script that fails.

**How to invoke**:
```
harness send fact-checker "BLOCK_REPORT from <agent-name>: \"<one-line claim>\" Context: <5-15 lines>. Verdict?"
```

**Reading the response**: fact-checker replies with `VERDICT: KNOWN|RELATED|NEW`, `REFERENCES:`, and `RECOMMEND:` sections. If KNOWN/RELATED, the orchestrator forwards the recommendation to the worker. If NEW, fact-checker writes a proposal to `_proposals/<slug>.md` and the orchestrator promotes it to a real leaf in `walls/`/`findings/` after sanity-check.

**Promotion of proposals**: see `_proposals/` (orchestrator-only — workers don't write here).
