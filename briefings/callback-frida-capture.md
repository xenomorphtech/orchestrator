# callback-frida-capture — capture the runtime-resolved cert callback live

## Role & workdir
Hook the cert-callback dispatch site in `libnmsssa.so` on the live ARM device via Frida; capture the resolved fn-pointer that points into the deleted-module body that actually computes the 48-char cert; dump the body so the static `nmsscore_port` Rust port has a callback to plug. Workdir: `/home/sdancer/nmss-emu-callback-frida/`.

## Current goal / sub-goal
- **Goal:** `nmss_cert_transformation_recovered` → and gates `nmss_cert_pure_rust` (5/5 algorithmic).
- **Metric:** `fraction_of_algorithm_slice_synthesized_to_rust`. Currently 0.65. Your contribution: +0.5 if you bring back the callback body, +0.2 if you only bring back the fn-ptr+disasm.
- Sister path `trace-diff` (worker `ad7fdd5fdfc346269`) is running confirmatory traces in parallel — do not interfere.

## Success criteria
1. Captured live fn-ptr value at the `blr x9` call site (libnmsssa.so module-relative `0x12c47c`) for at least one game session.
2. Identified the containing module on the device (likely a deleted-module shard with random hash name; the in-snapshot equivalent is `F7B3B00F8A5` / `CFF3FAD10` / `6273AE88C490C78D7B` / `86C9DF48BB32`).
3. Dumped the function body at that fn-ptr (≥1 page, ideally the whole function) to a file under `analysis/callback_capture_2026-05-11/`.
4. Set fact `cert_callback_body_captured_2026_05_11` to the artifact path.

## Why this is the right path (load-bearing context)
- The prior `nmssCore-disasm` path (worker a04cd1011f1108e20, completed cycle 6) proved the cert producer at `0x492ad4` is a **dispatch wrapper**, not the algorithm. It produced a Rust port at `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port/src/lib.rs` with an explicit `ProducerCallback` trait seam.
- A byte-sweep of ALL memdump shards in `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558_sg_orch_run_2026-04-23/memdump/` for cross-refs to typename literal `0x78cc6c12c2` (`"24NMSS_KIT_CERT_VALUE_INFO"`) returned **ZERO HITS** (fact `cert_callback_body_runtime_only_2026_05_11`). Static-only paths to the callback are EXHAUSTED.
- The callback is registered at runtime, likely via `std::type_info::name()` string match against the rodata typename literal. So **only live capture remains**.
- libnmsssa.so is **NOT libUnreal** — Frida is permitted on libnmsssa (no anticheat conflict). DO NOT hook libUnreal.so under any circumstance.

## Progress so far (this is a fresh path; no prior worker)
- Worktree: `/home/sdancer/nmss-emu-callback-frida/` (clean, on c654108).
- Device verified at cycle 7: `adb -s localhost:5558` returns root shell on aarch64.
- libnmsssa.so confirmed loaded at `0x6ccb62b000` in PID 1221 (`com.netmarble.thered`). Multiple PIDs may have it loaded — pick the one running cert path.
- Frida-server binaries present at `/data/local/tmp/frida-server*` on device. Check if running with `pgrep -f frida-server` on device; start if not (`/data/local/tmp/frida-server-17.9.1 &`).

## Next 2–3 concrete tasks (ORDERED)

1. **Bring up Frida** on the device. Verify with `frida-ps -U` from host. If the host doesn't have `frida` CLI, install via `pip install frida-tools` or run via the included scripts. Find the game PID (`com.netmarble.thered`).

2. **Write a Frida hook** at `/home/sdancer/nmss-emu-callback-frida/scripts/callback_capture.js` that:
   - Resolves the libnmsssa.so base via `Module.findBaseAddress("libnmsssa.so")`.
   - Hooks `base + 0x12c47c` with an `Interceptor.attach` whose `onEnter` reads `this.context.x9` — that's the resolved callback function pointer.
   - Logs (timestamp, pid, callback PC, value at *callback_PC[0..32 hex]). Also walks `/proc/self/maps` on first hit to identify the containing module (filename) at that address, and dumps a 64 KB window from `callback_PC & ~0xFFF` to `analysis/callback_capture_2026-05-11/window_<pid>_<ts>.bin`.
   - Use `Memory.readByteArray` and `File` to write. Trigger a cert call after attaching (the game generates certs naturally during play; you may need to invoke a known cert-fetch via shell broadcast or just wait).
   - Also hook `base + 0x12a59c` (post-resolver, `str x0, [x9, #2896]`) as a second witness — `x0` there is the SAME callback PC. Cross-check.

3. **Run the hook against the game and dump the callback**. Pull the dump back to host under `analysis/callback_capture_2026-05-11/`. Identify the containing module on device by reading `/proc/<pid>/maps` for the address range. Set fact `cert_callback_body_captured_2026_05_11` with path; if the module is a `(deleted)` mmap, also `cat /proc/<pid>/mem` slices to get the whole function body.

## Constraints & gotchas

- **DO NOT hook libUnreal.so.** Anticheat will crash the game / blacklist Frida. libnmsssa is fine.
- **adb / device transport may drop mid-capture.** If `adb devices` shows nothing partway, do NOT poll-spam — just save partial state and report. (Historical pattern: device dropped during 2026-05-03 captures; window may be short.)
- **No git commits, no git state changes.** Edit only files in your worktree.
- **The deleted module changes filename every session** (random hash). Identify it by its layout signature (3 r-xp segments totalling ~5 MB; the typename literal at offset matching `0x6c12c2` from module base) — NOT by filename.
- **The hook itself doesn't compute the cert.** It only captures the callback fn-ptr and dumps bytes. Synthesis (Rust port + plug into nmsscore_port) is a downstream task for a future path.
- **Frida-server is currently the v17 series** (`frida-server-17.9.1`). Host `frida` CLI must match major version; if mismatched, install matching version.

## Relevant files / references

- Rust wrapper (where the captured callback eventually plugs): `/home/sdancer/nmss-emu-nmsscore-disasm/nmsscore_port/src/lib.rs`
- Static disasm writeup: `/home/sdancer/nmss-emu-nmsscore-disasm/analysis/nmsscore_disasm_2026-05-11.md`
- Earlier Frida script (different hook PC — DO NOT reuse): `/home/sdancer/nmss-emu/frida/scripts/cert_late_frag2_v2_hook_2026-05-03.js`
- Cert callback chain fact: `harness facts | grep cert_callback_chain_resolved` (gives the hook target rationale)
- Device transport state: `harness facts | grep device_transport` (last drop 2026-05-03; recovered cycle 7 2026-05-11T18:25Z).
- 5 ground-truth pairs (snapshot-state certs): `/home/sdancer/nmss-emu/analysis/test_vectors_2026-04-24/summary.json`
- Live cert for 7BDA (different from snapshot — different session state): 48-char ASCII `B64C183D793AD722DE26F9301D7D66C20A448F1119C90596`

## Progress log
Write progress to `/home/sdancer/orchestrator/analysis/checkpoints/callback_frida_progress_2026-05-11.jsonl` (one JSON object per significant step) — the orchestrator reads this on next tick.

## Operating mode
In-process Agent (background). Iterate task 1→2→3. STOP and report if: (a) device transport drops permanently, (b) Frida attach is rejected by the game (anticheat triggered), or (c) the hook fires but x9 is consistently 0/garbage (suggests we're on the wrong PC — re-check libnmsssa offset).
