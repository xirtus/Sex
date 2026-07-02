# FIX_KERNEL_MODE_TIMER_PREEMPTION_V1

- date: 2026-05-25
- fixes: spindle GP fault / kernel-mode kstack corruption after PKU RSP scratch fix

## Symptom

After PKU GSbase RSP fix (push user RSP to per-task kstack at syscall_entry), many tasks
yielded successfully but a GP fault (with page faults) crashed the system:

```
task.running id=3 pd_id=3 rip=0xffffffff8020a7f5 rsp=0x44444452c600
iret.actual rsp=0x4444445f84a8 q1.cs=0xffffffff80238190 q2.rflags=0x4444445c4940 q3.rsp=0x1 q4.ss=0xffffffff80205d11
EXCEPTION: PAGE FAULT at 0x4443c49d5b80 (RIP: 0xffffffff8021b70b, RSP: 0x4444448182e0, ERR: 0x0)
```

IRET frames had garbage CS/SS (kernel addresses instead of selectors), causing iretq to fault.

## Root Cause

When the LAPIC timer fires while a task is in **kernel mode** (mid-syscall, CS.RPL=0):

1. No privilege-level change → CPU uses **current RSP** = shared CoreLocal kernel stack (GS:[16])
2. `timer_interrupt_handler` saves context: `old_ctx.kstack_top = (interrupt_frame) - 128`
   → kstack_top points into the **shared** kernel stack
3. Other tasks run their syscalls on the **same** shared stack, overwriting that memory
4. On reschedule, switch_to loads the corrupt frame → iretq with garbage SS → page fault/GP

This bug pre-existed but was masked: the PKU fix allowed more tasks to reach user mode
without early crash, increasing the timer window for mid-syscall preemption.

## Fix (kernel/src/interrupts.rs)

Added kernel-mode guard before `sched.tick()` in `timer_interrupt_handler`:

```rust
// Don't preempt kernel-mode execution (CS.RPL=0 = mid-syscall on shared CoreLocal kstack).
// Saving context there is unsafe: other tasks' syscalls reuse the same stack and will
// corrupt the saved frame before this task is rescheduled.
// Skip the context switch; the task finishes its syscall and yields cooperatively.
if stack_frame.code_segment.0 as u64 & 3 == 0 {
    unsafe { send_eoi(); }
    return;
}
```

Check placed BEFORE `sched.tick()` so no runqueue state is disturbed.

User-mode tasks (CS.RPL=3) continue to be preempted by the timer normally.
Kernel-mode syscalls complete without preemption and yield via `sched_yield` (syscall 32).

## Proof Markers

- `[spindle.init.start]` ✓
- `[spindle.boot]` ✓
- `task.running id=1 pd_id=1` ✓
- `task.running id=12 pd_id=12` multiple times with consistent RSP ✓
- `[sexusb.ready]` ✓
- No GP FAULT ✓
- No KERNEL PANIC ✓
- No PKU SECURITY VIOLATION ✓

## Remaining Known Fault

`silkbar` null-write (`fault.kill user_null_jump pd=6 rip=0x46005a3e err=0x6`) is pre-existing
(present in runs before this fix). Causes `faults_zero FAIL`. Not related to this fix.

## Next

USB_HID_BOOT_KEYBOARD_HUMAN_OPERATOR_PASS
