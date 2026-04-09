# NMSS Init Summary

Date: 2026-04-09

## Goal

Establish the real initialization path required for stable `NmssSa` certificate generation, separate that from dynamic JIT tracing issues, and identify which init shortcuts were unsafe.

## Main Findings

- The empty token problem was not caused by the dynamic JIT regime itself.
- `NmssSa` was fragile if touched too early or re-initialized aggressively from Frida.
- The reliable path was to let the app own initialization timing and avoid forcing extra init work.

## Java-Side Lifecycle We Confirmed

- `GameActivity` performs the normal singleton setup by calling:
  - `NmssSa.getInstObj().init(activity, null)`
- Later lifecycle flow calls:
  - `NmssSa.onResume()`

This is the normal app-owned path that makes the singleton valid.

## Failure Modes We Observed

- Fresh sessions could show `NoClassDefFoundError` if `NmssSa` or related static access was forced too early.
- Some probe strategies destabilized attach or initialization when they hooked too aggressively in Java.
- Forcing `loadCr()` or forced re-init in the old readiness path was unsafe.
- Earlier Java-side state snapshots showed bad readiness states such as:
  - `m_bAppExit = true`
  - `m_bIsRPExists = false`
  - `m_detectCallBack = null`

Those states were symptoms of an invalid init sequence, not proof that dynamic tracing was the root cause.

## Safe Readiness Path

The stable approach was:

1. Use minimal spawn/attach mode.
2. Avoid eager `loadCr()`.
3. Avoid forced re-init.
4. Let the app lifecycle establish `NmssSa`.
5. Then run plain `/prepare` and plain `/call`.

That path reliably produced a real token while keeping the app alive.

## Stable Result

Using the minimal readiness sequence, both `/prepare` and `/call` returned the same valid token:

`CC1583586D18D1BE28F5E4B48C554F0DA21FA3FFC05413A0`

This proved:

- the app-side NMSS initialization could be made stable
- token generation worked without forcing the old risky init path
- later tracing problems were downstream tracing/instrumentation issues, not a basic inability to initialize NMSS

## Practical Conclusion

The discovered rule is:

- Do not treat `NmssSa` as something to bootstrap manually from scratch during attach.
- Wait for the app-owned lifecycle to initialize it.
- Keep the readiness sequence minimal.
- Only arm deeper tracing after plain token generation is already working.

## Recommended Operating Order

1. Attach in the least invasive mode.
2. Confirm app-owned `NmssSa` initialization has happened.
3. Verify plain token generation first.
4. Only then enable deeper relay or page-trace instrumentation.
