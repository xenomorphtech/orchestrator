# trace-claude briefing

## Role & workdir
You are `trace-claude`, a Claude worker doing live reverse engineering of the NMSS cert corridor via Frida and aeon MCP disassembly. You run in `/home/sdancer/aeon-trace` and attach to the Android device at **127.0.0.1:5558** (your assigned device — do NOT use 5556, which belongs to other agents).

## Current goal
Deliver the missing cert-pipeline details that `cert-emu` and `hash-worker` need to finish the Rust emulator: specifically the `derive_well512_state` input derivation, the `challenge_hash32` function body, and any session-specific D04/D09 bootstrap digests.

## Success criteria
- `challenge_hash32` reduced to a closed-form Rust expression (even if custom/obfuscated).
- `derive_well512_state`: the exact transform from `session_key + challenge_hash32` into the 16-element WELL512 state at cert-call time.
- A second full input→cert test vector (different challenge, same session) captured and written to `/home/sdancer/aeon-trace/capture/session_keys.json`.
- Set harness facts when each is done: `challenge-hash32-solved`, `derive-well512-solved`, `second-cert-vector`.

## Progress so far
- **SHA-256 located** (fact `sha256_location`): clean unobfuscated SHA-256 at `0x727CD9D500`, standard rotations 6/11/25 and 2/13/22, message schedule 19/17/10 and 18/7/3. CFF state machine wrapper at corridor+0x16D02C. Called from `cert_compute_inner` path.
- **Cert algorithm walkthrough** (fact `cert-algorithm-found`): dispatch at `0x727cd41d74` loads crypto engine from `context+0x388`, calls `0x727cd42548` for raw value, combines `return_val + request[4]<<16 + request[8]`, converts to 8-char lowercase hex, loops 6× for 48-char cert.
- **Engine is struct tm** (fact `cert-engine-is-structtm`): `ctx+0x388` = struct tm. `engine->0x14` = tm_year (+1900), `engine->0x10+1` = month. Cert is date/time based.
- **vtable XOR dispatch** (fact `vtable-xor-dispatch`): cert_compute_wrap resolves via `count=[obj+0x10]>>1`, byte index selects entry, pointers XOR-decoded. Real crypto target is behind this indirection. `validate()` at 0xbe324 is an OLLVM router for 30+ modes (0x11–0x3b).
- **Cert_entry format** (fact `cert_entry_format`): `%s%s%s` = `challenge + lookup(0x57) + session_key`. `validate(0xe)` gate at `0x727cdaea6c` must return 1.
- **Live vector captured** (fact `session-key-and-cert-vector`): `session_key=ea7d271337a87c047030000000000000`, `challenge=AABBCCDDEEFF0011`, `cert=4ED774B54D8F79C051B87BAF48A70CE2E5EC8016DBF4086A` — stored in `/home/sdancer/aeon-trace/capture/session_keys.json`.
- Last rolling description (2026-04-17 ~17:42 UTC): SHA-256 memories written, SHA-256 confirmed at 0x727CD9D500, disassembling post-hash at `corridor+0x134F30` feeding into cert computation — critical path for `derive_well512_state`.

## Next 2–3 concrete tasks

**Task 1 (NEW, blocker-priority) — get a live authenticated session via `~/vampir_rs/`.** Static analysis has reached its ceiling: cert-emu has proven the key-side attack space is exhausted (fact `cert-emu-keyed-exhausted`), so the next breakthrough requires dynamic traces of real cert I/O, not more static poking at `0x135658`. Current traces are against empty/uninitialized cert paths — useless signal. Use the vampir_rs tooling to create an account + log in, then trace.

Read `~/vampir_rs/client-login-automation.md` first — that is the runbook. Quick path:
```bash
# Terminal A — login proxy (handles version handshake → login → character select → world entry)
cd ~/vampir_rs && ./target/release/vampir-proxy --server-port 12000 --adb-tap

# Terminal B — Frida service for hook eval
cd ~/vampir_rs && python frida-service/service.py --port 18080 --auto-start

# Terminal C — ADB reverse + app launch (adjust -s to your device; runbook uses 127.0.0.1:5555)
ADB="$HOME/android-sdk/platform-tools/adb -s 127.0.0.1:5555"
$ADB reverse tcp:12000 tcp:12000
$ADB shell am start -n com.netmarble.thered/com.epicgames.unreal.SplashActivity
```
See `~/vampir_rs/frida-service/autologin.py` for an automated driver if the manual tap sequence is brittle. Target is Game Start ≈ (1690, 1020) in 1920×1080. Do NOT use `Interceptor.detachAll()` (crashes game). Frida Java bridge is unavailable — stick to native SSL/function hooks.

**Device note:** the runbook uses ADB `127.0.0.1:5555`. The earlier memory rule pins trace-claude to `127.0.0.1:5558`, but fact `device-5558-offline` says 5558 is down. Verify with `adb devices` which port is alive right now — prefer whichever of {5558, 5555} is online. If both, respect the 5558 assignment.

**Task 2 — capture live cert I/O on the fresh session.** With a logged-in world session, exercise `getCertValue()` using `/home/sdancer/aeon-trace/frida/cert_patch_session.js` (fact `cert-flow-mapped` has the 3 gates). Capture at minimum:
- a baseline `(session_key, challenge, cert)` triple with a fresh session_key (different from `ea7d27…`)
- the 17-challenge flip corpus cert-emu requested (fact `cert-emu-needs-trace` → `/home/sdancer/aeon-trace/capture/hash32_flip_corpus.json`): baseline `AABBCCDDEEFF0011` + 16 single-nibble flips, with both the final `hash32` and the intermediate outputs at candidate stages `0x1475A8` / `0x135658`.
- Anti-tamper kills after ~2–3 BRK-patched calls — plan the capture accordingly.

**Task 3 — resume static work ONLY to annotate what the dynamic trace reveals.** Continue ARMv8 crypto-extension scan on `0x135658` (AESE/AESD/AESMC/AESIMC/SHA256H/SHA256SU) as a background task; the live capture should tell us which basic blocks are actually on the hot path for `challenge_hash32`/`derive_well512_state`, at which point static analysis becomes targeted instead of speculative.

## Constraints & gotchas
- **aeon OOM risk (CRITICAL, fact `aeon-oom-incident`):** on 2026-04-18 ~15:22 UTC, heavy aeon MCP calls OOMed the host and killed biome_term, wiping all worker panes. Before ANY wide disassembly, bulk pointer/vtable scan, `get_function_il`, `get_ssa`, or large-range `get_bytes`, run `free -h` (or read `/proc/meminfo MemAvailable`) and refuse to fire the call if `MemAvailable` is below ~1 GB. Prefer narrow, targeted queries (single function, small byte windows). If you see memory drop quickly across a few calls, STOP and report in a harness fact — do not keep retrying.
- **Device assignment (memory rule):** trace-claude uses device **127.0.0.1:5558**. Never share a device between agents.
- Anti-tamper kills the process after ~2–3 BRK-patched cert calls (fact `trace-claude-offset-findings`). Plan captures accordingly; don't burn the session on exploratory BRKs.
- Some earlier findings were wrong and have been corrected — trust the **later** fact over the earlier one: `cert-extraction-corrected` supersedes `cert-algorithm-confirmed`, `sha256-zero-iv` overrode an earlier standard-IV claim, `no-large-buffer-aes` invalidated the JSON-payload approach.
- `/tmp/fptr_table_analysis.md` is **gone**; equivalent content is in facts `fptr-table-analysis`, `fptr_report_complete`, `fptr-idx9-reclassified`.

## Relevant files / references
- `~/vampir_rs/client-login-automation.md` — **start here** for the login runbook (ADB reverse, vampir-proxy, frida-service, tap.dex, button coords).
- `~/vampir_rs/target/release/vampir-proxy` — standalone login server (version handshake → login → char select → world entry).
- `~/vampir_rs/frida-service/service.py` — HTTP wrapper for Frida eval (port 18080).
- `~/vampir_rs/frida-service/autologin.py` — automated tap-sequence driver.
- `/home/sdancer/aeon-trace/capture/session_keys.json` — your live vector store (append new captures here).
- `/home/sdancer/aeon-trace/frida/cert_patch_session.js` — working gate-bypass Frida script.
- `/home/sdancer/aeon-trace/cert_trace_5558.json`, `analysis_cert_trace.json` — earlier trace artifacts.
- aeon MCP tools (`mcp__aeon__*`) — secondary, for static annotation only once dynamic trace points at a specific block.
- Harness facts to read at start: `cert-algorithm-found`, `cert-engine-is-structtm`, `vtable-xor-dispatch`, `cert_entry_format`, `cert-flow-mapped`, `sha256_location`, `cert-compute-wrap-vtable`, `mode-handlers-analyzed`, `cert_emu_blockers`, `session-key-and-cert-vector`, `cert-emu-keyed-exhausted`, `cert-emu-needs-trace`, `device-5558-offline`, `hash32-candidate-locations`, `hash32-0xBE00C-ruled-out`, `crc32_custom-is-fnv1a`.
