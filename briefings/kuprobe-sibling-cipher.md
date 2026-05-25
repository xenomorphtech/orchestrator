# kuprobe-sibling-cipher — Run HW BP capture at the sibling cipher VA 0x6cde42aa30

## Role & workdir
Reuse durable HW BP infrastructure at a new target VA. Workdir: `/home/sdancer/dark-december-kuprobe-sibling-cipher`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `capture-cipher-state-at-sibling-helper`

## Why this turn exists
Cycle 376 `find-wire-cipher-sibling` located a SIBLING chained-XOR function at stale VA `0x6ce57bfa30` (image-rel `0x857ca30`). Live VA after cycle-372 ASLR delta `0x7395000` is **`0x6cde42aa30`**. This is a 448-byte inline helper with the same 7-instruction XOR recurrence + `+0xb8` state advance (owner register `x19` instead of `x0`).

Cycle 372 PROVED the HW BP mechanism: `perf_event_open(PERF_TYPE_BREAKPOINT, HW_BREAKPOINT_X)` attaches cleanly to all 125 PID 28722 threads, no Hercules reaction. Cycle 372 also empirically falsified `0x6cde42c904` (the original helper) as the gameplay wire cipher.

**The infrastructure is sitting ready at `/home/sdancer/dark-december-move-kuprobe-cipher-state/perf_hw_breakpoint_probe.c`** — extends to a mmap'd ring buffer with sample extraction (regs + /proc/mem reads).

## Hypothesis
The sibling helper `0x6cde42aa30` IS the wire cipher exercised during normal ingame traffic (including heartbeats). A 120-sec HW BP capture during foreground game state will yield ≥1 sample with valid cipher state at `[x19+0xb8]`.

## Falsification (3 outcomes)
- (a) **≥1 sample captured AND ≥1 captured frame body_hex prefix matches an event's cipher_head bytes AND cycle-335 recurrence decodes plausible plaintext (msg_type 0x0385/0x0386 + sensible handle)** → SUCCESS. Fact: `dark_december_wire_cipher_decoded_sibling_<n>_frames`.
- (b) **Samples captured but no cross-reference match to pcap corpus** → cipher fires for in-session traffic but cycle-322 pcap is from a different session/connection (different state stream). Document samples for future session-correlated decode. Fact: `dark_december_wire_cipher_sibling_captured_<n>_samples_no_pcap_match`.
- (c) **150-sec window, 0 samples again** → THIS helper also isn't the cipher. Both falsified; either there's a 3rd cipher (unlikely, capstone found only 2) OR the cipher signature differs from our pattern OR foreground state is wrong. Fact: `dark_december_wire_cipher_sibling_also_not_exercised`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-kuprobe-sibling-cipher/analysis/sibling_capture_2026-05-15.md` with:
1. **Copy** the probe source from `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/perf_hw_breakpoint_probe.c` into `analysis/perf_sibling_probe.c`. Change `BP_ADDR` (or wherever bp_addr is set) from `0x6cde42c904` to `0x6cde42aa30`. Note: state field is at `x19+0xb8` for this helper, not `x0+0xb8` — adjust the /proc/mem read base register accordingly.
2. **Recompile** with the NDK cross-compiler used in cycle 372: `/usr/lib/android-sdk/ndk/27.2.12479018/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android31-clang`.
3. **Run** the probe on device for at least 120 seconds against PID `pidof com.needsgames.darkdecember`. Capture events into `analysis/sibling_events.jsonl`.
4. **Cross-reference** captured events with `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` — match by body_hex prefix to event cipher_head_hex. Reuse `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/analyze_kuprobe_events.py` (same code path).
5. **Decode** matched frames using cycle-335 recurrence `p[0]=c[0]^state[0]; p[i]=c[i]^p[i-1]^state[i&7]` with the captured state. Verify msg_type ∈ {0x0385, 0x0386} for long frames, msg_type sensible for short frames.
6. **Output decoded fields** for all matched frames (msg_type, actor_handle, 6×u32 coord/param, flag).
7. Verdict matched to (a)/(b)/(c). Set the closing fact via `harness fact-set`.

Print `KUPROBE_SIBLING_CIPHER_DONE` on the final line.

## Execution flow

**Step 1 — Verify live VA at probe start:**
```bash
PID=$(adb shell pidof com.needsgames.darkdecember | tr -d '\r' | awk '{print $1}')
adb shell "cat /proc/$PID/maps | grep -i 'r-xp.*split_config.arm64'" | head -2
# Expect: text mapping starts at 0x6cdd83f000 (cycle-372 confirmed). If different, recompute delta.
```

**Step 2 — Copy + modify probe:**
```bash
cp /home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/perf_hw_breakpoint_probe.c \
   analysis/perf_sibling_probe.c
# Edit: BP address constant 0x6cde42c904 → 0x6cde42aa30
# Edit: state read base register x0 → x19
```

**Step 3 — Compile + push to device + run:**
```bash
/usr/lib/android-sdk/ndk/27.2.12479018/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android31-clang \
  -O2 -static analysis/perf_sibling_probe.c -o analysis/perf_sibling_probe
adb push analysis/perf_sibling_probe /data/local/tmp/perf_sibling_probe
adb shell chmod +x /data/local/tmp/perf_sibling_probe
adb shell "/data/local/tmp/perf_sibling_probe $PID 120" > analysis/sibling_capture.log 2>&1
# Move events log back:
adb pull /data/local/tmp/sibling_events.jsonl analysis/sibling_events.jsonl
```

**Step 4 — Cross-reference:**
```bash
python3 /home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/analyze_kuprobe_events.py \
  --events analysis/sibling_events.jsonl \
  --frames /home/sdancer/dark-december-body-decode/analysis/frames.jsonl
```

**Step 5 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 256 MB.**
- **NO bulk binary disasm. NO new memdump. NO Frida. NO ptrace.**
- **One Codex turn budget: ≤30 min wall time.**
- The owner register changed from `x0` to `x19` in this sibling. When reading the state struct from `/proc/<pid>/mem`, use `regs.x19 + 0xb8`, not `regs.x0 + 0xb8`.
- Also adjust the cipher buffer read: for `0x6cde42c904` it was `x1`; for the sibling at `0x6cde42aa30`, examine the function disasm (in `/home/sdancer/dark-december-find-wire-cipher-sibling/analysis/sibling_search_2026-05-15.md`) to find which register holds the packet pointer — likely also `x1` or `x21`/`x22` since the helper is larger (448B vs 184B).

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-kuprobe-sibling-cipher/`
- **Source probe** (copy + modify): `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/perf_hw_breakpoint_probe.c`
- **Analyzer** (reuse as-is): `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/analyze_kuprobe_events.py`
- Sibling cipher disasm: `/home/sdancer/dark-december-find-wire-cipher-sibling/analysis/sibling_search_2026-05-15.md`
- Frame corpus: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl`
- Cipher recurrence spec: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Live VA: `0x6cde42aa30`; image-rel: `0x857ca30`; cycle-372 ASLR delta: `0x7395000`
- success-fact key: `dark_december_wire_cipher_decoded_sibling_<n>_frames` (a)
- block-fact keys: `dark_december_wire_cipher_sibling_captured_<n>_samples_no_pcap_match` (b), `dark_december_wire_cipher_sibling_also_not_exercised` (c)
