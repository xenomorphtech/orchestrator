# libue4-pktsystem-disasm — Recover the RzPktSystem outer framing (packet header + send pipeline)

## Role & workdir
Static-disasm worker. Workdir: `/home/sdancer/dark-december-libue4-pktsystem-disasm`.

## Current goal / sub-goal
- **goal_key**: `dark_december_rz_packet_framing_recovered` (new)
- **sub_goal_key**: `pktsystem-encode-pipeline`

## Why this turn exists
Prior path `libue4-rz-protocol` (cycle 324, GOAL MET) recovered the per-message-body wire format: each `FRz*::Serialize(IRzBuffer&)` emits the struct's fields via the IRzBuffer vtable (raw write @ vt[0x60], FString length-prefixed @ vt[0x50], nested type @ vt[0x68]). **But that's just the message BODY.** What's still missing for live capture/decode:

1. **Outer packet framing** — packet-type ID, total length, sequence number, any magic bytes.
2. **Encryption / obfuscation** — is the wire bit-identical to IRzBuffer output, or is there a XOR/AES layer? (Dark december's bootstrap traffic showed an "outer affine layer" — same may apply here.)
3. **Socket send pipeline** — which TCP socket, what fd, what's the WriteData/Send call site.
4. **Packet-ID → message-class dispatch table** — server side must know to decode N bytes as FRzDamageInfo vs FRzStatInfo.

`RzPktSystem` is the obvious entry. The rodata mine recovered `_ZN11RzPktSystem10RQAppGuardEv` (the request-send-AppGuard method). Disassembling it + immediate callees will reveal the encode pipeline.

## Hypothesis
`RzPktSystem` exposes a request-send method per packet type (e.g. `RQAppGuard`). Each request method (1) constructs the body struct (`CL_GS::FRz*Rq`), (2) serializes the body via IRzBuffer (already understood), (3) prepends a packet header (typically `uint16 packet_id` + `uint32 length` + maybe a `uint32 sequence`), (4) optionally applies an obfuscation/encryption transform, (5) writes to the connected TCP socket via a `Send`/`Write` method. The full pipeline is statically recoverable from `RzPktSystem::RQAppGuard` + 2 hops of callees.

## Falsification (3 clean outcomes)
- (a) **Pipeline fully recovered**: header layout + at least one packet-ID literal + encryption-or-not + socket fd source identified → SUCCESS. Fact: `dark_december_rz_packet_framing_decoded_<header_size>_<crypto>`.
- (b) **Pipeline reaches an opaque wrapper** (compiled-out function pointer, dynamic loader, or anticheat-injected jump) within 2 callee hops → document the wall + recommend live trace. Fact: `dark_december_rz_packet_framing_static_wall`.
- (c) **RzPktSystem::RQAppGuard does not actually emit packets** (e.g. it's a stub registered in a different system) → outcome rare; reconnaissance failure, document. Fact: `dark_december_rz_packet_framing_wrong_anchor`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-pktsystem-disasm/analysis/pktsystem_framing_2026-05-15.md` documenting:
1. **dynsym-resolved VA** for `RzPktSystem::RQAppGuard` + its called helpers (1-2 hops).
2. **Packet header layout** — every byte before the message body, with field semantics.
3. **Packet ID** — the literal `mov w*, #<id>` that identifies this packet type.
4. **Encryption / obfuscation status** — XOR? AES? mask table? Or pass-through?
5. **Socket send call site** — final write/send VA + which fd / handle is used.
6. **Recommendation** for a successor live-capture path (kernel-side uprobe target, or eBPF socket-write hook).
7. Verdict matched to (a)/(b)/(c).

**Closing fact**: see (a)/(b)/(c).

Print `PKTSYSTEM_DISASM_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Resolve VAs via dynsym ONLY (memory-bounded).**
The prior `libue4-rz-protocol` worker hit 20 GB RAM by loading the full 232 MB reconstructed ELF into pyelftools. **DO NOT REPEAT.** Instead:
```python
# Stream-parse only the dynsym + dynstr sections, not the full ELF.
from elftools.elf.elffile import ELFFile
with open('/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf','rb') as f:
    elf = ELFFile(f)
    ds = elf.get_section_by_name('.dynsym')
    if ds:
        for sym in ds.iter_symbols():
            n = sym.name or ''
            if ('RzPktSystem' in n or 'CL_GS::' in n or 'GS_CL::' in n
                or 'RzNetwork' in n or 'RzClient' in n or 'RzServer' in n
                or 'RzPkt' in n or 'Encrypt' in n or 'Cipher' in n
                or 'Socket' in n or 'TcpSocket' in n):
                print(f'0x{sym.entry.st_value:x}  size=0x{sym.entry.st_size:x}  {n}')
```
**HARD memory cap: 1 GB python.** The reconstructed ELF is 232 MB on disk but pyelftools mmaps it — keep usage bounded by NOT iterating all sections, just .dynsym + .dynstr.

**Step 2 — Disassemble `RzPktSystem::RQAppGuard` and immediate callees.**
- Compute file offset: `va - 0x6ce4bd4000` for VAs in the .text shard.
- Dump 16-32 KB windows per function via `dd bs=1 skip=$off count=N`.
- `aarch64-linux-gnu-objdump -D -b binary -m aarch64 --adjust-vma=<va>` per function.

**Step 3 — Trace the encode pipeline.**
Look for the call sequence:
1. Allocate IRzBuffer (might be on stack or heap).
2. Reserve header space (look for `mov ... #N` writes at offset 0 of the buffer).
3. Body Serialize call (call to `FRz*::Serialize`).
4. Header write-back (length, packet-id stored at front).
5. Encryption call (look for `bl <function with XOR/AES name>` or inline byte-by-byte transform loops).
6. Socket send (look for `bl <Send/Write/sendto>` near the end).

**Step 4 — Document the header layout.**
Expected fields (verify): `uint16 packet_id`, `uint32 length`, optional `uint32 sequence`, optional padding/magic.

**Step 5 — Identify encryption (if any).**
Patterns:
- XOR loop: `ldrb wN, [src,idx]; eor wN, wN, wKey; strb wN, [dst,idx]`.
- AES: call to a function with `aes_encrypt` / `EVP_EncryptUpdate` / `CryptoPP::AES` name.
- No encryption: data passes directly from IRzBuffer to socket send.

**Step 6 — Identify the socket fd / handle source.**
Trace back from the final `bl` to a `Send`/`Write` to find what was loaded into the fd argument.

**Step 7 — Write artifact + fact-set + print DONE.**
```bash
/home/sdancer/orchestrator/harness fact-set <fact_key> "<summary>"
echo PKTSYSTEM_DISASM_DONE
```

## Constraints & gotchas
- **HARD memory budget: 1 GB.** Stream-parse the ELF — do NOT load all sections / all symbols into memory at once. Filter as you iterate.
- **HARD output cap**: ≤1000 lines disasm in artifact. Carve per-function files into the worktree dir but reference, don't inline, the full disasms.
- **No Frida / no device interaction.** Static disasm only.
- **No call-graph traversal beyond 2 hops** from `RzPktSystem::RQAppGuard`. The encode pipeline should resolve within 2 levels of `bl`.
- **VA resolution rule**: always use dynsym for function entries (carried from prior paths).
- **One Codex turn budget: ≤2 hours wall time.**
- Honor `[[bulk-enumeration-needs-explicit-memory-budget]]` — prior path violated this and hit 20 GB peak. Don't repeat.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-pktsystem-disasm/`
- .text shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (97 MB, base VA 0x6ce4bd4000)
- Rodata shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin` (122 MB, base VA 0x6cdd243000)
- Reconstructed ELF (for dynsym, NOT full-load): `/home/sdancer/dark-december-libue4-jni-disasm/analysis/libUE4_reconstructed.elf` (232 MB — stream-parse only)
- Prior closure (read first): `/home/sdancer/dark-december-libue4-rz-protocol/analysis/rz_protocol_2026-05-15.md` (vtable model + 17 VAs)
- Rodata mine inventory: `/home/sdancer/dark-december-libue4-rodata-mine/analysis/rodata_inventory_2026-05-15.md`
- Anchor symbol (from rodata): `_ZN11RzPktSystem10RQAppGuardEv` — resolve to actual function VA via dynsym
- success-fact key: `dark_december_rz_packet_framing_decoded_<header_size>_<crypto>` (a)
- block-fact keys: `dark_december_rz_packet_framing_static_wall` (b), `dark_december_rz_packet_framing_wrong_anchor` (c)
