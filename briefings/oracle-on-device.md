# oracle-on-device

## Role & workdir
You own the **on-device aarch64 replay lane** for the NMSS cert campaign: cross-compile `native_replay_ab` (and friends) from `native-replay-rs/` for `aarch64-linux-android`, push to the live Android device, and run replays on-device where ptrace works natively. You operate in an isolated git worktree at `/home/sdancer/nmss-emu-ondevice/` (branch `oracle-on-device`) so your changes never trample cert-ptrace's main-tree work.

## Goal / sub-goal
- Top-level goal: `nmss_cert_re_algorithmic` (produce certs matching donor for 7BDA93D2F45D36C0 + AABBCCDDEEFF0011).
- Lane: parallel to cert-ptrace's Frida live capture path. You unblock the case where Frida is offline or hooks don't fire.

## Success criteria
You succeed when ANY of:
1. `native_replay_ab` runs on-device, replays the trampoline_proc_memdump_5558 snapshot, and emits a captured sp+0x490 late-frag2 family for at least one of the 5 challenges (7BDA93D2F45D36C0, AABBCCDDEEFF0011, 0123456789ABCDEF, 0000000000000000, FFFFFFFFFFFFFFFF).
2. You produce evidence that this lane is structurally walled (e.g. ptrace blocked by Android SELinux, binary dies with specific signal whose root cause is unfixable on-device) — set fact `oracle_on_device_walled_with_<reason>`.

Set fact `oracle_on_device_first_capture` with path to the first captured JSON when (1) lands.

## Background
The trampoline_proc_memdump_5558 snapshot is a complete process memdump produced by a kernel module on the Android device (see `/home/sdancer/orchestrator/campaign-index/tools/on-device/INDEX.md`). `native_replay_ab` is the no-JIT replay binary in `native-replay-rs/src/bin/`, intended to replay that snapshot with ptrace-driven instrumentation. On the x86 dev host, qemu-user fails with `PTRACE_TRACEME ENOSYS` so the binary cannot run locally — but the Android device IS arm64, so cross-compiling and running on-device sidesteps that entirely.

**User correction (2026-05-02):** the prior strategic-options doc said "wait for arm64 device" — that was wrong framing. The Android device is the arm64 environment we need. ptrace works natively in Android's bionic; the only constraint is SELinux/permissions, which on-device tooling already operates around (see `arm_ptrace_helper` in the on-device INDEX).

## Progress so far
- Worktree created today (cycle 807) at `/home/sdancer/nmss-emu-ondevice/`, branch `oracle-on-device` from `main` (HEAD `c654108`).
- Nothing built yet — fresh start.
- Sibling lane (cert-ptrace, main worktree): updated Frida script with new PCs (0x15fc18/0x15fc4c/0x15fcb4) but blocked on adb/frida transport.
- Algorithmic reproducer at `cert-rust-repro/src/bin/cert_rust_repro.rs` already produces the first 16 bytes of the cert (the 8-byte tail is what we need late-frag2 captures for).

## Next 2-3 concrete tasks
1. **DONE — Toolchain survey**: NDK 27.2.12479018 selected at `/usr/lib/android-sdk/ndk/27.2.12479018/`, API 28, toolchain bin `toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android28-clang`. Cargo target block added to `native-replay-rs/.cargo/config.toml`. Checkpoint at `analysis/checkpoints/<your timestamp>` already records this.
2. **Build the binary**: `cd /home/sdancer/nmss-emu-ondevice/native-replay-rs && cargo build --release --target aarch64-linux-android` — the replay code lives in `src/main.rs` (NOT `src/bin/native_replay_ab.rs` — that path was wrong in an earlier briefing version; this worktree only has `src/main.rs`). Default bin name comes from the Cargo package. If unicorn or any other native dep fails to cross-compile for aarch64-linux-android, document the breakage in a checkpoint and try a smaller probe binary first (just ptrace + snapshot mmap reads) to validate the on-device pipeline before bringing the full replay in.
3. **Push and run** (DEVICE TRANSPORT WARNING — see below): `adb push target/aarch64-linux-android/release/<binname> /data/local/tmp/`. The snapshot may already be on-device under `/data/local/tmp/` — check before pushing the 100MB+ memdump; if not present, push it. Then `adb shell '/data/local/tmp/<binname> /data/local/tmp/trampoline_proc_memdump_5558 ...'`. Capture stdout/stderr to `analysis/checkpoints/oracle_on_device_<challenge>_2026-05-02.json` in your worktree.

**Device transport status (cycle 809):** adb to 127.0.0.1:5558 returns offline; frida 27042 connection refused. cert-ptrace tried `kill-server`/`start-server` and failed. This is a SHARED blocker between you and cert-ptrace — both paths converge on the same device. While device is down, focus on tasks 1–2 (build) so you're ready the moment transport recovers. Do NOT consume cycles polling adb; just attempt push when build is done, and if it fails, write a checkpoint and stop until next nudge.

## Constraints & gotchas
- **DO NOT modify files outside your worktree.** Specifically don't touch `/home/sdancer/nmss-emu/` directly — cert-ptrace owns that tree. If you need a file from main, copy or read it; don't push through.
- **DO NOT run destructive adb commands** (`adb reboot`, `pm uninstall`, etc.) without explicit orchestrator permission — the device is shared with cert-ptrace and any reset costs us the live-session state on which Frida captures depend.
- **DO NOT** assume `/tmp/libnmsssa.so` on the dev host matches what's on-device — ELF on the device is the authoritative copy for runtime PCs. cert-re-6 already published live ELF probe PCs at `analysis/checkpoints/live_libnmsssa_late_frag2_pcs_2026-05-02.json` — those are 0x15fc18/0x15fc4c/0x15fcb4 and you can reuse them.
- The snapshot (`trampoline_proc_memdump_5558`) is the kmod-produced memdump — see the kernel-module-dump-producer note. It contains protected dalvik mappings that userland tools can't replicate.
- ptrace on Android: `PTRACE_TRACEME` from a child of `adb shell` works; ptrace **across** unrelated processes may need root or a setuid helper. The device is rooted (xerda-server runs there).
- Any time your workdir context resets (compaction, restart), re-read this file before acting.

## Relevant files / references
- Worktree root: `/home/sdancer/nmss-emu-ondevice/`
- Replay binary source: `native-replay-rs/src/bin/native_replay_ab.rs`
- Snapshot (host copy): `/home/sdancer/nmss-emu/trampoline_proc_memdump_5558/` (read-only from your perspective)
- Algorithmic reproducer (16/24 bytes): `cert-rust-repro/src/bin/cert_rust_repro.rs`
- On-device tools index: `/home/sdancer/orchestrator/campaign-index/tools/on-device/INDEX.md`
- Live ELF PCs: `analysis/checkpoints/live_libnmsssa_late_frag2_pcs_2026-05-02.json`
- Sibling (cert-ptrace) status: `analysis/checkpoints/sp_0x490_late_frag2_live_retarget_status_2026-05-02.json`
- Fact keys to watch: `live_late_frag2_pcs_published_by_cert_re6`, `on_device_arm64_replay_lane_available`, `oracle_on_device_first_capture` (you set this).

## Reporting cadence
After each meaningful step (toolchain survey done / build attempt / push attempt / first run), write a checkpoint JSON in your worktree's `analysis/checkpoints/` directory and update the `oracle_on_device_*` facts. Don't wait until everything works — partial-progress checkpoints are valuable for the orchestrator.
