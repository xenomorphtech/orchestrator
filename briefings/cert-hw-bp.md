# cert-hw-bp-v16 — MAGIC32 heap scan in libnmsssa.so rw data segment

## Role & workdir
Continuation of cert-hw-bp-v15 (closed at commit `6584126` 2026-05-18). Same worker, same worktree `/home/sdancer/nmss-emu-cert-hw-bp` on branch `cert-hw-bp`. v15 verdict in `analysis/cert_hw_bp_v15_verdict.md`.

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `cert-hw-bp-v16-libnmsssa-rw-scan`

## Why v16
v15 dumped 68 MB of scudo:primary heap from thered PID 24114. Marker search for cert pre-image strings (challenge, selector, cluster prefixes) returned ZERO hits. 27 unique 32-char hex candidates → 0/27 MD5 match against ground-truth selector. MAGIC32 is NOT in scudo:primary. v16 widens the scan to the **most-likely host region**: libnmsssa.so's writable data segment — that library implements the cert algorithm and is the natural cache for an install-keyed Google PGS player ID.

## Hypothesis (v16)
The captured selector `21bd3dc15046f910d7143353d60694de` (challenge=`176062C5A333E9E7`) was computed as `MD5(challenge || MAGIC32 || challenge)`. Scanning libnmsssa.so's rw data segment for 32-char ASCII hex strings and MD5-testing each against the known selector recovers MAGIC32. The library's writable data is the conceptually-correct cache for a once-fetched-forever-cached install constant.

## Falsification criteria (any one)
- One MD5 test matches → MAGIC32 identified. Patch cert-rust-repro constants; run pipeline on 13 wire pairs; ≥10/13 match → `cert_algorithm_end_to_end_validated_2026_05_18=true`, **goal closes 5/6 → 6/6**.
- All scanned hex candidates in libnmsssa rw → 0/N MD5 match. Pivot to v16 lane 2 (cert-emitting thread stack) or v16 lane 3 (scudo:secondary at 0x73fc08d000).
- All three lanes exhaust without MAGIC32 in 60 min → pivot to v16 lane 4 (streaming 1 MB chunk scanner over full 5.6 GB rw space, regex+MD5 per chunk).

## Hard rules
- **Workdir is the existing worktree** — don't `git worktree add`.
- **NO `pm clear`**.
- **read_mem returns 68 MB contiguous spans** regardless of requested size (per v15 observation) — use small per-region calls or anticipate the over-read.
- **adb is fragile under large reads** — recover with `adb kill-server && adb connect localhost:5558` (proven in v15). The until-loop watcher pattern works.
- **30-min wall cap** on the v16 lane 1 attempt. If lane 1 falsifies, move to lane 2 immediately.

## v16 Lane 1 — libnmsssa.so writable rw data
### Step 1 — locate libnmsssa rw region
1. `adb -s localhost:5558 shell "cat /proc/24114/maps" | grep -E "libnmsssa.*rw-p"` — find rw mapping for libnmsssa.so.
2. Record `start_va` and `length`.

### Step 2 — dump
1. Use existing `analysis/cert_hw_bp_v2/read_mem` tool to read the region into `/data/local/tmp/v16_libnmsssa_rw.bin`.
2. `adb pull` to `/tmp/v16_libnmsssa_rw.bin`.

### Step 3 — marker + brute scan
1. Re-use `analysis/cert_hw_bp_v15_brute_magic32.py` against the new dump.
2. First grep for any of: `21bd3dc1...`, challenge hex, cluster prefixes — if ANY appear, we're in the right region.
3. Extract all unique 32-char ASCII hex candidates. MD5 each. Compare to selector.

### Step 4 — outcomes
- **MATCH** → save MAGIC32 to `analysis/cert_hw_bp_v16_magic32_found.txt`. Patch cert-rust-repro. Run on 13 wire pairs.
- **NO MATCH but markers present** → pre-image lives nearby but not as ASCII hex. Re-encode candidates (base64, big-endian binary, little-endian). Refine.
- **NO MATCH no markers** → libnmsssa rw is not the cache. Move to lane 2 (cert-emitting thread stack — tid 21092 from v11 capture).

## v16 Lane 2 (fallback) — cert-emitting thread stack
1. `cat /proc/24114/task/21092/maps` — locate stack range.
2. Dump (likely <8 MB). Re-run brute scan.

## v16 Lane 3 (fallback) — scudo:secondary
1. scudo:secondary mapping at runtime VA ~0x73fc08d000 (19 MB per v15 verdict).
2. Dump + brute scan.

## v16 Lane 4 (last resort) — streaming chunk scanner
1. Read /proc/24114/mem in 1 MB increments per rw region.
2. Regex+MD5 each chunk, discard. Cover full 5.6 GB rw space without holding it in RAM.

## Step 5 — verdict + commit
Single commit on `cert-hw-bp`, message `cert HW BP v16: MAGIC32 search in libnmsssa rw — <verdict>`. Verdict in `analysis/cert_hw_bp_v16_verdict.md`. Final line `CERT_HW_BP_V16_DONE`. On goal closure, emit `analysis/cert_hw_bp_v16_clientless_repro.rs` and set `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`.

## Relevant files / references
- `analysis/cert_hw_bp_v15_verdict.md` — scudo:primary closure + v16 plan
- `analysis/cert_hw_bp_v15_brute_magic32.py` — reusable brute-test script
- `analysis/cert_hw_bp_v13_pipeline.py` — Python pipeline (5/5 emu validated)
- `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` — MAGIC32 constant for patching
- Ground truth: challenge=`176062C5A333E9E7`, selector=`21bd3dc15046f910d7143353d60694de`, Token=`BA00FE3EAB6937AD8183100FEB4D14B7B76C67EAD7C38D9C`
- 13 captured wire pairs in falsified.md / verdicts
- thered PID 24114 (current install). cert-emitting tid was 21092 in v11 capture (may differ this session — re-resolve).
