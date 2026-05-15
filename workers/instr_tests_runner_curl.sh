#!/bin/bash
# Instrumentation Tests Runner - uses curl to call Claude API directly

cd /home/sdancer/aeon

echo "Checking sample binaries and test structure..."
SAMPLES=$(ls /home/sdancer/aeon/samples 2>/dev/null | head -5 || echo "No samples found")
TEST_FILES=$(find /home/sdancer/aeon/crates/aeon-instrument/tests -name "*.rs" 2>/dev/null | wc -l)

# Create prompt for Claude
read -r -d '' PROMPT << 'EOF'
You are writing Rust integration tests for aeon-instrument, a dynamic instrumentation engine.

Available sample binaries: SAMPLES_PLACEHOLDER
Existing test count: TEST_FILES_PLACEHOLDER
Engine path: /home/sdancer/aeon/crates/aeon-instrument

Task: Write 2-3 Rust integration tests that:
1. Load an ELF binary sample
2. Create a SnapshotMemory from the loaded binary
3. Instantiate the instrumentation engine
4. Execute the engine on the sample
5. Assert the engine processes without crashing

Use patterns from aeon-jit's compile_block function if relevant.

Respond with ONLY Rust code in test module format - no explanation:
#[cfg(test)]
mod tests {
    use ...;

    #[test]
    fn test_name() {
        // implementation
    }
}
EOF

PROMPT="${PROMPT//SAMPLES_PLACEHOLDER/$SAMPLES}"
PROMPT="${PROMPT//TEST_FILES_PLACEHOLDER/$TEST_FILES}"

# Call Claude API
echo "Asking Claude to generate integration tests..."
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

# Extract content
CONTENT=$(echo "$RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin)['content'][0]['text'])" 2>/dev/null)

if [ -z "$CONTENT" ]; then
  echo "Failed to get response from Claude"
  exit 1
fi

echo "Generated test code:"
echo "$CONTENT"

# Save tests
TEST_DIR="/home/sdancer/aeon/crates/aeon-instrument/tests"
mkdir -p "$TEST_DIR"
TEST_FILE="$TEST_DIR/integration_engine.rs"
echo "$CONTENT" > "$TEST_FILE"
echo "Saved to $TEST_FILE"

# Try to compile
echo "Compiling instrumentation tests..."
cd /home/sdancer/aeon
cargo test --package aeon-instrument --lib 2>&1 | tail -30

echo "instr_tests_completed"
