# NMSS Native Harness

## Goal

The native harness exists to execute the ARM64 cert path in a more faithful environment than the Python emulator can provide. The main hope is that a native path can preserve more of the runtime behavior around address layout, threading, libc/libart interactions, and JIT calling conventions.

## Progress So Far

### Crash-family issues already cleared

The fact store shows a meaningful amount of progress on the native path:

- the native ARM64 harness built successfully
- several early crash families were resolved
- later notes report that vector push-back, stale vtable, low dispatch faults, guest-event handling, and signal-safe logging were all fixed
- the harness progressed past earlier libart infinite-loop behavior and into libc++

That means the native path is no longer in the "dies immediately" phase.

### Important findings

- one root cause was in the pthread wrapper layer:
  - `wrapped_pthread_create` generated fake thread ids
  - `pthread_join` / `pthread_detach` were later called on bogus handles
  - this caused libc futex crashes
- manifest-based runtime slot patching started working once the live bionic base was parsed correctly

## Current Blockers

The native harness still has serious correctness issues.

### Threading and runtime state

- child-thread entry behavior still needs better instrumentation
- synthetic thread creation got past earlier crashes but still led to stalls

### Android runtime dependencies

- libart still expects objects from a Dalvik heap region that was not fully modeled in some capture/replay paths
- one blocker specifically referenced zero-mapped access around `0x133361e8`

### JIT control-flow correctness

Later runs showed:

- a SIGBUS near `0x9b620e68`
- then progress past that point
- then a SIGSEGV where JIT code effectively returned to `0x4`

The most recent useful interpretation is:

- the native harness is executing substantially more of the path than before
- but it still loses control-flow integrity around returns / indirect branches

## Why The Native Path Still Matters

Even with those blockers, the native harness is still strategically useful because it may solve problems the emulator cannot:

- better fidelity for address layout
- less reliance on handwritten behavioral shortcuts
- better reproduction of thread and runtime interactions

If the cert really is as process-layout-sensitive as the current evidence suggests, the native harness may eventually become the faster route to parity.

## Near-Term Recommendations

1. Keep the Python emulator as the primary analysis surface.
2. Keep the native harness alive as a parity / validation path.
3. Focus native work on:
   - return-address correctness
   - child-thread entry tracing
   - remaining libart / heap expectations
   - indirect-branch target validation after JIT transitions

