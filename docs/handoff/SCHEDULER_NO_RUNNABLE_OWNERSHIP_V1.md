# SCHEDULER_NO_RUNNABLE_OWNERSHIP_V1

Date: 2026-07-21

## Problem

`Scheduler::tick()` previously performed `current_task.swap(next_task)` before
proving `next_task` was non-null. When no runnable replacement existed, this
detached the current task and returned `None`. In the page-fault path this caused
the handler to skip its containment branch and potentially return through the
original user IRET frame.

## Fix

Phase 1 introduces an explicit `ScheduleDecision` enum:

```rust
pub enum ScheduleDecision {
    Switch { old: *mut TaskContext, next: *const TaskContext },
    NoRunnable,
}
```

`Scheduler::tick()` now:

1. Selects a candidate `next_task` from the local runqueue and cross-core steal.
2. If no task is found, returns `NoRunnable` **without** modifying `current_task`
   or `current_pd`.
3. Only when a concrete `next_task` exists does it swap `current_task`, mark the
   next task `Running`, and bind `current_pd`.

Old-task requeue rules are preserved unchanged:

- `Running` old task → `Ready` and requeued.
- `Blocked` old task → not requeued.
- `Exited` old task → not requeued.

All `tick()` callers were updated to match `ScheduleDecision`:

- `timer_interrupt_handler`
- `page_fault_handler`
- `yield_and_switch`
- `kernel/src/lib.rs` scheduler debug kick

## Files changed

- `kernel/src/scheduler.rs` — added `ScheduleDecision`, reordered `tick()`,
  updated `yield_and_switch`.
- `kernel/src/interrupts.rs` — mechanical `ScheduleDecision` matching in timer
  and page-fault handlers.
- `kernel/src/lib.rs` — mechanical `ScheduleDecision` matching in scheduler
  debug kick.
- `scripts/scheduler_no_runnable_ownership_gate.sh` — new static gate.
- `docs/handoff/SCHEDULER_NO_RUNNABLE_OWNERSHIP_V1.md` — this file.

## Validation

Source gate:
- `./scripts/scheduler_no_runnable_ownership_gate.sh` PASS

Build gates:
- `cargo build -Zbuild-std=core,alloc -p sex-kernel --target x86_64-sex.json` PASS

## Known remaining limitations

Phase 1 does **not** fix the following; they are scheduled for later phases:

1. `unpark_thread()` still only transitions state to `Ready`; it does not
   enqueue the task. Detached `Ready` tasks are not yet schedulable.
2. No wake queue exists yet.
3. `WorkStealingQueue::push()` can still silently fail when full.
4. Recoverable page-fault progress in the no-runnable case is not yet designed.
5. Direct user-return PKRU restore is not yet implemented.
6. Scheduler idle behavior is not yet fully designed.
7. The existing `yield_and_switch()` transient null `current_task` (between
   pushing the yielded task and `tick()` selecting the next) is intentionally
   preserved in Phase 1.

## Invariants established

- `NoRunnable` + attached current task:
  - `current_task` before == `current_task` after
  - `current_pd` before == `current_pd` after
  - task state unchanged
- `Switch` only occurs with a concrete `next_task`.

## Next step

Authorize Phase 2: bounded scheduler wake queue + CAS `wake_task`.
