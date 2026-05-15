# JIT Tests Autonomous Worker

## Role & Workdir
Autonomous Python worker for aeon-jit roundtrip test suite expansion. Working in `/home/sdancer/aeon/`.

## Current Goal
ACTIVE - jit_tests sub-goal: Build comprehensive roundtrip test suite for aeon-jit.

## Success Criteria
- Generate 2-3 new C test samples for loops, conditionals, memory patterns
- Each sample compiles with aarch64-linux-gnu-gcc
- All roundtrip tests pass via cargo test
- Success fact: jit_tests_comprehensive

## Progress So Far
1. ✅ Previous aeon-jit evaluation complete (101 tests passing)
2. ✅ Sub-goal jit_tests assigned
3. Current: Python script will generate test samples via Claude API

## Next Tasks
1. Run /home/sdancer/orchestrator/workers/jit_tests_runner.py
2. Claude generates 2-3 C test samples
3. Verify with cargo test --package aeon-jit

## Constraints
- Python subprocess runner (no interactive Claude Code permission issues)
- Direct API calls avoid terminal UI complications
- Samples in: /home/sdancer/aeon/crates/aeon-jit/samples/
