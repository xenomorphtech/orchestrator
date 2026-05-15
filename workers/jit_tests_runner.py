#!/usr/bin/env python3
"""
JIT Tests Runner - Autonomous worker for aeon-jit test suite expansion.
Builds roundtrip test samples for loops, conditionals, and memory patterns.
"""
import anthropic
import subprocess
import os
import json
from pathlib import Path

def run_cargo_test():
    """Check current test state."""
    os.chdir("/home/sdancer/aeon")
    result = subprocess.run(
        ["cargo", "test", "--package", "aeon-jit", "--", "--nocapture"],
        capture_output=True,
        text=True,
        timeout=120
    )
    return result.stdout + result.stderr

def create_test_sample(sample_name: str, c_code: str) -> str:
    """Write a C test sample."""
    sample_dir = Path("/home/sdancer/aeon/crates/aeon-jit/samples")
    sample_dir.mkdir(parents=True, exist_ok=True)
    sample_file = sample_dir / f"{sample_name}.c"
    sample_file.write_text(c_code)
    return str(sample_file)

def main():
    client = anthropic.Anthropic()

    # Get current test status
    print("Checking current test suite status...")
    test_output = run_cargo_test()
    test_lines = test_output.split('\n')[-50:]  # Last 50 lines

    # Ask Claude to generate new test samples
    prompt = f"""You are building a roundtrip test suite for aeon-jit.

Current test status (last output):
{chr(10).join(test_lines)}

Task: Generate 2-3 new C test samples for roundtrip testing. Each sample should:
1. Be self-contained C code that compiles with aarch64-linux-gnu-gcc
2. Test a specific pattern: loops, conditionals, or memory patterns
3. Print a result to stdout that can be verified

For each sample, provide:
- Sample name (simple identifier)
- Complete C code

Format your response as JSON with array of objects:
[{{"name": "loop_simple", "code": "..."}}, ...]

Generate ONLY the JSON array, no other text."""

    message = client.messages.create(
        model="claude-opus-4.7",
        max_tokens=4096,
        messages=[
            {"role": "user", "content": prompt}
        ]
    )

    response_text = message.content[0].text
    print(f"Claude response:\n{response_text}\n")

    # Parse and save samples
    try:
        samples = json.loads(response_text)
        for sample in samples:
            sample_path = create_test_sample(sample["name"], sample["code"])
            print(f"Created sample: {sample_path}")
    except json.JSONDecodeError as e:
        print(f"Failed to parse JSON from Claude: {e}")
        print("Trying to extract code blocks...")
        # Fallback: try to extract code from markdown code blocks

    # Run tests with new samples
    print("\nRunning cargo test to verify new samples...")
    test_result = run_cargo_test()
    summary_lines = test_result.split('\n')[-20:]
    print('\n'.join(summary_lines))

    return "jit_tests_completed"

if __name__ == "__main__":
    main()
