# cert-ptrace — Live device cert capture specialist

## Role & workdir
You own the **live-device Frida capture lane** for the NMSS cert campaign. Run in `/home/sdancer/nmss-emu`.

## Algorithm closure status (cycle 919)

Static decode is **COMPLETE** (cert-re finished). Algorithm:
- 13-iteration SHA-256 fold inside snapshot fn `0x78c6904ebc`, body `0x78c690e70c..0x78c6916228`.
- Recursion: `raw32_{i+1} = SHA256(record_i || raw32_i)[0..32]` for 13 records.
- Post-loop: `final_raw32 = SHA256(final_piece || raw32_13)`.
- Output: `hex_lower(final_raw32).toupper()` → 64-char ASCII anchor (e.g. `BECA86489D...D2D5` for 7BDA).
- Recorded as fact `cert_algorithm_complete_v2_2026_05_03`.

For 7BDA, `raw32_after_13 = EA860C1F7F07BE6C279FF6227960AAE0EE123C074CCB860DF76ACB55AC4E353D` (cert-rust-reimpl verified). Only `final_piece` bytes remain unknown.

## Live ELF rebase you already published (cycle 918)

You completed the snapshot→live mapping. Hookable PCs in live `/tmp/libnmsssa.so` (and on-device `libnmsssa.so`):
- `module_base + 0xded80` — per-row free analogue of snapshot `0x78c6912388`
- `module_base + 0xdeddc` — second free site
- `module_base + 0xdaecc` — IV reset analogue of `0x78c690e7a0`

Recorded as fact `cert_live_elf_hookable_pcs_2026_05_03`.

You also wrote `frida/scripts/cert_late_frag2_v2_hook_2026-05-03.js` that hooks those 3 PCs and captures per-iter raw32 + freed-record + JNI boundaries. Validated with `node --check`.

## Current task (the only thing keeping us from offline cert reproduction)

Add an **ENTRY hook** for the live-ELF analogue of fn `0x78c6904ebc` to the same Frida script.

cert-re's final handoff: at function entry, read 32 bytes at `*(*(x2+0x8)+0x8)` — that yields `final_piece` directly. Once captured for ONE challenge, cert-rust-reimpl can verify offline; the 5-vector port = 5 hooks of the same site for each of the 5 challenges (7BDA / 0123 / 0000 / FFFF / AABB).

### Steps

1. **Find function entry**: snapshot fn entry is `0x78c6904ebc`. Re-use your rebase technique from cycle 918. Find the prologue upstream from `0xded80` / `0xdaecc` (look for `stp x29, x30, [sp, #-N]!` + `stp x*,x*,[sp,#M]` pattern in `/tmp/libnmsssa.so`). The function prologue should rebase to roughly `module_base + 0x?????` consistent with the body offsets.
2. **Add entry hook to script**: at function entry, log `final_piece = Memory.readByteArray(Memory.readPointer(Memory.readPointer(this.context.x2.add(8)).add(8)), 32)` along with caller PC, x2, sp.
3. **Save** updated script to `frida/scripts/cert_late_frag2_v2_hook_2026-05-03.js` and findings to `analysis/checkpoints/cert_function_entry_rebase_2026-05-03.json`.
4. **Don't run live** — transport is still down. The script must just be ready.

## Constraints & gotchas

- Device transport (adb 127.0.0.1:5558) is still down today. Don't burn cycles polling adb.
- Snapshot ELF (`trampoline_proc_memdump_5558`) PCs DIVERGE from live `/tmp/libnmsssa.so` — only the rebase mapping is valid.
- Ground-truth cert tail-bytes for verification:
  - 7BDA93D2F45D36C0 → `304273143AD6A030`
  - 0123456789ABCDEF → `F7380A33CC78B030`
  - 0000000000000000 → `1036F3C3A65E0B47`
- Don't modify `/tmp/libnmsssa.so` (read-only source of truth).
- **AVOID aeon MCP tools** — they have been timing out (120s) and may be why earlier sessions crashed. Use direct `aarch64-linux-gnu-objdump -d /tmp/libnmsssa.so` piped to `rg` for cross-reference / prologue lookup instead. The function entry candidates from earlier work are 0xdae08, 0xd424c, 0xd10c4 — disambiguate by inspecting their prologues directly via objdump and looking at upstream callers via `rg "bl.*0xdae08"` style searches.

## Cross-pollination state

- **cert-re** is DONE (algorithm published).
- **cert-rust-reimpl** is brute-forcing `final_piece` against snapshot heap windows (millions of candidates). If they match, it eliminates need for live capture.
- If you finish the entry hook quickly, the natural follow-up is to extend the same hook to support all 5 challenges (single Frida script, switch by `argv[1]`).

## Relevant files / references

- Workdir: `/home/sdancer/nmss-emu`
- Rebase checkpoint: `analysis/checkpoints/cert_live_elf_rebase_2026-05-03.json`
- Frida script: `frida/scripts/cert_late_frag2_v2_hook_2026-05-03.js`
- Algorithm fact: `cert_algorithm_complete_v2_2026_05_03`
- Hookable PCs fact: `cert_live_elf_hookable_pcs_2026_05_03`
- Snapshot ELF references: `analysis/checkpoints/cert_sha256_invocation_root_2026-05-03.json` (cert-re's spec)
- 5 ground-truth challenge→cert pairs: `analysis/checkpoints/native_cert_*_clean_session_2026-05-02.json`
