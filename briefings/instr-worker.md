# Instrumentation Worker - aeon-instrument Sub-goals

## Role & Workdir
Build integration tests for aeon-instrument RET instrumentation engine. Working in `/home/sdancer/aeon/`.

## Current Goal
ACTIVE - instr_tests sub-goal: Build integration tests for aeon-instrument using sample binaries.

## Success Criteria
- ✅ Assigned to instr_tests sub-goal (success_fact_key: instr_tests_passing)
- Write tests that exercise instrumentation engine on sample binaries
- Tests use existing samples/ directory, load ELF, create SnapshotMemory, run engine
- All new tests pass

## Progress So Far
1. ✅ Previous aeon-instrument validation complete (13 integration tests passing, RET fix implemented)
2. ✅ feature/aeon-instrument-validation branch pushed to remote
3. ✅ Comprehensive test coverage and documentation complete
4. ✅ Now assigned to instr_tests sub-goal
5. Current: Starting integration test suite for instrumentation engine

## Next 3 Tasks
1. Check `/home/sdancer/aeon/samples/` directory for available sample binaries
2. Write tests in test suite that load ELF samples, create SnapshotMemory, instantiate instrumentation engine
3. Run tests to verify engine can process samples correctly

## Constraints & Gotchas
- Use existing samples/ directory (don't create new binaries)
- Tests should exercise: ELF loading, SnapshotMemory creation, engine execution
- Reference how aeon-jit compile_block works for similar patterns
- The lifter is in aeon::lifter module
- Feature branch feature/aeon-instrument-validation already has 13 passing tests as foundation

## Relevant Files
- Harness sub-goal: instr_tests (assigned to instr-worker, status: active)
- Samples directory: `/home/sdancer/aeon/samples/`
- Instrumentation engine: `/home/sdancer/aeon/crates/aeon-instrument/`
- Lifter: `/home/sdancer/aeon/crates/aeon/src/lifter.rs`
