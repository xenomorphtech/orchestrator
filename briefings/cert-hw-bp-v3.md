# cert-hw-bp-v3 — BP at the FIRST str/memcpy into sp+0x968 (pre-message-schedule)

## Role & workdir
Continuation of cert-hw-bp-v2 (closed at commit `909695d` 2026-05-18). Same worker, same worktree `/home/sdancer/nmss-emu-cert-hw-bp` on branch `cert-hw-bp`. All v2 tooling still on disk: `cert_hw_bp_probe`, `read_mem`, `find_fn_entry.py`, `correlate.py`, `verify_blocks.py`, `drive_v2.sh`, `drive_v2_norestart.sh`.

## Goal of this turn
v2 verdict (your own analysis in `analysis/cert_hw_bp_v2_verdict.md`): BP at off=0xf38 (first `ldr w,[sp,#0x968]`) fires **after** the SHA-256 message schedule has already overwritten sp+0x968 with W[i>0]. Captured block shape (16-byte zero prefix, 0x02 marker at byte 16, 20 bytes session-varying hash, 0x0c at byte 48) is mid-compression state, NOT the pre-image. SHA-256 algorithmically requires the original 64-byte message at sp+0x968 BEFORE schedule starts — capture that.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay`
- **sub_goal_key**: `cert-hw-bp-v3`

## Hypothesis
The cert function has a **first WRITE** to sp+0x968 (either an unrolled `str`/`stp`-chain or a `memcpy(sp+0x968, src, 64)` call) that happens BEFORE the message schedule loop. A HW execute BP placed at that first write site (or the first instruction after that write completes) captures the original 64-byte pre-image. The pre-image, hashed via `sha256_compress_one_block(block)[4..28]` (the 4/4-verified algorithm in `analysis/verify_blocks.py`), produces the on-wire op901 Token.

## Falsification criteria (any one)
- No `str`/`stp`/`memcpy` to `[sp,#0x968]` exists in the cert function before the first `ldr w,[sp,#0x968]` at off=0xf38 → the pre-image is built elsewhere (heap, register-only) and never sits at sp+0x968 in plain form.
- First-write BP fires + captures a 64B block, but it still doesn't hash to the live Token. Then either: (a) algorithm spec wrong (would contradict 4/4 prior validation), or (b) the sp+0x968 stack slot isn't actually the SHA pre-image — it's a related buffer.
- 0 firings at the new offset across 2 driven logins (with confirmed op901 emission) — function not at this offset.

## Hard rules
- **Workdir is the existing worktree** — don't `git worktree add` a new one. Same branch `cert-hw-bp`. Reuse all v2 tooling.
- **HW BP only**, no kernel uprobe (NMSS detects, cycle 1090).
- **adb localhost:5558**.
- **Driven login MUST produce op901** before declaring verdict.
- **30 min wall cap.** Context budget reset via the `/clear` you just got — be terse.

## Step 1 — find the first WRITE to sp+0x968 in the cert function
1. Open `analysis/cert_hw_bp_v2/range2.bin` (already dumped from PID 2272 last session; module bytes are stable across same-install reboots).
2. Search the cert function range (seg2 offsets 0x0..0x6eed8 roughly, but really anywhere in seg2) for instructions that **write** to sp+0x968:
   - `str Xd, [sp, #0x968]` encoding: `0xF900_xxxx` family with imm=`0x968>>3`=`0x12d`. Mask + match the immediate field.
   - `str Wd, [sp, #0x968]` encoding: `0xB900_xxxx` family.
   - `stp Xa, Xb, [sp, #0x960]` (writes 16B starting at sp+0x960; covers 0x968 with the second register): `0xA900_xxxx` family, imm=`0x960>>3`=`0x12c`.
   - `add Xd, sp, #0x968` followed shortly by `bl <memcpy>` or `bl <some_init_fn>` — pre-image written via call.
3. Build a list of all such write sites. The cert function had `27 str hits to sp+0x968` per cycle-1112 disasm (from candidate at off=0x260790, which was the wrapper-writer); the actual SHA function at off=0xf38 may have its own first-write site distinct from that wrapper. Re-scan WITHIN the cert function's (entry, ret) boundary using `find_fn_entry.py`.
4. Pick the **lowest-offset** write site in the function body (closest to function entry). That's the "first write" candidate.
5. Compute its runtime VA at the current thered PID (`seg2_base + offset_in_seg2`).

## Step 2 — re-arm probe at the first-write VA + run capture
1. Reuse `drive_v2_norestart.sh` with `CERT_OFFSET_IN_SEG2` set to the new offset. The probe binary `cert_hw_bp_probe` from v2 is still valid (same encoding).
2. Run 120s driven login. The login NEEDS to reach cert-emit — verify with `pidof com.netmarble.thered` survival + non-empty pcap before considering 0-samples a falsification.
3. If samples land: scan the 768B sp-window dump for the 64-byte pre-image (worker has `scan_blocks.py` for this from v2).

## Step 3 — algorithm validation
1. For each captured 64-byte candidate block: `python3 analysis/verify_blocks.py --block-hex <hex> --expected-token <Token>`.
2. **MATCH** → set fact `cert_algo_validated_via_hw_bp_v3_2026_05_18=true` with block + cert_fn VA + offset_in_seg2. Goal `nmss_clientless_fresh_login_replay` → 6/6 closure.
3. **No match across all candidates in dump** → BP timing still wrong, OR the pre-image is built via in-place writes that race the BP. Set diagnostic fact `cert_hw_bp_v3_first_write_no_match_2026_05_18`. STOP.

## Step 4 — verdict + commit
- Single commit on branch `cert-hw-bp`, message `cert HW BP v3: first-write BP — <verdict>`.
- Update `analysis/cert_hw_bp_v3_verdict.md` with: chosen offset, runtime VA, sample count, block hex of any candidate(s), algorithm match result.
- Final line: `CERT_HW_BP_V3_DONE`.

## Constraints & gotchas
- The cert function is the giant flat-CFG blob in seg2 with **1752 SHA-256 rotations** (per v2 verdict). Boundaries via `find_fn_entry.py`; do NOT re-scan all of seg2 — bounded search inside the cert fn body only.
- Block structure observed in v2: 16-byte zero prefix, 0x02 at byte 16, 20-byte session-varying material, 0x0c at byte 48. This is **mid-state**, not pre-image. Don't chase it — the pre-image has the canonical SHA-256 message shape (your message ≤ 56B + 0x80 padding + zeros + BE u64 bitlen).
- If 1st-write site is a `memcpy` call, BP at the **instruction after** the `bl` is the cleanest — pre-image is fully written.
- 5 ground-truth pairs from prior cycles available in `harness facts | grep thered_cert_pair`. New pairs collected this run get appended.
- The worker has done 4 runs and ~50 min already on this campaign — be terse, don't redo disasm that's in `analysis/cert_hw_bp_v2/`.

## Relevant files / references
- `analysis/cert_hw_bp_v2_verdict.md` — v2 closure with v3 hypothesis
- `analysis/cert_hw_bp_v2/range2.bin` — deleted module bytes, stable across same-install
- `analysis/cert_hw_bp_v2/find_fn_entry.py`, `correlate.py`, `scan_blocks.py`
- `analysis/verify_blocks.py` — 4/4 algorithm verifier
- `analysis/cert_hw_bp_probe` — perf_event_open HW BP harness (works)
- `analysis/cert_hw_bp_v2/drive_v2_norestart.sh` — driver with `CERT_OFFSET_IN_SEG2` env
- Facts: `cert_hw_bp_substrate_validated_2026_05_18`, `cert_hw_bp_v2_run3_timing_too_early_2026_05_18` (NOTE: that fact's title says "early" — your v2 verdict corrected this to "too LATE" — the message schedule overwrites sp+0x968 by the time the BP fires; both pre-write and post-write descriptions amount to "BP isn't at the right moment for original pre-image bytes").
