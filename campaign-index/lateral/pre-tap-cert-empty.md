# Lateral: bypass "live cert returns empty pre-tap"

**Wall** (implicit): cycle 657 confirmed Frida-spawn pre-title-tap returns empty cert from `nmssNativeGetCertValue` for all 5 challenges. Cycle 633 fluke (7BDA → 3763E965) likely came from a transient post-tap window.

## Experiments

1. `[ ]` **Inject auth state from a captured authenticated session** — dump the relevant nmcore/game memory regions (session token, login state) from a known-good post-login session, restore them into a fresh spawn before calling getcert. Cost: significant — requires identifying the auth state surface first.

2. `[ ]` **Patch the cert-time auth check** — find the branch in libnmsssa.so that returns empty when not authenticated, NOP it. Then getcert should always return a result. Cost: half-day RE; risk: returns garbage if the check ALSO controls the algorithm path.

3. `[ ]` **Snapshot of post-login process state** — once user does manual tap and reaches in-game world, freeze the process via SIGSTOP + dump full /proc/<pid>/mem to a checkpoint file. Replay by restoring memory + resuming. Cost: heavy tooling but creates a reproducible snapshot for repeated cert calls.

4. `[ ]` **Get past tap programmatically (sendevent path)** — see `lateral/adb-input-tap.md` experiment 1. If we can dismiss the title gate via `sendevent`, the auth state will populate naturally and cert calls will return real data. Cheapest path. Cost: ~15 min.

5. `[ ]` **Replay nmcore protocol from authenticated socket capture** — if the user can do one authenticated session and we tcpdump/strace the file socket exchange, we can replay it offline against a fresh nmcore. Cost: 1-2 hour RE of the protocol shape.

## Recommended next: experiment 4 (sendevent past title) is the cheapest unblock and reuses the lateral work for `adb-input-tap.md`.
