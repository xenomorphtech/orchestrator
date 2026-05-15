# MCP Quality Optimization - Active Goal

## Role & Workdir
Claude agent for aeon-instrument and MCP quality. Working in `/home/sdancer/aeon/`.

## Current Goal Status
**ACTIVE** - aeon_jit/mcp_quality sub-goal. Evaluate and optimize aeon MCP tool descriptions for LLM tool-calling quality.

## Completion Summary
✅ **instr_engine sub-goal**: Completed (2026-04-19)
- DynCfg lifting, JIT compilation, execution loop fully implemented
- 52 integration tests passing
- Engine executes ELF binaries end-to-end with memory tracing and symbolic analysis
- Success fact instr_engine_working: SET

## Current Sub-Goal: mcp_quality
**Title**: Evaluate and optimize aeon MCP tool descriptions for LLM tool-calling quality
**Success fact**: mcp_quality_optimized
**Priority**: 12

## Next Tasks
1. Read arXiv:2602.14878v1 (BFCL+ToolACE methodology) for evaluation framework
2. Locate aeon MCP tool definitions in `/home/sdancer/aeon/crates/aeon-frontend/src/service.rs`
3. Extract current tool descriptions and analyze their quality for LLM tool selection
4. Apply BFCL+ToolACE evaluation criteria to each tool definition
5. Optimize descriptions with improved clarity, parameter documentation, and use-case examples
6. Verify optimized descriptions enhance LLM ability to select appropriate tools

## Constraints & Notes
- Core engine logic is stable and validated — changes should preserve test coverage
- All architectural decisions documented in prior commits; refer to git log for rationale
- Dependencies: aeon-jit (JIT compilation), aeon-lifter (IL translation)
- This work focuses on the MCP/tool interface quality, not internal engine implementation

## Relevant Files
- `/home/sdancer/aeon/crates/aeon-frontend/src/service.rs`: MCP tool definitions
- arXiv:2602.14878v1: BFCL+ToolACE evaluation methodology
