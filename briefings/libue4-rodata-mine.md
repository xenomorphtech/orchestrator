# libue4-rodata-mine — Inventory the decrypted libUE4 rodata for protocol/crypto/JNI surfaces

## Role & workdir
Offline string/byte miner. Workdir: `/home/sdancer/dark-december-libue4-rodata-mine`.

## Current goal / sub-goal
- **goal_key**: `dark_december_libue4_protocol_surface_inventoried` (new)
- **sub_goal_key**: `rodata-mine-inventory`

## Why this turn exists
The libUE4 decrypted dump landed prior cycle: 3 shards at `/home/sdancer/dark-december-libue4-memdump/memdump/` (rodata 122 MB, .text 97 MB, data 14 MB). Combined sha256 `189e2144...c022f6bb69`. Before launching expensive disasm on the 97 MB .text, we want a cheap structured inventory of the rodata so subsequent paths can target hot anchors (URLs, JNI symbols, crypto strings, message-type tables) instead of doing full-binary closures.

## Hypothesis
The 122 MB rodata shard (`memdump/6cdd243000.bin`, virt base 0x6cdd243000) contains plaintext anchors for: (a) UE4 internal symbols + the game's C++ symbols (mangled), (b) JNI bridge method names (`Java_com_*`), (c) HTTPS/protocol URL literals (api hosts, route paths), (d) crypto identifiers (`RSA`, `AES`, `JWT`, key names, OID strings), (e) message-type tables for the network protocol. Mining these gives us a sub-100-line inventory that picks the top-3 disasm anchors.

## Falsification (3 clean outcomes)
- (a) **Inventory yields ≥3 actionable disasm anchors** (e.g. a HTTPS URL, a JNI entry, a crypto routine name) with byte-offsets → SUCCESS. Fact: `dark_december_libue4_rodata_inventory_completed_<n>_anchors`.
- (b) **Inventory yields only generic UE4 strings**, no game-specific protocol/crypto/JNI surfaces → mine is too coarse OR rodata holds only UE4 boilerplate. Fact: `dark_december_libue4_rodata_only_ue4_boilerplate`.
- (c) **Shard cannot be parsed** (corrupt / partial) → fall back to `.text` direct mine. Fact: `dark_december_libue4_rodata_unreadable`.

## Success criteria
**Primary**: write `/home/sdancer/dark-december-libue4-rodata-mine/analysis/rodata_inventory_2026-05-15.md` with:
1. **JNI entries** — every `Java_com_*` and `Java_org_*` symbol with file offset + virt address (= base + offset).
2. **URLs / hostnames** — every full URL, partial URL path (`/api/*`, `/v1/*`), and bare hostname literal.
3. **Crypto identifiers** — `RSA`, `AES`, `HMAC`, `SHA`, `JWT`, `PEM`, `BEGIN CERTIFICATE`, OID strings (`1.2.840...`), key-name fragments.
4. **Top 3 recommended disasm anchors** — for each, give: (a) the string, (b) virt address, (c) one-line rationale for why disasm starting there yields high-value (protocol entry, crypto setup, key load).
5. **Bytes-level surprises** — any unexpected high-entropy region, repeated magic constants, ELF/PE/ZIP signatures embedded in rodata.

**Closing fact**: `dark_december_libue4_rodata_inventory_completed_<n>_anchors` (a) where `<n>` is the count of distinct actionable disasm anchors.

Print `RODATA_MINE_DONE` on the final line.

## Execution flow — atomic, single Codex turn

**Step 1 — Validate input + extract baseline strings.**
```bash
mkdir -p analysis
SHARD=/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin
ls -lah $SHARD   # expect 122 MB
sha256sum $SHARD  # expect 7d4d7059ee7775b0b7e13e1e2b1c8129b9028924b5c4d551236541b899c4f47f
strings -n 8 $SHARD > analysis/all_strings.txt
wc -l analysis/all_strings.txt
```

**Step 2 — Targeted filters with byte offsets (use `-t d` or `-t x` for offset).**
```bash
# JNI entries
strings -n 12 -t x $SHARD | grep -E '^[0-9a-f]+ Java_[a-zA-Z0-9_]+$' > analysis/jni_symbols.txt
wc -l analysis/jni_symbols.txt

# URLs
strings -n 8 -t x $SHARD | grep -E '^[0-9a-f]+ https?://' > analysis/urls.txt
# Partial paths
strings -n 4 -t x $SHARD | grep -E '^[0-9a-f]+ /(api|v[0-9]+|sso|auth|login|user|game|account|cert)/[a-zA-Z]' > analysis/api_paths.txt

# Crypto
strings -n 6 -t x $SHARD | grep -iE 'RSA|AES|HMAC|SHA-?(1|256|512)|JWT|JWS|JWE|PEM|BEGIN CERTIFICATE|PKCS|x509|ECDSA|ED25519' > analysis/crypto.txt

# Hostnames (xxx.yyy domain pattern)
strings -n 6 -t x $SHARD | grep -E '^[0-9a-f]+ [a-z0-9-]+\.[a-z0-9-]+\.(com|net|org|io|kr|jp|cn)\b' > analysis/hosts.txt
```

**Step 3 — Compute virt addresses.**
```bash
# virt = file_offset + 0x6cdd243000 (the base address from maps)
python3 -c "
import sys
base = 0x6cdd243000
for fname in ['analysis/jni_symbols.txt','analysis/urls.txt','analysis/api_paths.txt','analysis/crypto.txt','analysis/hosts.txt']:
    with open(fname) as f: lines = f.readlines()
    out = []
    for L in lines:
        parts = L.strip().split(None,1)
        if len(parts) < 2: continue
        try: off = int(parts[0],16); va = base+off
        except: continue
        out.append(f'{off:08x}  va=0x{va:x}  {parts[1]}')
    with open(fname+'.va','w') as f: f.write('\n'.join(out))
    print(fname,'->',len(out),'entries')
"
```

**Step 4 — Magic-constants + embedded blob scan.**
```bash
# ELF/PE/ZIP/PNG magics that shouldn't be in rodata
python3 << 'PY'
import re
sig = {b'\x7fELF':'ELF', b'MZ':'PE', b'PK\x03\x04':'ZIP', b'\x89PNG':'PNG', b'-----BEGIN':'PEM'}
found = []
with open('/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin','rb') as f:
    data = f.read()
for s,n in sig.items():
    for m in re.finditer(re.escape(s), data):
        found.append((m.start(), n, data[m.start():m.start()+64]))
        if len(found) > 50: break
for off,n,blob in found[:50]:
    print(f'{off:08x}  {n}  {blob[:48].hex()}')
PY
```
**MEMORY BOUND: 1 read of 122 MB rodata ≈ 122 MB python heap. Hard cap 4 GB total. Don't slurp .text or data shards in same script.**

**Step 5 — Synthesize the inventory + top-3 anchors + fact-set.**

**Step 6 — Print `RODATA_MINE_DONE`.**

## Constraints & gotchas
- **HARD memory budget: 4 GB.** No simultaneous slurp of multiple shards. The 122 MB rodata can be fully loaded once; .text shard must stay streamed.
- **HARD output cap**: per-category file ≤10K lines. If `analysis/all_strings.txt` exceeds 2M lines, sample to first 1M.
- **NO Frida / NO device interaction.** Pure offline static work on the dumped shards.
- **NO disasm in this path** — that's the next path's job. Output is purely the inventory.
- **One Codex turn budget: ≤1 hour wall time.**
- Honor memory rule `[[bulk-enumeration-needs-explicit-memory-budget]]`.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-libue4-rodata-mine/`
- Source shard (read-only): `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin`
- Sister shards (do NOT load): `6ce4bd4000.bin` (.text 97 MB), `6ceac63000.bin` (.data 14 MB)
- Prior closure artifact: `/home/sdancer/dark-december-libue4-memdump/analysis/libue4_memdump_2026-05-15.md` — confirms UE4 plaintext anchors (`GUObjectArray`, `_ZN11FEngineLoop4InitEv`, `Java_com_epicgames_ue4_GameActivity_nativeSetObbFilePaths`)
- success-fact key: `dark_december_libue4_rodata_inventory_completed_<n>_anchors` (a)
- block-fact keys: `dark_december_libue4_rodata_only_ue4_boilerplate` (b), `dark_december_libue4_rodata_unreadable` (c)
