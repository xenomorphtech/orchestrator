# Live game vs snapshot-replay: two different cert code paths

**Confirmed**: cycle 633, 2026-05-02

## The two paths

### Path A: snapshot-replay (`aeon_jit_replay`)
- Run on-device, replays a captured trace
- Code path includes the writer cluster at `0x78620dad04..` (see `findings/algorithm/writer-cluster.md`)
- Produces deterministic outputs that the Rust reproducer matches
- All the campaign-final's "expected witnesses" came from this path

### Path B: live game (`com.netmarble.thered` + nmcore)
- The actual production cert-generation path
- Triggered by Java `nmss.app.NmssSa.nmssNativeGetCertValue(Activity, String)`
- **Does NOT touch the writer cluster** (verified: instrumented PCs fired 0 times during cycle 633 live capture)
- Different starting point and likely different intermediate primitives

## Architecture facts
- nmcore (`/data/data/com.netmarble.thered/files/9775cca1`) owns the cert UNIX socket `/files/nmss`
- Game spawns nmcore as a helper process; cmdline: `nmcore ser/0/com.netmarble.thered/files/9775cca1 <game-pid> /data/user/0/com.netmarble.thered/files/nmss 1`
- Heartbeat exchange via fd=4: write `'<pid>\0'`, read `'<pid>|unity|criticism'`
- nmcore exe is deleted from disk but loaded (anti-tamper)

## Live ground truth captured so far
| Challenge | native_cert | Source |
|---|---|---|
| 7BDA93D2F45D36C0 | `3763E9656BF1116EAB35AF137F59F72689ACFAD286EEB7AE` | cycle 633 frida_spawn_native_cert_probe (fluke, no longer reproducible — see walls/frida-spawn-probe-load.md) |
| 0000000000000000 | (unknown — pending) | |
| 0123456789ABCDEF | (unknown — pending) | |
| 1111111111111111 | (unknown — pending) | |
| AABBCCDDEEFF0011 | (unknown — pending) | |

## Stale "expected witnesses" (from snapshot path — DO NOT use to validate live)
| Challenge | snapshot witness |
|---|---|
| 0000000000000000 | `4E5150219BEB565F352A4FFF300F87841036F3C3A65E0B47` |
| 0123456789ABCDEF | `3083EFBA67F78ADBBD44EA727AF751E1F7380A33CC78B030` |
| 1111111111111111 | `3933F258F2EDF72D3CDD7E31D075D1F34AE73DF72D60A16B` |
| 7BDA93D2F45D36C0 | `90237F0E03DF6993A54669AA7CF27E36304273143AD6A030` |
| AABBCCDDEEFF0011 | `8F868A5849505353C39BA200827F07EA635A3F71D2DE812C` |

## JNI symbol offsets (live `libnmsssa.so`)
- `Java_nmss_app_NmssSa_nmssNativeGetCertValue` = `0x1400ec`
- `Java_nmss_app_NmssSa_nmssNativeGetVersion` = `0x141274`
