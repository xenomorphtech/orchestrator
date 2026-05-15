# magic32-hw-bp — HW-breakpoint capture of MAGIC32 AES key at runtime

## Role & workdir
Live-device dynamic analyst. Workdir: `/home/sdancer/nmss-emu-magic32-hw-bp` (worktree of `nmss-emu`, branch `magic32-hw-bp`).

## Current goal / sub-goal
- Goal: `nmss_magic32_numerical_repro` — reproduce MAGIC32 offline from Google PGS player ID.
- Sub-goal (this path): **Capture the AES-128 key bytes at runtime via HW-BP**, sidestepping the static-disasm chain-walk that has stalled on the sibling `magic32-disasm` path.

## Success criteria
Closing fact: `nmss_magic32_numerically_reproduced` (shared with sibling path). Concrete deliverable for THIS path:
- Captured (plaintext_PGS_player_id, AES_key_bytes, ciphertext_MAGIC32) triple from a live `com.netmarble.thered` process.
- AES-128 mode confirmed (ECB / CBC / etc).
- Pure-Rust `compute_magic32(pgs_player_id_bytes: &[u8]) -> [u8; 16]` in `cert-rust-repro` that produces `2FCF997702C244969BFEAF7F0D6AAA1C` from the captured player_id.

## Progress so far (from sibling magic32-disasm — its key facts inform your HW-BP placement)

- Producer JNI: `Java_com_epicgames_unreal_GameActivity_OnGetPGSPlayerIdWithAuthCode` in libUnreal.so.
- AES site narrowed (by magic32-disasm) to one of two candidate PCs: **`0x195b9f8`** and **`0x195be04`** — confirmed by parallel magic32-apk-strings-sweep (now retired) as pure `AESE`/`AESIMC` transforms over caller-provided state with no rodata key pointers. Key is runtime-derived.
- The 0x30-byte heap work item submitted via dispatcher `0x17fbc7c` carries field `googleClientSecret` at payload offset `+0x78` (sourced from context `[x26 + 0x80]` in serializer `0x57f6900`).
- Dispatch chain pinned: `0x58069f8 → 0x5806a1c → 0x57f6900` then `0xa8` payload at `0x8ccf150` wrapped via vtable `0x8cd0458` to async worker `0x4af4c10`.
- Static `magic32-apk-strings-sweep` definitively ruled out a baked AES key (artifacts at `/home/sdancer/nmss-emu-magic32-apk-strings-sweep/analysis/task1_apk_strings_sweep_2026-05-14.md` + `task2_broader_sweep_2026-05-14.md`).
- Prior cert-builder-hw-bp campaign used `NT_ARM_HW_BREAK` via ptrace on aarch64 with success — same harness applies here.

## Next 2–3 concrete tasks

1. **Locate the running process and the AES PCs.**
   - `adb connect localhost:5558` and `adb shell pidof com.netmarble.thered` to get the live PID.
   - Get libUnreal.so's load base for that process: `adb shell cat /proc/<pid>/maps | grep libUnreal.so` — first column is load_base.
   - Compute runtime PCs: `aes_pc_1 = load_base + 0x195b9f8`, `aes_pc_2 = load_base + 0x195be04`.

2. **Set HW-BP at both candidate AES PCs via ptrace.**
   - Pattern reference: `/home/sdancer/nmss-emu-const32b-hwbp/` has a working `find_srcobj.c` and the HW-BP-via-NT_ARM_HW_BREAK invocation pattern. Use that as the skeleton.
   - Two breakpoints (DR0, DR1 equivalents on aarch64) — one per candidate.
   - At hit: snapshot `x0–x4`, plus the 64-byte regions around the addresses they point to (this is where AES key + plaintext+ciphertext buffers live for a typical AES-128-ECB single-block call).
   - Trigger the producer: typically firing on app launch (login flow). May need a fresh app launch via `adb shell am force-stop com.netmarble.thered && am start -n com.netmarble.thered/.MainActivity` (adjust activity name from `aapt dump badging`).

3. **Decode the captured state into key + plaintext + ciphertext.**
   - The AES key is one of {x0, x1, x2} when calling an AES helper — read the literature on AArch64 calling convention + Unreal's typical AES helper signature (key, in, out).
   - Compare the captured ciphertext bytes (in raw 16-byte form, NOT hex) against the bytes that hex-decode from MAGIC32 = `2FCF997702C244969BFEAF7F0D6AAA1C` (i.e., `0x2FCF9977 0x02C24496 0x9BFEAF7F 0x0D6AAA1C` little-endian or big-endian — try both). That confirms which call is the right one.
   - Compare the captured plaintext bytes against the player_id sources (might be UTF-16 encoded — NMSS uses GetStringChars which produces UTF-16).

4. **Verify offline.** Write `compute_magic32(pgs_player_id_bytes: &[u8]) -> [u8; 16]` in pure Rust (use the `aes` crate). Test: encrypt the captured player_id bytes with the captured key, assert output equals captured ciphertext. If success → set fact `nmss_magic32_numerically_reproduced` and write closing artifact.

## Constraints & gotchas

- **No Frida on libUnreal.so** (memory rule — anticheat). Pure ptrace + HW-BP.
- The `cert-builder-hw-bp` and `const32b-hwbp` campaigns already established working aarch64 HW-BP infrastructure. Reuse those.
- Producer fires **once on PGS login** — may need a fresh process attach + force-stop/restart. Be prepared to re-attach quickly.
- This worker runs under systemd `harness-worker@magic32-hw-bp.service` in `system.slice` with MemoryMax=24G — won't take down the box on aeon misuse.
- adb may briefly disconnect on Waydroid restart; tolerate that (idempotent `adb connect localhost:5558`).
- This is a **PARALLEL HEDGE** to magic32-disasm (which is at-risk after 4 cycles in deep query mode). If either path closes, the other retires naturally.

## Relevant files / references

- Working HW-BP skeleton: `/home/sdancer/nmss-emu-const32b-hwbp/` (especially `find_srcobj.c`).
- Producer JNI body decomp: `/home/sdancer/nmss-emu-magic32-disasm/analysis/task1_jni_body_2026-05-13.md`.
- Target ELF (for symbol-table cross-ref): `/home/sdancer/tmp/nmss_apk/extract/lib/arm64-v8a/libUnreal.so`.
- Sibling path's chain dump (in paths.json): see `task2c_*` and `task2d_progress_*` entries under `nmss_magic32_numerical_repro` → `magic32-disasm`.
- Captured ground truth: MAGIC32 = `2FCF997702C244969BFEAF7F0D6AAA1C` (hex_upper).
- WIKI: `/home/sdancer/nmss-emu/WIKI.md`.

## Falsification

This path is killed if:
- Both candidate AES PCs (`0x195b9f8`, `0x195be04`) are never hit during 3 fresh-launch attempts (PGS login flow either uses a different code path or anticheat blocks ptrace).
- Hit but x0/x1/x2 don't contain plausible AES key (e.g., point to unmapped memory, or contain ASCII strings) — implies the AES helper signature differs from what we expect.
- adb / Waydroid persistently unavailable for the entire cycle.

If falsified, retire and let the planner propose another vector (e.g., Frida-on-Java to capture player_id input + reading post-AES output from sharedprefs).
