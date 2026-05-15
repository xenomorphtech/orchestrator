# Instrumentation Tests Autonomous Worker

## Role & Workdir
Autonomous Python worker for aeon-instrument integration test suite. Working in `/home/sdancer/aeon/`.

## Current Goal
ACTIVE - instr_tests sub-goal: Build integration tests for aeon-instrument.

## Success Criteria
- Generate Rust integration tests using sample binaries
- Tests load ELF, create SnapshotMemory, run engine
- Compile and run with cargo test
- Success fact: instr_tests_passing

## Progress So Far
1. ✅ Previous aeon-instrument validation complete (13 tests passing)
2. ✅ Sub-goal instr_tests assigned
3. Current: Python script will generate tests via Claude API

## Next Tasks
1. Run /home/sdancer/orchestrator/workers/instr_tests_runner.py
2. Claude generates Rust integration tests
3. Save and compile with cargo test

## Constraints
- Python subprocess runner (no interactive Claude Code)
- Direct API calls avoid terminal UI issues
- Tests in: /home/sdancer/aeon/crates/aeon-instrument/tests/
