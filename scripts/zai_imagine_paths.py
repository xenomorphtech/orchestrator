#!/usr/bin/env python3
"""z.ai GLM-4.5-flash path-variation imaginer for /orchestrate.

Reads a goal description + falsified.md context from stdin or args, asks GLM
to enumerate N untried path variations using the mechanism-not-path framing
from the orchestrator skill.

Usage:
  echo "<goal+context>" | zai_imagine_paths.py [--n 5] [--max-tokens 4000]
  zai_imagine_paths.py --goal "<goal-key>" --context-file <path>

Key at /home/sdancer/.zai_api_key (mode 600).
"""
import argparse, json, os, sys, urllib.request, urllib.error

KEY_PATH = "/home/sdancer/.zai_api_key"
ENDPOINT = "https://api.z.ai/api/coding/paas/v4/chat/completions"  # subscription Coding plan endpoint
MODEL = "glm-5.1"  # max-tier subscription; fallback glm-4.7/4.6/4.5-flash also work

SYSTEM = """You are an adversarial path-imagineer for an autonomous research \
orchestrator. Your job is to enumerate UNTRIED path variations the orchestrator \
or its workers may have missed.

Hard rules:
- Falsifying ONE mechanism does NOT falsify the path. Always think mechanism-class.
- For every "blocked" claim, surface >=3 untried alternative mechanisms in the same class.
- DIVERSITY REQUIREMENT: your N variants MUST span at least 3 DIFFERENT mechanism classes from the list \
below. Do not return all N variants in the same class — that is exactly the failure mode the orchestrator \
hired you to break. If the goal scope makes one class dominant, still include >=2 cross-class variants \
(orthogonal angles).
- Mechanism classes (with examples, NOT exhaustive — invent more if relevant):
  * input-injection: xdotool/xclip/ydotool/uinput/XSendEvent/AT-SPI/USB-HID/RFB-vncdo/LD_PRELOAD-libX11
  * network-probe: pcap-wire/eBPF-kprobe/LD_PRELOAD-socket/kernel-module/HW-tap
  * UI-scraping: pixel-match/OCR/a11y-tree/memory-read/DOM-CDP/screencap-diff
  * binary-instrumentation: Frida/uprobes/kprobes/LKM/LD_PRELOAD-hook/HW-watchpoints/QEMU-replay
  * static-analysis: disasm/dataflow/symbolic-exec/decompile-port/static-call-graph
  * supervision-orchestration: systemd-unit-spawn/multi-runtime-coordinator/state-machine-poller/event-bus
  * decision-policy: rule-based/learned-policy/scripted-tree/LLM-in-loop/replay-of-recorded-trace
  * integration-glue: API-wrapper/IPC-bridge/shared-mem/file-watch-bus/HTTP-microservice
  * substrate-swap: different-host/different-DE/different-VM/different-GPU/different-OS
  * data-source-pivot: different-corpus/different-account/different-timestamp/synthetic-data
- Do NOT propose anything that matches a row in the provided falsified.md context with the same mechanism.
- Be terse. One line per path. Format: `path-name | mechanism-class | one-line hypothesis | one-line falsification trigger | cost (h)`.
- After the N rows, add a one-line meta-note: "class-coverage: <comma-separated classes used>".
"""

def call(messages, max_tokens):
    key = open(KEY_PATH).read().strip()
    body = json.dumps({
        "model": MODEL,
        "messages": messages,
        "max_tokens": max_tokens,
        "temperature": 0.7,
    }).encode()
    req = urllib.request.Request(
        ENDPOINT, data=body,
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=180) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return {"error": e.read().decode()}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=5)
    ap.add_argument("--max-tokens", type=int, default=4000)
    ap.add_argument("--goal", default=None)
    ap.add_argument("--context-file", default=None)
    ap.add_argument("--show-reasoning", action="store_true")
    args = ap.parse_args()

    if args.context_file:
        ctx = open(args.context_file).read()
    elif not sys.stdin.isatty():
        ctx = sys.stdin.read()
    else:
        ctx = ""

    user = f"""Goal: {args.goal or '(unspecified, see context)'}

Context (current state, prior attempts, falsified rows):
---
{ctx[:8000]}
---

Enumerate {args.n} UNTRIED path variations. Skip anything that matches a falsified \
mechanism already in the context above. Return a numbered list in the exact format \
specified by the system prompt."""

    resp = call([
        {"role": "system", "content": SYSTEM},
        {"role": "user", "content": user},
    ], args.max_tokens)

    if "error" in resp:
        print("ERROR:", resp["error"], file=sys.stderr)
        sys.exit(1)
    msg = resp.get("choices", [{}])[0].get("message", {})
    content = msg.get("content", "")
    reasoning = msg.get("reasoning_content", "")
    if args.show_reasoning:
        print("=== REASONING ===")
        print(reasoning)
        print("=== /REASONING ===\n")
    print(content)
    print(f"\n--- usage: {resp.get('usage')} model={resp.get('model')}", file=sys.stderr)

if __name__ == "__main__":
    main()
