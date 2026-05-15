# JIT Coverage Expansion - Active Goal

## Role & Workdir
Claude agent for aeon-jit coverage expansion. Working in `/home/sdancer/aeon/`.

## Current Goal Status
**ACTIVE** - aeon_jit/jit_coverage sub-goal. Expand JIT expression/statement coverage and commit uncommitted work.

## Completion Summary
✅ **jit_hash_eval sub-goal**: Completed (2026-04-19)
- 5 hash algorithm samples created: CRC32, FNV-1a, MD5, SHA-256, SipHash
- All samples self-contained, no external libraries, single main() return value
- 16/16 roundtrip tests passing (5 hash samples + 3 control samples)
- Native x86-64 output verified to match JIT execution for all algorithms
- Success fact jit_hash_eval_complete: SET

## Current Sub-Goal: jit_coverage
**Title**: Expand JIT expression/statement coverage and commit uncommitted work
**Success fact**: jit_coverage_expanded
**Priority**: 10

## Next Tasks
1. Run `cargo test` in `/home/sdancer/aeon/` to check current JIT state and identify failing tests
2. Examine `UnsupportedExpr` and `UnsupportedStmt` variants (in aeon-jit IL compiler) to identify gaps in coverage
3. Implement support for additional unsupported expr/stmt types found in step 2
4. Commit the changes with clear message describing new coverage added
5. Verify all tests pass after changes

## Constraints & Notes
- Test suite is deterministic and reproducible; commit-tracked for regression detection
- JIT instruction coverage currently includes: loops, conditionals, memory access, multiply/XOR/shifts
- All samples should be production-quality with correct output for known inputs
- Changes must maintain backward compatibility with existing test suite

## Relevant Files
- `/home/sdancer/aeon/crates/aeon-jit/samples/`: Hash algorithm test samples
- `/home/sdancer/aeon/crates/aeon-jit/`: Main JIT compiler crate
- `/home/sdancer/aeon/Cargo.toml`: Build configuration
