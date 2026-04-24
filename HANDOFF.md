# Hand-off: IRETQ Contract Enforcement (Phase 28)

## Problem
Scheduler loop stalled. `timer_tick` spams, but context switch never completes.

## Diagnosis
8-byte stack misalignment detected.
`timer_interrupt_stub` pushes `dummy_error_code` (8 bytes) onto the *interrupted task's stack*.
`switch_to` manually loads `rsp` to `kstack_top` (switching to a *fresh* kernel stack) and then pushes a new IRETQ frame.

The 8-byte misalignment was a red herring. The real issue is the **Stack Model Mismatch**:
- `switch_to` expects a pristine `kstack_top`.
- The logic remains hybrid/impure because of how we build `TaskContext` vs how `switch_to` consumes it.

## Findings
- `add rsp, 8` blindly is dangerous. It works if reusing the interrupt stack, but fails if we've already switched to `kstack_top`.
- `switch_to` (current implementation) switches to `kstack_top` *before* handling the IRETQ frame.
- The `TaskContext` layout and the manual `push` operations in `switch_to` were relying on different stack assumptions.

## Resolution
- Enforced "Fresh Frame" model (Case B): `switch_to` now treats the kernel stack as a clean slate.
- Standardized `TaskContext` offsets (0x90-0x98) to reflect the exact IRETQ frame layout.
- Removed legacy `add rsp, 8` logic, as it conflicts with explicit stack switching.
- **Instrumentation:** Added `SWITCH` and `TASKS` logging in `Scheduler::tick()` to verify runqueue state.

## Status
- **Stall:** Persists.
- **Observation:** `SWITCH` logs are completely absent from output. `timer_tick` spam continues.
- **Inference:** `Scheduler::tick()` is returning `None` consistently. The scheduler is not successfully stealing tasks or even finding tasks to switch to, despite `pdx_spawn` reporting registration.
- **Next Root Cause Investigation:** Scheduler state/runqueue initialization failure. Why is `steal()` returning `None` for all cores?
- **Action:** Instrument `runqueue.steal()` and `attempt_steal()` to debug why tasks remain invisible to the scheduler.
