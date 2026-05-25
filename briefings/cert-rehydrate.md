# cert-rehydrate — reconstruct cert-rust-repro session.json for current PID

## Role & workdir
Algorithm-validation worker. Workdir: `/home/sdancer/nmss-emu-device-webview-signin` (reuse — has the captured op901 artifacts). The cert-rust-repro tree lives at `/home/sdancer/nmss-emu/cert-rust-repro/` (different tree, you may `cd` to it for builds; do NOT modify it without care — it's outside the worktree).

## Why this turn exists
Cycle 1022-1023 captured op901 + 902 end-to-end (fact `thered_device_op901_902_witnessed_2026_05_17`). Hands us our FIRST ground-truth (challenge → cert) pair for current device PID `87BCB7…`:
- **Captured Token (cert)** = `6045D74EB176E0C4A4DB23F0CEBEBEB7EE7644730D213147` (48 hex / 24 B)
- **Captured Log/sessionID** = `D0D2691014BE1858` (16 hex / 8 B; used as both Log halves separated by `,`)
- Frame 40 of `analysis/artifacts/conn2.pcap`; full extraction at `analysis/artifacts/conn2_op901_extracted.json`

Per cert-rust-repro `README.md`, the verified algorithm is:
1. Take a 64-byte `sp_0x968_block_hex64` from session state.
2. Run ONE SHA-256 compression-block from canonical IV.
3. Take digest bytes `[4..28]`.
4. Return as lowercase 24-byte (48-char) hex.

The challenge field is currently IGNORED by cert-rust-repro — the cert is purely a function of `sp_0x968_block`. The unknown is `f(session_facts) → sp_0x968_block`.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (successor goal, opened cycle 1024)
- **sub_goal_key**: `cert-rehydrate`

## Hypothesis
A `sp_0x968_block_hex64` can be reconstructed for the current device session by (a) capturing it directly from libnmsssa's memory at op901 emit time, OR (b) finding a deterministic encoding from observable session facts (sessionID, PID, NId, timestamp) that satisfies SHA-256 padding constraints.

## Falsification criterion
Reconstructed `sp_0x968_block` does NOT make cert-rust-repro produce `6045D74EB176E0C4A4DB23F0CEBEBEB7EE7644730D213147`, AND no candidate encoding fits within 2 hours of work → mark CL2 falsified, hand off to backlog H-CL1.

## Plan — two-track in one turn

### Track A (CHEAP, do FIRST) — Try synthesizing session.json from wire facts alone
Take what we already have and see if cert-rust-repro accidentally produces the right answer:
1. Read `donor_session_2026-04-29.json` at `/home/sdancer/nmss-emu/cert-rust-repro/donor_session_2026-04-29.json` (10 fields).
2. Construct candidate JSONs for current device by replacing fields with the captured wire data:
   - `session_token_hex32` ← derive from sessionID `D0D2691014BE1858`. Try: (i) padded right with zeros to 32 hex, (ii) the duplicated form `D0D2691014BE1858D0D2691014BE1858`, (iii) ASCII-of-hex `0x44 0x30 0x44 0x32 …`, (iv) the captured Token's first 32 chars `6045D74EB176E0C4A4DB23F0CEBEBEB7` (unlikely but cheap).
   - `ctx_0x240_text` ← set to captured Token `6045D74EB176E0C4A4DB23F0CEBEBEB7EE7644730D213147` (since donor's ctx_0x240_text = donor's cert output — same field shape).
   - Other fields: copy from donor unchanged (workspace_*, sp_0x968_block, ctx_0x258_text, package_name).
3. For each candidate JSON, run:
   ```bash
   cd /home/sdancer/nmss-emu/cert-rust-repro
   target/release/cert_rust_repro --challenge D0D2691014BE1858 --session /tmp/candidate.json
   ```
   Capture output. If output == `6045d74eb176e0c4a4db23f0cebebeb7ee7644730d213147` → SUCCESS.
4. **Expected outcome of Track A**: probable failure, because `sp_0x968_block` is the SHA input and we just copied the donor's. The point is to verify the binary runs cleanly + confirm the cert depends on `sp_0x968_block` (challenge-insensitive per README).

### Track B — Capture real `sp_0x968_block` from device via Frida on libnmsssa
Per memory `feedback_no_frida`: "No Frida on libUnreal.so (anticheat); **OK for Java/system via xerda binary**." `libnmsssa.so` is NOT libUnreal — it's the netmarble SSA library; Frida should be safe there.

1. Verify Frida-server is available on device:
   ```bash
   adb -s localhost:5558 shell "su 0 sh -c 'ls -la /data/local/tmp/frida-server 2>&1; pidof frida-server'"
   ```
   If frida-server absent, STOP this track and emit fact `cert_rehydrate_frida_absent_2026_05_17` — defer to backlog H-CL3 (kuprobe).

2. If frida-server present, identify the cert function in libnmsssa:
   - Per cycle 33-43 facts, cert generation lives in `nmssCoreGetCertValue` at offset `0x17ded0` (relative to libnmsssa.so base).
   - More precisely: encoder at libnmsssa offset `0x1113c0` produces cert via `bl 0x11b104` at `0x17ee1c`. The block lives at `*[sp+0x230]=x8` (per cycle 54 fact).
   - Get the libnmsssa.so base address for running thered:
     ```bash
     adb -s localhost:5558 shell "su 0 sh -c 'cat /proc/$(pidof com.netmarble.thered)/maps | grep -i libnmsssa | head -3'"
     ```
   - Compute absolute target = base + 0x17ee1c (the bl call site after sp+0x968 block fill).

3. Write a 30-line Frida JS hook (in `analysis/artifacts/frida_cert_block_capture.js`):
   ```js
   const TARGET = ptr('0xLOAD_BASE_PLUS_OFFSET');  // compute on device
   Interceptor.attach(TARGET, {
     onEnter(args) {
       const sp = this.context.sp;
       const block = sp.add(0x968).readByteArray(64);
       console.log('SP_0x968_BLOCK:' + Array.from(new Uint8Array(block)).map(b => b.toString(16).padStart(2,'0')).join(''));
     }
   });
   ```

4. Run frida-server + driver, force-stop + relaunch thered (same flow as cycle 1022), capture the printed block. Save as new candidate session.json.

5. Run cert_rust_repro with the new session.json; check output matches captured Token.

### Track B alternative — if Frida blocked or libnmsssa-anticheat hits
Defer to backlog H-CL3 (kuprobe on libnmsssa cert function). Don't attempt the kprobe in this turn; emit a "needs kuprobe" fact.

## Hard rules
- **adb target**: `localhost:5558` only (NOT Waydroid, NOT 192.168.2.2).
- **No Frida on libUnreal.so.** libnmsssa.so is OK.
- **Do NOT modify** `/home/sdancer/nmss-emu/cert-rust-repro/` source. Build the existing binary if needed (`cargo build --release` in that dir), then call it.
- **Do NOT clear thered app data.** force-stop + relaunch is OK (cycle 1022 precedent).
- **30 min wall cap.** 512 MB RSS cap.
- If Track A succeeds, STOP — no need for Track B.

## Outputs
- `analysis/artifacts/cert_rehydrate_track_A.json` — Track A candidates + results (4-6 rows max).
- If Track B runs: `analysis/artifacts/cert_rehydrate_track_B.json` with `sp_0x968_block_hex64`, computed cert, ground-truth cert, match=true/false.
- Facts to emit:
  - SUCCESS: `cert_rehydrate_pair_validated_2026_05_17` = true, source_ref to artifact
  - PARTIAL: `cert_rehydrate_block_captured_no_match_2026_05_17` = true (block captured but cert mismatch — schema deeper than expected)
  - FALSIFY: `cert_rehydrate_falsified_2026_05_17` = true (neither track works)
- Final line: `CERT_REHYDRATE_DONE` (or `CERT_REHYDRATE_DEFERRED` if Frida absent).

## References
- `/home/sdancer/nmss-emu/cert-rust-repro/README.md` (algorithm + schema docs)
- `/home/sdancer/nmss-emu/cert-rust-repro/donor_session_2026-04-29.json` (10-field schema reference)
- `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/conn2_op901_extracted.json` (captured Token + Log)
- `/home/sdancer/nmss-emu-device-webview-signin/analysis/artifacts/conn2_cert_analysis.md` (cycle 1023 verdict)
- Memory: `feedback_no_frida` (libnmsssa is OK target for Frida)
- Fact `thered_device_op901_902_witnessed_2026_05_17` (ground truth proof)
