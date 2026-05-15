#!/usr/bin/env python3
"""
Instrumentation Tests Runner - Autonomous worker for aeon-instrument test suite.
Builds integration tests for sample binary instrumentation.
"""
import anthropic
import subprocess
import os
from pathlib import Path

def list_samples():
    """List available sample binaries."""
    samples_dir = Path("/home/sdancer/aeon/samples")
    if samples_dir.exists():
        samples = list(samples_dir.glob("*"))
        return [s.name for s in samples if s.is_file()]
    return []

def main():
    client = anthropic.Anthropic()
    os.chdir("/home/sdancer/aeon")

    # Get available samples
    print("Checking available sample binaries...")
    samples = list_samples()
    print(f"Available samples: {samples}")

    # Check instrumentation engine structure
    instr_dir = Path("/home/sdancer/aeon/crates/aeon-instrument")
    test_files = list(instr_dir.glob("tests/*.rs"))
    existing_tests = [f.name for f in test_files] if test_files else []
    print(f"Existing test files: {existing_tests}")

    # Ask Claude to write integration tests
    prompt = f"""You are building integration tests for aeon-instrument, a dynamic instrumentation engine.

Available sample binaries: {samples}
Existing test structure: {existing_tests}
Engine location: /home/sdancer/aeon/crates/aeon-instrument

Task: Write Rust integration tests that:
1. Load an ELF sample binary
2. Create a SnapshotMemory from the loaded binary
3. Instantiate the instrumentation engine
4. Execute the engine on the sample
5. Verify the engine processes without crashing

Provide 2-3 test functions ready to add to the test suite.
Each test should follow patterns from aeon-jit's compile_block function.

Format your response as Rust code (test module format):
#[cfg(test)]
mod tests {{
    // imports...

    #[test]
    fn test_... {{
        // implementation
    }}
}}

Generate ONLY the Rust code, no other text."""

    message = client.messages.create(
        model="claude-opus-4.7",
        max_tokens=4096,
        messages=[
            {"role": "user", "content": prompt}
        ]
    )

    response_text = message.content[0].text
    print(f"Claude generated tests:\n{response_text}\n")

    # Save tests to a file
    test_file = instr_dir / "tests" / "integration_engine.rs"
    test_file.parent.mkdir(parents=True, exist_ok=True)
    test_file.write_text(response_text)
    print(f"Saved tests to: {test_file}")

    # Try to compile tests
    print("\nCompiling instrumentation tests...")
    result = subprocess.run(
        ["cargo", "test", "--package", "aeon-instrument", "--test", "integration_engine", "--", "--nocapture"],
        capture_output=True,
        text=True,
        timeout=120
    )
    print("Compilation/test output:")
    print(result.stdout)
    if result.returncode != 0:
        print("STDERR:")
        print(result.stderr)

    return "instr_tests_completed"

if __name__ == "__main__":
    main()
