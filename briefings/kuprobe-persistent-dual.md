# kuprobe-persistent-dual — Long-running HW BP probe at BOTH cipher VAs simultaneously

## Role & workdir
Engineering setup worker. Workdir: `/home/sdancer/dark-december-kuprobe-persistent-dual`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing — user-resource gated)
- **sub_goal_key**: `persistent-dual-vmh-capture`

## Why this turn exists
Cycles 372 + 380 both proved the HW BP mechanism works (125/125 attach, no AC reaction). But BOTH known cipher helpers showed 0 hits in 150-sec windows during foreground game state. The cycle-322 pcap proves these ciphers DO produce encrypted frames at some point — they just aren't firing during our short observation windows. Player must be in **active movement gameplay** for these helpers to execute.

This is the moment to commit: build a persistent dual-VA probe that runs indefinitely, captures whichever helper fires first, dumps state + cipher buffer to disk. Then escalate to user for one gameplay session.

## Hypothesis
A continuously-running HW BP probe armed at BOTH `0x6cde42c904` AND `0x6cde42aa30` simultaneously will capture cipher state the moment the player initiates movement. Cross-referencing captured state with the 47-frame pcap will decode at least some frames (frames from the SAME session would decode; frames from the cycle-322 session may have different state).

## Falsification (3 outcomes)
- (a) **User runs a gameplay session and the probe captures samples** → decode whatever subset of the pcap matches. Fact: `dark_december_wire_cipher_live_capture_<n>_events_decoded_<m>_frames`.
- (b) **User runs gameplay but probe still captures 0 samples** → ciphers we found aren't the gameplay ones; search non-libUE4 libraries OR re-memdump during active play. Fact: `dark_december_wire_cipher_libue4_helpers_dont_fire_in_gameplay`.
- (c) **No user gameplay session within reasonable time window** → infrastructure is ready, parked. Fact: `dark_december_wire_cipher_capture_armed_awaiting_gameplay`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-kuprobe-persistent-dual/analysis/persistent_probe_2026-05-15.md` with:
1. **Dual-VA probe binary** at `analysis/perf_dual_probe` — modified from `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/perf_hw_breakpoint_probe.c` to:
   - Open TWO breakpoint events per TID (one at `0x6cde42c904` for `x0+0xb8` state, one at `0x6cde42aa30` for `x19+0xb8` state)
   - Read both register sets correctly per VA
   - Tag events with `helper_va` field in the JSON output
   - Take a duration arg of 86400 (24 hours) so it can run as a daemon
2. **systemd-style runner script** at `analysis/run_persistent_probe.sh` that:
   - adb push the binary
   - adb shell launches it with nohup + setsid + redirected stdout to /data/local/tmp/dual_events.jsonl
   - Returns immediately after launching (don't block on its 24h runtime)
3. **Pull script** at `analysis/pull_events.sh` to retrieve `/data/local/tmp/dual_events.jsonl` via adb periodically.
4. **Decoder + cross-reference script** that monitors `dual_events.jsonl`, decodes any matched frames automatically.
5. **Operator instructions** in the markdown: how the user can trigger capture by playing actively for 60 seconds.
6. **Launch** the persistent probe before exiting (probe runs in background on device).
7. Verdict: outcome (c) `armed_awaiting_gameplay` is the EXPECTED result for this turn.

Print `KUPROBE_PERSISTENT_DUAL_DONE` on the final line.

## Execution flow

**Step 1 — Copy + dual-VA modify probe:**
```bash
cp /home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/perf_hw_breakpoint_probe.c \
   analysis/perf_dual_probe.c
# Edit to open 2 BPs per TID, tag events with helper VA, parametrize duration
```

**Step 2 — Compile + push:**
```bash
/usr/lib/android-sdk/ndk/27.2.12479018/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android31-clang \
  -O2 -static analysis/perf_dual_probe.c -o analysis/perf_dual_probe
adb push analysis/perf_dual_probe /data/local/tmp/perf_dual_probe
adb shell chmod +x /data/local/tmp/perf_dual_probe
```

**Step 3 — Launch persistent (background-on-device):**
```bash
PID=$(adb shell pidof com.needsgames.darkdecember | tr -d '\r' | awk '{print $1}')
adb shell "nohup /data/local/tmp/perf_dual_probe $PID 86400 > /data/local/tmp/dual_events.jsonl 2>/data/local/tmp/dual_probe.err < /dev/null &"
```

**Step 4 — Verify it's running:**
```bash
adb shell "ps -A | grep perf_dual"
```

**Step 5 — Write instructions for user.**

## Constraints & gotchas
- **HARD memory budget: 256 MB worker-side.**
- **NO ptrace, NO Frida, NO .text patch.**
- **DO NOT block on the probe runtime — launch and exit.**
- Probe runs as `u0_a103` (game user) — should work because perf_event_open from root via adb shell sets uid that can attach to the target.
- ARM64 has 4 HW BP slots per core; we're using 2 — fine.
- **One Codex turn budget: ≤20 min** (engineering only, no waiting for gameplay).

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-kuprobe-persistent-dual/`
- Source probe: `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/perf_hw_breakpoint_probe.c`
- Capture analyzer: `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/analyze_kuprobe_events.py`
- Both target VAs: `0x6cde42c904` (state owner `x0`) and `0x6cde42aa30` (state owner `x19`)
- Cycle-372 ASLR delta: `0x7395000` (verify still valid via `/proc/<pid>/maps`)
- Frame corpus: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`
- success-fact key: `dark_december_wire_cipher_live_capture_<n>_events_decoded_<m>_frames` (a)
- expected-outcome key: `dark_december_wire_cipher_capture_armed_awaiting_gameplay` (c) — this is the right closure for this turn
