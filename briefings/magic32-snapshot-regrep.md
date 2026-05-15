# magic32-snapshot-regrep — Re-mine trampoline memdump with corrected PGS playerId format

## Role & workdir
Offline snapshot re-analyst. Workdir: `/home/sdancer/nmss-emu-magic32-snapshot-regrep` (worktree of `/home/sdancer/nmss-emu`, branch `magic32-snapshot-regrep`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro`.
- Sub-goal: re-search the existing `trampoline_proc_memdump_5558` snapshot for the PGS playerId using the **corrected** modern format `a_[0-9]{17,22}` (and other variants), then run the algorithm sweep with the recovered (playerId, MAGIC32) pair.

## Why this is distinct from the earlier `magic32-snapshot-mining` path

Prior path: `/home/sdancer/nmss-emu-magic32-snapshot-mining/` searched for `g[0-9]{17,22}` — the **legacy** Google+ Play Games format. Found 0 matches → falsified.

**New evidence** from sibling path `magic32-gms-state-read`:
- Modern PGS `external_player_id` format is `a_<19 digits>` (confirmed for `ktion23@gmail.com` = `a_1408633172786630918`).
- The 1706b671 GMS db schema column is `external_player_id` (not `original_player_id`).
- Multiple per-game keyed forms also exist: `a_<digits><append-bytes>` (e.g. `a_1408633172786630918136301552081`).

The original snapshot is from a DIFFERENT device (captured 2026-04-29) — that device's playerId/MAGIC32 pair is what was originally analyzed. If the snapshot was captured while the producer's state was still resident, the playerId-formatted-as-`a_<digits>` may be in heap.

This path is distinct because it uses a corrected/expanded regex and additional formats; the prior path's hypothesis "playerId is `g<digits>` and findable in snapshot" was falsified, this path's hypothesis is "playerId is `a_<digits>` and findable in snapshot" — different mechanism, different falsification criterion.

## Success criteria
Closing fact: `nmss_magic32_numerically_reproduced`. Concrete deliverable:
- (playerId, MAGIC32) pair extracted from the trampoline snapshot.
- AES-128 key derivation that produces `2FCF997702C244969BFEAF7F0D6AAA1C` from the snapshot's playerId.
- Pure-Rust `compute_magic32(player_id_str)` in cert-rust-repro + passing test.

## Progress so far (siblings)

Critical sibling artifacts:
- `/home/sdancer/nmss-emu-magic32-snapshot-mining/analysis/*.md` (closing artifact for the `g<digits>` search — 0 matches).
- `/home/sdancer/nmss-emu-magic32-gms-state-read/analysis/task1_gms_state_inventory_2026-05-14.md` — proves modern format is `a_<digits>`, schema column `external_player_id`.
- `/home/sdancer/nmss-emu-magic32-gms-state-read/analysis/task23_sweep_results_2026-05-14.json` — 22272 simple AES derivations against ktion23's player_id, 0 matches. The simple-hash hypothesis IS falsified for any playerId from gms-state — but a fresh playerId from the snapshot may unlock a different test.

## Next 2–3 concrete tasks

1. **Inventory snapshot shards.**
   - `ls /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/ | head -50`
   - Total size: `du -sh /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`
   - Map shards likely holding heap: anything that's `>10MB` and not `/system/...` lib backing.

2. **Re-grep with corrected formats** across all shards:
   ```bash
   rg -a 'a_[0-9]{17,25}' /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/ > /tmp/match_a_prefix.txt
   rg -a 'g[0-9]{17,25}' /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/ > /tmp/match_g_prefix.txt
   rg -a '"player_id":"[a-z_0-9]+"' /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/ > /tmp/match_json_pid.txt
   rg -a 'external_player_id[":][^,}]+' /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/ > /tmp/match_xpid.txt
   rg -a 'GooglePlayGames|com\.google\.android\.gms\.games' /home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/ > /tmp/match_pgs_ctx.txt
   ```
   - Take each distinct match. Dedupe. Capture file:offset for each candidate. Write the unique playerId list to `analysis/snapshot_playerid_candidates_2026-05-14.md`.
   - Also: try Korean/Chinese game ID formats — Netmarble may store the original PGS in their own DB format alongside.

3. **Algorithm sweep** for each candidate playerId.
   - For each (playerId_str, derived_byte_form):
     - `key = sha256(secret || playerId)[:16]` for secret in {empty, PGSClientSecret bytes from libUnreal rodata, ANDROID_ID `54c7b43cb642e8d3`, app_signing_cert hash} — borrow the same SECRETS list from gms-state-read's task23_sweep_results_2026-05-14.json.
     - Plaintext: utf8(playerId), utf8(playerId.split('_')[-1]), int(digits).to_bytes(16, LE/BE), MD5(playerId), zeropad-16, PKCS#7.
     - AES-128-ECB(key, pt) == `2FCF997702C244969BFEAF7F0D6AAA1C`? hit logged.
   - Also try AES-128-CBC zero-IV (sibling fact `dark_december_hive_http_keys_known_2026_05_15` shows Korean SDKs love zero-IV CBC).
   - Also try AES-128-CTR (per-block-counter).

4. **If algorithm matched**: implement `compute_magic32()` in `cert-rust-repro/src/magic32.rs`. Add test. `cargo test`. Set fact `nmss_magic32_numerically_reproduced`. Done.

5. **Bonus path: scan snapshot for the KEY directly.** Even without a known playerId, we can brute-force the snapshot for the AES key — for each 16-byte window in heap-class shards, try `AES-128-ECB(window, candidate_plaintext)` against MAGIC32. The plaintext space is small (~50 forms). Total: ~30M windows × 50 plaintexts = 1.5G AES ops. Time: ~1h on CPU. If a key is found, the surrounding context tells us what it is. Implement as a follow-on if step 2-4 falsifies.

6. **Write artifact** `analysis/snapshot_regrep_2026-05-14.md`:
   - Match counts per regex.
   - Unique playerId candidates (de-duplicated, with file:offset context).
   - Algorithm sweep results table.
   - Verdict.

## Constraints & gotchas

- **No device, no Frida, no NMSS interaction** — pure file-based offline.
- Some shards are very large (>100 MB). `rg -a` streams without loading full file into memory.
- The snapshot was captured ~2026-04-29 — playerId may be ephemeral. If it was never resident at snapshot time, no regex will find it.
- The trampoline snapshot's original device's Google account is UNKNOWN — but whatever playerId it has, the MAGIC32 `2FCF997702C244969BFEAF7F0D6AAA1C` is the same ciphertext (per prior magic32-disasm analysis chain). The playerId from THIS snapshot is the one that pairs with MAGIC32.
- This worker runs under systemd `harness-worker@magic32-snapshot-regrep.service` (system.slice, MemoryMax=24G).

## Relevant files / references

- Snapshot memdump: `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/memdump/`.
- Captured MAGIC32: `2FCF997702C244969BFEAF7F0D6AAA1C`.
- 14 rodata-block secret candidates (from apk-sweep): `/home/sdancer/nmss-emu-magic32-apk-strings-sweep/analysis/task2_broader_sweep_2026-05-14.md`.
- Symbol-string seeds (`PGSClientSecret`, `EncryptPlayerIdKey`, etc.): same artifact.
- gms-state-read sweep results: `/home/sdancer/nmss-emu-magic32-gms-state-read/analysis/task23_sweep_results_2026-05-14.json`.
- Hive sibling fact `dark_december_hive_http_keys_known_2026_05_15` shows the Hive AES idiom (timestamp-derived SHA1/SHA256 hex first-16-chars UTF-8 → AES-128-CBC zero-IV). The Netmarble SDK may use a similar idiom — test it.
- cert-rust-repro: `/home/sdancer/nmss-emu/cert-rust-repro/`.
- Tools: `rg`, `xxd`, `python3 (cryptography)`, `cargo`.

## Falsification

- Re-grep produces NO `a_[0-9]{17,25}` matches in any shard AND no JSON-encoded `player_id` strings → playerId is genuinely not resident at snapshot time.
- ≥10 playerId candidates tested across ≥1000 derivations × ≥50 plaintext forms → no AES match → derivation is even more complex than current sweep covers, OR the snapshot's MAGIC32 originated from a different code path than expected.
- The 1.5G-window brute-force (task 5) yields no match → MAGIC32's key is not 16 contiguous bytes anywhere in this snapshot's heap (key was ephemerally stack-only and gone by snapshot time).

If fully falsified, the kernel-uprobe path needs the PGS bootstrap fixed (paths `magic32-uprobes-fixed-pgs` or `magic32-uprobes-via-aion2` in backlog).
