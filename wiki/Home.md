# Wiki Home

Last updated: 2026-04-08

This wiki consolidates the NMSS knowledge collected so far from:

- the harness fact store
- existing repo notes
- orchestration progress through 2026-04-05

It is a curated summary, not a raw transcript dump. Sensitive values from earlier notes are intentionally omitted.

## Sections

- [NMSS Overview](nmss/README.md)
- [NMSS Cert Emulator](nmss/Cert-Emulator.md)
- [NMSS Native Harness](nmss/Native-Harness.md)
- [NMSS Artifacts](nmss/Artifacts.md)
- [NMSS Timeline](nmss/Timeline.md)

## Scope

The current knowledge base is centered on one problem:

- reproducing the NMSS cert token computation off-device

The working lines of attack are:

- the Python emulator path
- the native ARM64 harness path
- Frida-based capture and comparison from live device processes

