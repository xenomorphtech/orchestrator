# magic32-unicorn-bruteforce — find MAGIC32 in snapshot by direct verifier

## Role & workdir
Codex worker, workdir `/home/sdancer/nmss-emu-magic32-unicorn-bruteforce` (branch `magic32-unicorn-bruteforce` forked from 4204b43 = Lane J commit).

## Current goal / sub-goal
- **goal_key**: `nmss_clientless_fresh_login_replay` (5/6 → 6/6)
- **sub_goal_key**: `magic32-bruteforce-via-pure-rust-verifier`

## Why this path
The cert algorithm is **fully understood and verified 5/5 in pure Rust** (`/home/sdancer/nmss-emu/cert-rust-repro/`). The only unknown is the 32-byte MAGIC32 install constant. Lane I proved MAGIC32 lives in a region that's properly mapped in the snapshot. Lane J ruled out wrong-fptr.

**The reframe:** instead of searching for MAGIC32 by instrumentation, **enumerate all candidates from the snapshot bytes and verify each with the pure-Rust pipeline.** Each verification is sub-millisecond; if MAGIC32 is ASCII-hex 32-char in the snapshot anywhere, this finds it in O(N_candidates) seconds.

## Hypothesis
The live install's MAGIC32 appears as a 32-char `[0-9A-F]` (or `[0-9a-f]`) ASCII run somewhere in the 4.4 GB snapshot. Running cert-rust-repro with each candidate as MAGIC32 against captured challenge `A80CFDEF30357F22` produces a cert equal to expected Token `82B9E000299075AA2C2A070A370BFA01F24DE12B17A0189B` for exactly one candidate — that candidate IS the install's MAGIC32.

## Falsification criteria (any one)
- No ASCII-hex candidate in the snapshot produces the expected Token → MAGIC32 isn't stored as contiguous ASCII (next: expand to 16-byte binary forms via hex-encoding wrapper).
- A candidate matches and validates against ≥10/14 wire pairs → **MAGIC32 recovered, goal closes 5/6 → 6/6.**
- Build/run failure on the arm64 server (cert-rust-repro doesn't compile on aarch64-linux) → escalate.

## Hard rules
- **Server**: `root@162.244.80.97` (aarch64, 62 GB RAM, 44 GB disk free). DO NOT disturb `oracle-service` running on port 9876.
- **Snapshot source**: `adb -s localhost:5558 pull /data/local/tmp/live_snapshot_20260518_103654 → host → rsync to server`.
- **No device modifications.** Read-only on snapshot.
- **3h wall budget.** Direct path; if the grep-then-verify pipeline doesn't yield in 3h, expand candidate space.

## Step 1 — relocate snapshot to server
1. If not already there: `rsync -av --info=progress2 /home/sdancer/.../live_snapshot_20260518_103654/ root@162.244.80.97:/data/snapshots/live_20260518_103654/`. Check if a copy already exists on server (might be in `/data/snapshots/` or `/root/`).
2. Verify manifest + sample `.bin` SHA on both ends.

## Step 2 — extract candidates
1. SSH into 162.244.80.97. cd to snapshot directory.
2. `cat *.bin | grep -aoE '[0-9A-Fa-f]{32}' | sort -u > /tmp/magic32_candidates.txt`. Expect <1M unique strings.
3. Also extract 16-byte binary candidates: `cat *.bin | python3 -c 'import sys, binascii; data=sys.stdin.buffer.read(); ... extract 16-byte aligned windows and hex-encode'` → second candidate file.

## Step 3 — verify with pure-Rust pipeline
1. `git clone /home/sdancer/nmss-emu/cert-rust-repro to the server` (or rsync). It's in the parent repo nmss-emu — easiest is rsync the cert-rust-repro/ subdir.
2. Patch `src/native_oracle/stages/stage_two_step_sha256_cert.rs` to read MAGIC32 from `argv[1]` instead of hardcoded constant.
3. Build `cargo build --release` (verify aarch64-linux target compiles).
4. For each candidate in `magic32_candidates.txt`: run cert binary with `--magic32 <candidate> --challenge A80CFDEF30357F22 --expected 82B9E000299075AA2C2A070A370BFA01F24DE12B17A0189B`. Print MATCH if cert == expected. Use `parallel` or xargs for parallelism.
5. On match: emit the MAGIC32 value, validate against the other 13 captured wire pairs in `/home/sdancer/nmss-emu-cert-hw-bp/analysis/cert_hw_bp_v*_verdict.md`.

## Step 4 — verdict + commit
Single commit on `magic32-unicorn-bruteforce`. Verdict at `analysis/magic32_unicorn_bruteforce_verdict.md`. Final line `MAGIC32_UNICORN_BRUTEFORCE_DONE`. On success: emit MAGIC32, document substitution method for downstream consumers, set `nmss_clientless_fresh_login_replay_complete_2026_05_18=true`.

## Constraints & gotchas
- **cycle 1206 cert-hw-bp v17** already grep'd libnmsssa rw for `[0-9A-Fa-f]{32}` with 0 hits. This path extends scope to the FULL 4.4 GB snapshot (all dumped regions), which is a strictly larger search space.
- MAGIC32 is AES-encrypted PGS player ID per cert algorithm fact. If encrypted-at-rest, ASCII form might not exist anywhere — fall through to binary form sweep.
- Per fact `lane_g_committed_2026_05_18`: 14 wire pairs documented for validation in `/home/sdancer/nmss-emu-cert-hw-bp/analysis/`.
- Memory `oracle-service` already runs on 162.244.80.97:9876 — DO NOT touch its process.

## Relevant files / references
- `/home/sdancer/nmss-emu/cert-rust-repro/` — the verified 5/5 pure-Rust cert pipeline
- `/home/sdancer/nmss-emu/cert-rust-repro/src/native_oracle/stages/stage_two_step_sha256_cert.rs` — MAGIC32 constant location
- Snapshot: `/data/local/tmp/live_snapshot_20260518_103654/` on device, rsync target `/data/snapshots/live_20260518_103654/` on server
- Ground truth: challenge `A80CFDEF30357F22` → expected Token `82B9E000299075AA2C2A070A370BFA01F24DE12B17A0189B`
- Memory: `feedback_no_frida` (no Frida on libUnreal), `proc_mem_read_substrate_validated_nmss_undetected_2026_05_18`
