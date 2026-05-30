# Spawn-owned Frida RPC agent + Python driver

**Owner**: cert-ptrace
**Status (cycle 688)**: scaffold IMPLEMENTED, NOT YET EXECUTED LIVE
**Goal**: replace one-shot probe scripts (which fast-kill) with a persistent spawn-owned Frida agent exposing rpc.exports for `init/getcert/getall`, driven by a Python session driver.

## Built artifacts (cycle 688)
- `/home/sdancer/nmss-emu/analysis/frida/nmss_live_rpc_agent_2026-05-02.js` — agent with exports: `ping`, `status`, `waitforready`, `initnmss`, `getcert`, `getall`
- `/home/sdancer/nmss-emu/frida/nmss_live_rpc_5558.py` — host driver, spawn-owned, attach-before-resume, interactive commands: `STATUS`, `INIT [CR_PATH]`, `GETCERT <challenge>`, `GETALL`, `GETALL <hex,...>`, `PING`, `QUIT`
- Checkpoint: `analysis/checkpoints/nmss_live_rpc_lane_scaffold_2026-05-02.json`
- Validation: `python3 -m py_compile` OK, `python3 frida/nmss_live_rpc_5558.py --help` OK, `node --check` OK on the JS agent

## Why this might work
Existing one-shot Frida probes fail because anti-debug detects the JS runtime / hook trampolines on script load. A persistent RPC agent has different injection timing and shape — may slip past detection. Worth testing.

## Why this might work
Existing one-shot Frida probes fail because anti-debug detects the JS runtime / hook trampolines on script load. A persistent RPC agent has different injection timing and shape — may slip past detection. Worth testing.

## Pattern source
In-tree reference: `getcert_trampoline_fork_ptrace_5558.py` (host-side RPC pattern that targets the trampoline ptrace service). cert-ptrace is adapting this shape for the live NmssSa cert API.

## Required exports
- `init()` — call NmssSa.init(Activity, null) and the loadCr/onResume/run prelude
- `getcert(challenge: hex)` — call `nmssNativeGetCertValue(Activity, challenge)`, return cert hex string
- `getall(challenges: list[hex])` — batch over the 5 standard challenges
- `status()` — return current Activity / NmssSa class state for diagnostics

## Constraints
- Must use **spawn mode + attach-before-resume** (NOT post-spawn hot-reload)
- Must NOT use any of the walled scripts in `tools/built/INDEX.md`
- Must drive against the current monkey-launched session (game 31790, nmcore 31909) — but only AFTER the title-tap gate clears

## Success criteria
5 native_cert hex strings (one per standard challenge) saved as JSON to `/home/sdancer/nmss-emu/analysis/checkpoints/native_cert_<challenge>_clean_session_2026-05-02.json`.

## If it fails
Fall back to non-Frida lanes: `dumpsys activity`, `am broadcast` to NMSS action namespaces, content provider queries, `nmsscr.dec` cache parsing. Last resort: repackaged-with-frida-gadget APK (escalate before doing).
