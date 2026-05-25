# arm64 server (`nothing`)

Persistent Linux/ARM64 host available for native aarch64 work that can't run on Android (build farms, long-lived services, profiling jobs that need a real Linux kernel rather than bionic-on-Android).

## Connection

| Field | Value |
| --- | --- |
| IP | `162.244.80.97` |
| User | `root` |
| Auth | password (stored in `.env` as `ARM64_PASSWORD`, gitignored) |
| Hostname | `nothing` |

Connect with `sshpass` reading from `.env`:

```bash
set -a; source .env; set +a
SSHPASS="$ARM64_PASSWORD" sshpass -e ssh -o PreferredAuthentications=password \
  -o PubkeyAuthentication=no "$ARM64_USER@$ARM64_HOST"
```

Host key (ED25519) fingerprint as of registration: `SHA256:gsGDV4xyFNs24w0J+d//mw0mYjXPRWd09YTIXhNpGic`. The box was rebuilt at some point — if a prior `known_hosts` entry exists, remove it with `ssh-keygen -R 162.244.80.97` and re-accept.

## Hardware / OS (verified 2026-05-11)

- Arch: `aarch64`
- CPU: 8 cores, implementer `0x50` (Ampere / Applied Micro)
- RAM: 62 GiB
- Disk: 225 GB root, 1.2 GB used (essentially fresh)
- Kernel: `4.15.0-213-generic` (Ubuntu 18.04.3 LTS, Bionic — out of standard support, no kernel upgrades expected)
- Uptime at registration: 19h, 0 active users

## Toolchain status (fresh install — nothing pre-provisioned)

Present: `python3` (3.6.9).

Missing: `gcc`, `g++`, `make`, `git`, `tmux`, `rustc`, `cargo`, `adb`. Install via `apt-get install -y build-essential git tmux` then bootstrap `rustup` if Rust is needed.

## Intended use

Native aarch64 substrate for tasks that don't fit on the on-device Android ARM path (`arm_substrate_unblocked_2026_05_01`). Not a replacement for the device-side harness — use for cross-builds, replay tooling, and long-running services that want a real filesystem and 62 GiB of RAM.
