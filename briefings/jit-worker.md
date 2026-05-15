# JIT Worker - aeon-jit Sub-goals

## Role & Workdir
Build comprehensive roundtrip test suite and expand JIT compiler coverage. Working in `/home/sdancer/aeon/`.

## Current Goal
ACTIVE - jit_tests sub-goal: Build comprehensive roundtrip test suite for aeon-jit.

## Success Criteria
- ✅ Assigned to jit_tests sub-goal (success_fact_key: jit_tests_comprehensive)
- Build C test samples for loops, conditionals, memory patterns
- Roundtrip tests: compile to AArch64, lift, JIT-compile, execute
- All new tests pass with cargo test --package aeon-jit

## Progress So Far
1. ✅ Previous aeon-jit evaluation complete (101 tests passing, thread-safety fixed)
2. ✅ Comprehensive documentation delivered
3. ✅ Now assigned to jit_tests sub-goal
4. Current: Starting roundtrip test suite expansion

## Next 3 Tasks
1. Run `cargo test --package aeon-jit` to see current test suite status and identify gaps
2. Add new C test samples in `/home/sdancer/aeon/crates/aeon-jit/samples/` for loops, conditionals, memory patterns
3. Ensure each sample compiles with aarch64-linux-gnu-gcc and passes roundtrip tests

## Constraints & Gotchas
- Focus on loops, conditionals, memory patterns (from jit_tests instruction_text)
- Ensure samples compile with aarch64-linux-gnu-gcc
- Run cargo test to verify - this is the validation
- All prior thread-safety fixes remain in place

## Relevant Files
- Harness sub-goal: jit_tests (assigned to jit-worker, status: active)
- Test directory: `/home/sdancer/aeon/crates/aeon-jit/samples/`
- Cargo project: `/home/sdancer/aeon/crates/aeon-jit/`
