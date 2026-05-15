# magic32-ssl-uprobe-sniff — Find SSL_write entry in libUnreal.so via adrp+add xref, install uprobe, capture

## ONE TASK
The string `SSL_write` is in libUnreal.so rodata. The BoringSSL function that references it as its `__FUNCTION__` literal IS SSL_write itself. Find the rodata offset of the literal, find the adrp+add xref in .text, that's SSL_write's entry. Install a uprobe on the runtime-loaded libUnreal.so at that offset, capture 30s of plaintext, write artifact, set fact. **ONE END-TO-END SCRIPT — do not stop mid-way.**

## Known state (do NOT redo)
- thered runtime PID was 28677 (may have changed; refresh).
- Runtime libUnreal path: `/data/app/~~cGPM14AP6lPy6m9_u22NmA==/com.netmarble.thered-6XSqEE8z5mpf3XJtmS-BlQ==/lib/arm64/libUnreal.so`.
- Runtime exec base: `0x6d081fc000`.
- BoringSSL strings confirmed present in rodata: `SSL_write_ex`, `SSL_read_ex`, `ssl_write_internal`, `SSL_write`.
- Host reference copy: `/home/sdancer/aeon/libUnreal.so` (157.5 MB; symbols are stripped from the dynsym but strings are in rodata).

## SCRIPT to execute

```bash
set -e
WORKDIR=/home/sdancer/nmss-emu-magic32-ssl-uprobe-sniff
cd "$WORKDIR"
HOST_LIB=/home/sdancer/aeon/libUnreal.so

# Step 1: find rodata offset of the literal "SSL_write"
# strings -a -tx prints "OFFSET STRING" pairs (offset in hex)
SSL_WRITE_STR_OFF=$(aarch64-linux-gnu-strings -a -tx "$HOST_LIB" 2>/dev/null | awk '$2=="SSL_write"{print "0x"$1; exit}')
echo "SSL_write literal at rodata offset: $SSL_WRITE_STR_OFF"

# Verify it's actually rodata, not random match
if [ -z "$SSL_WRITE_STR_OFF" ]; then echo "FATAL: no SSL_write literal found in $HOST_LIB"; exit 1; fi

# Step 2: find an adrp+add instruction pair in .text that materializes the address SSL_WRITE_STR_OFF
# Need libUnreal load base from ELF program headers
SSL_WRITE_VA=$(python3 -c "
import struct
with open('$HOST_LIB', 'rb') as f:
    f.seek(0); _, _, _, _, e_phoff, _, _, _, _, e_phentsize, e_phnum = struct.unpack('<16sHHIQQQIHHH', f.read(0x40))
    f.seek(e_phoff)
    text_vaddr = None
    for _ in range(e_phnum):
        ph = f.read(e_phentsize)
        p_type, p_flags, p_offset, p_vaddr, _, p_filesz, _, _ = struct.unpack('<IIQQQQQQ', ph)
        if p_type == 1 and (p_flags & 4):  # PT_LOAD, readable. Find segment containing $SSL_WRITE_STR_OFF
            if p_offset <= $SSL_WRITE_STR_OFF < p_offset + p_filesz:
                print(hex(p_vaddr + ($SSL_WRITE_STR_OFF - p_offset)))
                break
")
echo "SSL_write literal VA in libUnreal: $SSL_WRITE_VA"

# Step 3: scan the file's .text via objdump for adrp/add pair that lands on SSL_WRITE_VA
# This is a search — find within ~3MB of recent text where the literal would be referenced
# Heavy: full objdump of 158MB ELF would be huge. Cheaper: use python + capstone if available.
# Fallback: search for the specific encoding pattern.
# But simplest: there are tools that do "find xref to address". Try llvm-readobj or just objdump with grep:

# Quick approach: dump .text section, search for instruction patterns referencing the VA
# adrp rd, page(addr); add rd, rd, offset(addr) — the literal will be encoded in those two insns
# We extract the page (high 33 bits) and the page offset (low 12 bits)
PAGE_VA=$(printf '0x%x' $((SSL_WRITE_VA & ~0xfff)))
PAGE_OFF=$(printf '0x%x' $((SSL_WRITE_VA & 0xfff)))
echo "adrp page target: $PAGE_VA, add immediate: $PAGE_OFF"

# Step 4: bounded objdump search — disassemble .text section, find adrp pairs near the right page
# This is the expensive step but unavoidable for stripped libUnreal. Constrain to .text.

aarch64-linux-gnu-objdump -d -j .text "$HOST_LIB" 2>/dev/null > /tmp/libunreal_text.S
TEXT_LINES=$(wc -l /tmp/libunreal_text.S | awk '{print $1}')
echo "disassembled .text: $TEXT_LINES lines (~$(du -sh /tmp/libunreal_text.S | awk '{print $1}'))"

# grep for adrp instructions targeting the page of SSL_write literal
# adrp output format: "  abc123: 90000080  adrp x0, 0xRESULT"
PAGE_HEX=$(printf '%x' $((SSL_WRITE_VA & ~0xfff)))
echo "searching for adrp pairs to page 0x$PAGE_HEX"
grep -nE "adrp\s+x[0-9]+,\s*0x${PAGE_HEX}\$" /tmp/libunreal_text.S | head -10

# Each match's NEXT line should be "add Xd, Xd, #PAGE_OFF" or "ldr Xd, [Xd, #PAGE_OFF]"
# Capture line numbers; the function containing the FIRST adrp pair is likely SSL_write
ADRP_LINE=$(grep -nE "adrp\s+x[0-9]+,\s*0x${PAGE_HEX}\$" /tmp/libunreal_text.S | head -1 | cut -d: -f1)
if [ -z "$ADRP_LINE" ]; then
  echo "FATAL: no adrp xref to SSL_write page found"
  /home/sdancer/orchestrator/harness fact-set magic32_ssl_xref_not_found "Searched for adrp xref to page $PAGE_VA containing SSL_write literal in libUnreal.so .text — no matches. Fallback to LKM injection per standing rule."
  exit 0
fi
echo "first adrp match at .S line $ADRP_LINE"

# Walk backwards from the adrp line to find the containing function entry (look for sub sp,sp pattern or function label)
awk -v L=$ADRP_LINE 'NR<=L{lines[NR]=$0} END {for(i=L;i>L-200;i--){if(lines[i] ~ /^[0-9a-f]+ <[^>]+>:/){print i": "lines[i]; exit}}}' /tmp/libunreal_text.S > /tmp/func_entry.txt
cat /tmp/func_entry.txt
FUNC_ENTRY_VA=$(awk '{gsub(":",""); print "0x"$2}' /tmp/func_entry.txt | head -1)
echo "function entry VA: $FUNC_ENTRY_VA"

# Step 5: convert to file offset, install uprobe, capture
TEXT_VA=0x0  # need to read from ELF
TEXT_OFF=$(python3 -c "
import struct
with open('$HOST_LIB', 'rb') as f:
    f.seek(0); _, _, _, _, e_phoff, _, _, _, _, e_phentsize, e_phnum = struct.unpack('<16sHHIQQQIHHH', f.read(0x40))
    f.seek(e_phoff)
    for _ in range(e_phnum):
        ph = f.read(e_phentsize)
        p_type, p_flags, p_offset, p_vaddr, _, p_filesz, _, _ = struct.unpack('<IIQQQQQQ', ph)
        if p_type == 1 and (p_flags & 1):  # PT_LOAD executable
            if p_vaddr <= int('$FUNC_ENTRY_VA', 16) < p_vaddr + p_filesz:
                print(hex(p_offset + (int('$FUNC_ENTRY_VA',16) - p_vaddr)))
                break
")
echo "SSL_write file offset: $TEXT_OFF"

# Find current thered PID + runtime lib path
PID=$(adb shell 'su 0 pidof com.netmarble.thered' | tr -d '\r')
[ -z "$PID" ] && { adb shell 'monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1' >/dev/null 2>&1; sleep 6; PID=$(adb shell 'su 0 pidof com.netmarble.thered' | tr -d '\r'); }
RUNTIME_LIB=$(adb shell "su 0 cat /proc/$PID/maps | grep libUnreal.so | grep 'r-xp' | head -1 | awk '{print \$NF}'" | tr -d '\r')
echo "PID=$PID runtime_lib=$RUNTIME_LIB"

# Install uprobe
adb shell "su 0 sh -c 'echo > /sys/kernel/debug/tracing/uprobe_events'" 2>/dev/null || true
adb shell "su 0 sh -c 'echo p:ssl_write_thered $RUNTIME_LIB:$TEXT_OFF buf=+0(%x1):string len=%x2:u64 > /sys/kernel/debug/tracing/uprobe_events'"
adb shell "su 0 cat /sys/kernel/debug/tracing/uprobe_events"

# Capture 30s with HOME/relaunch traffic trigger
adb shell "su 0 sh -c 'simpleperf record -e uprobes:ssl_write_thered -c 1 -o /data/local/tmp/ssl_sniff.perf -- sleep 30'" &
CAP=$!
sleep 3
adb shell 'input keyevent KEYCODE_HOME'
sleep 5
adb shell 'monkey -p com.netmarble.thered -c android.intent.category.LAUNCHER 1' >/dev/null 2>&1
sleep 10
wait $CAP

# Dump + extract
adb shell 'su 0 simpleperf dump -i /data/local/tmp/ssl_sniff.perf' > /tmp/ssl_dump.txt
wc -l /tmp/ssl_dump.txt
grep -c "uprobes:ssl_" /tmp/ssl_dump.txt || true
grep -aoE '(GET|POST|PUT) [^ ]+ HTTP|Host:[^\r\n]{1,80}|userKey=[A-F0-9]{1,40}|googleAuthCode|"playerId":"[A-F0-9]{1,40}"' /tmp/ssl_dump.txt | head -50 > /tmp/ssl_findings.txt
cat /tmp/ssl_findings.txt

# Write artifact + set fact
COUNT=$(grep -c "uprobes:ssl_" /tmp/ssl_dump.txt 2>/dev/null || echo 0)
cat > "$WORKDIR/analysis/ssl_sniff_capture_2026-05-14.md" << ARTEOF
# SSL sniff — libUnreal SSL_write capture (Turn 5)

## Resolved offsets
- libUnreal.so SSL_write literal at rodata offset: $SSL_WRITE_STR_OFF (VA $SSL_WRITE_VA)
- SSL_write function entry VA: $FUNC_ENTRY_VA
- SSL_write file offset for uprobe: $TEXT_OFF

## Capture
- Runtime lib: $RUNTIME_LIB
- thered PID: $PID
- Capture window: 30s with HOME+relaunch trigger
- ssl_write uprobe events captured: $COUNT

## Findings
\`\`\`
$(cat /tmp/ssl_findings.txt 2>/dev/null || echo "(no HTTP markers in dump)")
\`\`\`
ARTEOF

if [ "$COUNT" -gt 0 ]; then
  /home/sdancer/orchestrator/harness fact-set magic32_ssl_bundled_uprobe_captured_$COUNT "Bundled SSL_write uprobe in libUnreal.so at file offset $TEXT_OFF fired $COUNT times during 30s capture. HTTP findings: $(head -3 /tmp/ssl_findings.txt | tr '\n' '|')"
else
  /home/sdancer/orchestrator/harness fact-set magic32_ssl_bundled_uprobe_zero_fires "Bundled SSL_write uprobe installed at libUnreal:$TEXT_OFF (entry VA $FUNC_ENTRY_VA) but 0 events in 30s window. Either wrong function localization, or thered HTTP traffic not firing during HOME/relaunch. Next: extend capture or try SSL_write_ex."
fi
echo DONE
```

## Constraints
- Memory budget: ≤2GB. objdump of 158 MB .text → ~500 MB .S file. Use `wc -l` to check, don't `cat` the whole thing.
- ONE script run, ONE artifact write, ONE fact set, ONE exit. ≤10 minutes wall time.
- adb device: `localhost:5558`.
