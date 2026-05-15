# libue4-memdump — Capture the post-decrypt libUE4.so image from a running com.needsgames.darkdecember process

## Role & workdir
Offline host-side memdump worker. Workdir: `/home/sdancer/dark-december-libue4-memdump`.

## Current goal / sub-goal
- **goal_key**: `dark_december_libue4_memdumped` (new — single-shot capture goal)
- **sub_goal_key**: `libue4-memdump-shard-capture`

## Why this turn exists
The on-disk `libUE4.so` (252 MB, at `/home/sdancer/dark-december/extract/arm64/lib/arm64-v8a/libUE4.so`) is **packed**: ELF has only 4 section headers (.dynsym/.dynstr/.dynamic/.shstrtab), entry at 0x7991000 inside a single ~101 MB R+X LOAD segment, and the middle 24 MB (file offsets 0x5f00000–0x6c00000) shows entropy 7.9+ (encrypted). The actually-executable code only exists in memory after Hercules/INCA AppGuard decrypts it at runtime.

The running process (Waydroid PID 12213, device-side) has 6887 map entries but `grep -i libUE4 /proc/12213/maps` returns nothing — INCA's standard pattern is to load the .so then unmap the dentry to evade filename scans. The decrypted code lives in **anonymous RX or RWX segments**.

We have **adb root on Waydroid** (`adbd is already running as root`).

## Hypothesis
The decrypted libUE4 code is loaded into one (or several) contiguous anonymous RX/RWX VMAs in PID 12213. A signature-match against on-disk bytes near the entry point (file offset 0x7991000, the .text start, the first ~512 bytes of which are likely the ELF-trampoline init stub the packer copies verbatim) will identify the in-memory base; once identified, we dump the matching VMA(s) as shards.

## Falsification (3 clean outcomes)
- (a) **Found unique anon RX VMA whose bytes match libUE4 entry signature → dump succeeds.** SUCCESS. Fact: `dark_december_libue4_memdump_captured_<sha256>_<size>`. Artifacts: per-VMA shards in `memdump/<base>.bin` + maps snapshot.
- (b) **No anon RX segment matches the entry signature**, but signature matches deeper inside the file (e.g. .rodata strings unique to UE4 like "FUnrealEngine" or "Epic Games") in some anon segment → dump those VMAs and document, mark partial. Fact: `dark_december_libue4_memdump_partial_<sha256>`.
- (c) **No anon segment contains any UE4-unique strings/bytes** → the game hasn't loaded libUE4 yet (splash screen, or AC is gating). Document state and recommend bringing game to a known scene first. Fact: `dark_december_libue4_not_loaded_in_pid12213`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-memdump/analysis/libue4_memdump_2026-05-15.md` documenting:
1. `pidof com.needsgames.darkdecember` at capture time.
2. Anchor signatures used (first 256 bytes of file offset 0x7991000 from on-disk libUE4; plus 3 unique UE4 strings).
3. For each matching anon VMA: address range, RW(X) flags, size, sha256 of dumped content.
4. The shard files written to `./memdump/<hex_base>.bin`.
5. Verdict matched to (a)/(b)/(c).

**Closing fact**: `dark_december_libue4_memdump_captured_<sha256>_<size>` (a) OR fact for (b)/(c).

Print `LIBUE4_MEMDUMP_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Confirm PID + capture maps snapshot.**
```bash
PID=$(adb shell pidof com.needsgames.darkdecember | awk '{print $1}')
echo "PID=$PID"
mkdir -p memdump analysis
adb shell "cat /proc/$PID/maps" > analysis/maps_snapshot.txt
wc -l analysis/maps_snapshot.txt   # expect ~6800
```
If empty → game not running; print error + outcome (c) and exit.

**Step 2 — Extract anchor signature from on-disk libUE4.**
```bash
DISKLIB=/home/sdancer/dark-december/extract/arm64/lib/arm64-v8a/libUE4.so
# entry = 0x7991000 = file offset (DYN, so virtaddr == file offset for first load)
dd if=$DISKLIB bs=1 skip=$((0x7991000)) count=256 2>/dev/null > analysis/entry_sig_256.bin
xxd analysis/entry_sig_256.bin | head -8
# Also extract 3 UE4-unique strings (search file)
strings -n 16 $DISKLIB | grep -E 'FUnrealEngine|Epic Games|UE4Editor|FName::' | head -5
```

**Step 3 — List anonymous executable VMAs from the running process.**
```bash
awk '$2 ~ /x/ && NF==5 {print $0}' analysis/maps_snapshot.txt > analysis/anon_x_vmas.txt
wc -l analysis/anon_x_vmas.txt
# Also include any RWX with pathname suppressed
awk '$2 ~ /rwx/ {print $0}' analysis/maps_snapshot.txt >> analysis/anon_x_vmas.txt
```

**Step 4 — For each candidate VMA, read first 4 KB via /proc/PID/mem and signature-match.**
Use a small Python helper (host-side, via adb shell). Cap per-VMA read at 8 MB initially; if it matches, dump full VMA. **HARD memory cap: 4 GB total per python process.** Stream to disk, never accumulate all VMAs in RAM.

```python
import subprocess, os, struct
PID = os.environ['PID']
ANCHOR = open('analysis/entry_sig_256.bin','rb').read()
ANCHOR_PREFIX = ANCHOR[:32]   # 32-byte prefix for boyer-moore-ish search

with open('analysis/anon_x_vmas.txt') as f:
    vmas = []
    for line in f:
        parts = line.split()
        if not parts: continue
        rng = parts[0]
        lo,hi = [int(x,16) for x in rng.split('-')]
        vmas.append((lo, hi, parts[1]))

# Pull samples via adb shell "dd if=/proc/$PID/mem bs=1 skip=LO count=N" — but dd on /proc/mem only works as root and may need iflag=skip_bytes
for lo, hi, perms in vmas:
    size = hi - lo
    sample_n = min(size, 8*1024*1024)
    # Use adb root + dd to read directly from /proc/PID/mem
    out_path = f'memdump/sample_{lo:x}.bin'
    cmd = f'adb shell "dd if=/proc/{PID}/mem bs=4096 iflag=skip_bytes skip={lo} count={sample_n//4096} 2>/dev/null" > {out_path}'
    subprocess.run(cmd, shell=True, check=False)
    data = open(out_path,'rb').read()
    if ANCHOR_PREFIX in data:
        offset = data.find(ANCHOR_PREFIX)
        print(f'HIT: vma 0x{lo:x}-0x{hi:x} ({perms}) size={size} match_offset={offset}')
        # Dump the entire VMA
        full = f'memdump/{lo:x}.bin'
        subprocess.run(f'adb shell "dd if=/proc/{PID}/mem bs=4096 iflag=skip_bytes skip={lo} count={size//4096} 2>/dev/null" > {full}', shell=True)
```

**Step 5 — Verify shard sha256, write artifact.**

**Step 6 — fact-set + print DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set dark_december_libue4_memdump_captured_<sha>_<size> "<summary>"
echo LIBUE4_MEMDUMP_DONE
```

## Constraints & gotchas
- **HARD per-step memory budget: 4 GB.** No `open().read()` on multi-MB shards.
- **HARD enumeration cap: top 50 anon X VMAs by size descending; sample 8 MB each; only full-dump confirmed matches.**
- **No Frida on libUE4** — anticheat will detect and kill. Use adb-root + /proc/PID/mem ONLY. Memory rule `[[no-frida-libue4]]` applies (anticheat doctrine extends from libUnreal to libUE4).
- **/proc/PID/mem reads via adb root are CURRENTLY the cheapest path.** If INCA detects (PID dies mid-dump or maps changes), fall back to kernel-side: write a small LKM that exports `/proc/libue4_dump` and reads pages via `copy_from_user`-equivalent kernel APIs. Memory rule `[[lkm-library-injection]]` applies. **Do not stall** — if userspace fails, escalate to kernel-side same turn. Memory rule `[[no-stall-kernel-aggressive]]`.
- **The process may be in transient state** (splash/loading). If outcome (c), document — don't fabricate a partial.
- **Honor `[[bulk-enumeration-needs-explicit-memory-budget]]`.** Cycle-209's 22-GB python heredoc is the negative example.
- **One Codex turn budget: ≤2 hours wall time.**

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-memdump/`
- On-disk libUE4: `/home/sdancer/dark-december/extract/arm64/lib/arm64-v8a/libUE4.so` (252 MB, packed)
- Reference for memdump pattern: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/` (2966 shards from prior NMSS campaign — same pattern, different game)
- Process: `com.needsgames.darkdecember` PID 12213 on `localhost:5558`
- Existing dark-december briefings (for context, not action): `briefings/dark-december-recon.md`, `briefings/dark-december-libcompatible-disasm.md`
- success-fact key: `dark_december_libue4_memdump_captured_<sha256>_<size>` (a)
- block-fact keys: `dark_december_libue4_memdump_partial_<sha256>` (b), `dark_december_libue4_not_loaded_in_pid12213` (c)
