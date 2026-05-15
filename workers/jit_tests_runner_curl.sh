#!/bin/bash
# JIT Tests Runner - uses curl to call Claude API directly

cd /home/sdancer/aeon

echo "Checking current test suite status..."
cargo test --package aeon-jit 2>&1 | tail -50 > /tmp/jit_test_status.txt

TEST_STATUS=$(cat /tmp/jit_test_status.txt)

# Create prompt for Claude
read -r -d '' PROMPT << 'EOF'
You are building a roundtrip test suite for aeon-jit Cranelift JIT compiler.

Current test status (last output):
TEST_STATUS_PLACEHOLDER

Task: Generate 2-3 new C test samples for roundtrip testing. Focus on:
1. Loops (for, while with different exit conditions)
2. Conditionals (if-else branches)
3. Memory patterns (array access, pointer dereferencing)

For each sample, provide a JSON object with:
- "name": Simple identifier (e.g., "loop_simple")
- "code": Complete self-contained C code

Requirements:
- Each sample must compile with: aarch64-linux-gnu-gcc
- Must print a result to stdout that can be verified
- Should test a single pattern well
- Keep under 100 lines

Respond with ONLY a JSON array, no markdown or explanation:
[{"name": "...", "code": "..."}, ...]
EOF

PROMPT="${PROMPT//TEST_STATUS_PLACEHOLDER/$TEST_STATUS}"

# Call Claude API
echo "Asking Claude to generate test samples..."
RESPONSE=$(curl -s https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d @- <<PAYLOAD
{
  "model": "claude-opus-4-7",
  "max_tokens": 4096,
  "messages": [
    {"role": "user", "content": "$PROMPT"}
  ]
}
PAYLOAD
)

echo "Claude response:"
echo "$RESPONSE" | python3 -m json.tool 2>/dev/null | tail -100

# Extract content from response
CONTENT=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['content'][0]['text'])" 2>/dev/null)

if [ -z "$CONTENT" ]; then
  echo "Failed to get response from Claude"
  exit 1
fi

echo "Generated test code:"
echo "$CONTENT"

# Parse and save samples (simplified - just save as comment for now)
SAMPLE_DIR="/home/sdancer/aeon/crates/aeon-jit/samples"
mkdir -p "$SAMPLE_DIR"
echo "// Generated test samples from Claude:" > "$SAMPLE_DIR/generated_tests.c"
echo "$CONTENT" >> "$SAMPLE_DIR/generated_tests.c"

echo "Saved to $SAMPLE_DIR/generated_tests.c"
echo "jit_tests_completed"
