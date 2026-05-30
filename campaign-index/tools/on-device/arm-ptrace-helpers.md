# arm_ptrace_helper / arm_ptrace_helper_bp512 / arm_ptrace_lane_watch

15 KB ARM ptrace helpers under `/data/local/tmp/nmss/`. Used during the snapshot-replay campaign (May 02 01–03 cycles) to capture certobj_root/formatter_chain dumps from `aeon_jit_replay`.

## Status: snapshot-only — cannot be used on live game
- Live game (`com.netmarble.thered`) anti-debug kills any process that ptrace-attaches (see `walls/frida-attach.md` — same mechanism applies)
- These helpers are designed for `aeon_jit_replay` which does NOT have NMSS anti-debug

## Outputs they produced (for reference)
- certobj_root_*.json
- formatter_chain_*.json
Stored under `/data/local/tmp/nmss/` and pulled to host as needed. **These dumps are from the snapshot path; do not use them as live ground truth** — see `walls/snapshot-replay-path.md`.

## argv interface
Likely `--pid <pid> --bp <pc> --dump <reg>+<offset>:<len>` style. Confirm via `--help` or strings if needed for any future snapshot-replay work.
