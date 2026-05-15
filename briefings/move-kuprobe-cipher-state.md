# move-kuprobe-cipher-state — Capture cipher state via eBPF uprobe on 0x6ce57c1904

## Role & workdir
eBPF / uprobe development worker. Workdir: `/home/sdancer/dark-december-move-kuprobe-cipher-state`.

## Current goal / sub-goal
- **goal_key**: `dark_december_move_message_decoded` (continuing)
- **sub_goal_key**: `kernel-uprobe-cipher-state-capture`

## Why this turn exists
After 7 cycles of offline cryptanalysis (cycles 347-363), the offline corpus has been exhausted. We have:
- Wire layout 100% recovered (37/33-byte exact, packet IDs 0x0385/0x0386)
- Cipher recurrence 100% verified at 0x6ce57c1904: `p[0]=c[0]^k[0]; p[i]=c[i]^p[i-1]^k[i&7]` with state at `[x0+0xb8]` advancing `+=payload_len` post-loop
- **MISSING**: the per-packet value of `[x0+0xb8]` at time of each XOR call. Offline analysis proves identical state across 9 broadcast frames is required but impossible under monotonic advance — there's a caller writing state per-packet that we couldn't find via offline disasm (cycle 363 OOM'd).

Per user standing rule "Stale/parked is never the answer — go kernel" and `[[feedback_no_stall_kernel_aggressive]]`: we go kernel-side. eBPF uprobe is Hercules-clean per cycle-173 doctrine (uprobes don't show in /proc/self/maps, don't modify .text — they use lazy fetch-time INT3-equivalent patching).

## Hypothesis
An eBPF uprobe attached to entry of `0x6ce57c1904` in libUE4.so will capture, per invocation:
- `[x0+0xb8 .. x0+0xc0]` — the 8-byte cipher state actually used for this packet's XOR
- `x1` and `x2` — ciphertext pointer + total length (for matching against captured pcap frames)
- caller PC from `x30` — identifies WHICH code path is invoking the cipher (might reveal multiple cipher entry points or one centralized dispatcher)
- `[x1+0..x1+8]` — first 8 bytes of cipher buffer (matches pcap body_hex prefix for cross-referencing)

Once captured, decoding all 22 long frames in the existing pcap corpus is a one-shot exercise.

## Falsification (3 outcomes)
- (a) **eBPF uprobe attaches AND captures ≥10 events; states + caller PCs collected → decode all 22 frames + write Python decoder using captured per-packet state** → SUCCESS. Fact: `dark_december_move_kuprobe_state_captured_<n>_events_decoded_<m>_frames`.
- (b) **Uprobe attaches but Hercules detects within 3 captures (process exits / app freezes / network disconnect)** → AC tier higher than expected. Fact: `dark_december_move_kuprobe_anticheat_detected`. Fallback: LKM page-fault-based instrumentation (cycle-173 backlog).
- (c) **Uprobe attaches but cipher function not invoked during a 5-minute idle+gameplay window** → cipher path is taken only on specific events; needs longer capture or different gameplay actions. Fact: `dark_december_move_kuprobe_function_not_exercised`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-move-kuprobe-cipher-state/analysis/kuprobe_state_2026-05-15.md` with:
1. **Address resolution**: get the runtime base address of libUE4.so in the com.needsgames.darkdecember process. `cat /proc/<pid>/maps | grep libUE4` (after `adb shell` into Waydroid). Compute the file-offset of 0x6ce57c1904 within the libUE4.so DSO: offset = `0x6ce57c1904 - libUE4_load_base`. Then uprobe target = `<libUE4.so file path>:<file_offset>`.
2. **eBPF program** (BCC or bpftrace; bpftrace is simpler if available):
   ```bpftrace
   uprobe:/data/.../libUE4.so:0x... {
       printf("ts=%lu pid=%d tid=%d x0=%p x1=%p x2=%p\n", nsecs, pid, tid, arg0, arg1, arg2);
       printf("state: 0x%lx\n", *((uint64*)(arg0 + 0xb8)));
       printf("cipher_head: 0x%lx\n", *((uint64*)arg1));
       printf("ra: 0x%lx\n", reg("lr"));
   }
   ```
   Equivalent BCC Python program acceptable.
3. **Attach + capture**: start the eBPF prog with a small ringbuffer (4MB), trigger gameplay (player movement). Capture ≥10 events. Save to `kuprobe_capture.jsonl`.
4. **Cross-reference with pcap**: for each captured event, find the matching captured frame in `frames.jsonl` (match by cipher prefix bytes or by length+timestamp proximity). State should explain that frame's decryption.
5. **Decoder**: write a Python script that takes a frame's `body_hex` + the matching state from `kuprobe_capture.jsonl` and produces decoded plaintext using cycle-335 recurrence. Verify msg_type=0x0385/0x0386 in plaintext bytes 0..1 + plausible handle in bytes 2..9.
6. **Decode all 22 long frames** if state coverage is sufficient. List decoded msg_type, handle, coord/param fields per frame.
7. Verdict matched to (a)/(b)/(c). Set the closing fact via `harness fact-set`.

Print `MOVE_KUPROBE_STATE_DONE` on the final line.

## Execution flow

**Step 1 — Address resolution.**
```bash
adb shell 'pidof com.needsgames.darkdecember'
# Then via adb shell, find libUE4 in /proc/<pid>/maps
adb shell "cat /proc/<pid>/maps | grep -i libUE4"
# This gives load_base; libUE4 file in /data/app/.../base.apk or extracted .so
```

**Step 2 — Determine uprobe target offset.**
```python
# 0x6ce57c1904 is the runtime VA from prior cycles
# libUE4 image base was 0x6cdd243000 (per cycle 322 memdump)
# .text offset within image: 0x6ce57c1904 - 0x6cdd243000 = 0x14b58904
# This is the file offset (assuming .text starts at file offset 0 in the unpacked .so)
# VERIFY against the dumped ELF header / readelf to confirm
```

**Step 3 — Check eBPF availability on Waydroid kernel.**
```bash
adb shell 'cat /sys/kernel/debug/tracing/available_filter_functions 2>/dev/null | head -5'
adb shell 'which bpftrace bpftool'
adb shell 'ls /sys/kernel/tracing/uprobe_events'
# If bpftrace is not available on the Waydroid container, fall back to:
# - Direct write to /sys/kernel/tracing/uprobe_events (uprobe registration via tracefs)
# - Read events from /sys/kernel/tracing/trace
```

**Step 4 — Attach uprobe, capture.**
```bash
# Example via tracefs (works on most Android kernels):
adb shell 'echo "p:cipher /data/.../libUE4.so:0x14b58904" > /sys/kernel/tracing/uprobe_events'
adb shell 'echo 1 > /sys/kernel/tracing/events/uprobes/cipher/enable'
# Read events:
adb shell 'cat /sys/kernel/tracing/trace_pipe'
```
Note: tracefs only gives PC+timestamp; deeper register inspection needs bpftrace or BCC. If those aren't available on-device, install bpftrace via apt on Waydroid OR use a custom kernel module.

**Step 5 — Trigger gameplay.**
Coordinate with the user via the harness for a 60-second active-movement capture window. If user is not interacting, the cipher path may not be exercised — document and exit at outcome (c).

**Step 6 — Decode.**
Pair captured (state, cipher_bytes) with pcap `body_hex`. Apply cycle-335 recurrence. Verify msg_type + handle for one frame; if it works, decode all 22.

**Step 7 — Write artifact + fact-set + DONE.**

## Constraints & gotchas
- **HARD memory budget: 256 MB**. NO bulk binary disasm. NO capstone full-shard scan (caused cycle-363 OOM at 20.9 GB).
- **NO Frida on libUE4.so** (anticheat blocks).
- **NO ptrace** (anticheat may detect).
- **NO .text patching** (anticheat may detect).
- **eBPF/uprobes/kprobes ALLOWED** per kernel-instrumentation memory.
- **HARD output cap**: artifact ≤500 lines.
- **One Codex turn budget**: ≤30 min wall time for the first turn. If gameplay capture is needed beyond that, document setup and pause for user.
- If gameplay capture requires user interaction, **do NOT block waiting** — set up the trap, document instructions, save state, exit. Subsequent turn picks up captured events.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-move-kuprobe-cipher-state/`
- Verified cipher function: `/home/sdancer/dark-december-xor-recurrence-verify/analysis/recurrence_verify_2026-05-15.md` (VA 0x6ce57c1904)
- libUE4 memdump (3 shards): `/home/sdancer/dark-december-libue4-memdump/memdump/` (image base 0x6cdd243000)
- Frame corpus: `/home/sdancer/dark-december-body-decode/analysis/frames.jsonl` (47 frames, 22 long)
- Move/Stand layout: `/home/sdancer/dark-december-move-decode/analysis/move_decode_2026-05-15.md`
- Cipher algorithm spec: `/home/sdancer/dark-december-libue4-xor-disasm/analysis/xor_algorithm_2026-05-15.md`
- Substrate exhaustion record: `/home/sdancer/orchestrator/analysis/falsified.md` entries 2026-05-15 (cycles 347-363)
- adb target: `localhost:5558`
- success-fact key: `dark_december_move_kuprobe_state_captured_<n>_events_decoded_<m>_frames` (a)
- block-fact keys: `dark_december_move_kuprobe_anticheat_detected` (b), `dark_december_move_kuprobe_function_not_exercised` (c)
