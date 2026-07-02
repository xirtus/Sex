# Debug Plan: Probing first userland instruction execution

## Summary
The kernel successfully enters the scheduler and performs the first context switch to PD 1 (sexdisplay).
Log evidence shows:
- IRETQ frame is correctly forged on the kernel stack.
- ASM pivot `switch_to` loads `rsp` and performs `iretq`.
- The scheduler continues to tick, implying the kernel remains alive and pre-emption works.
- PD 1 is re-scheduled multiple times, with its state (RIP/RSP) being saved and restored.
- **Problem**: No serial output from PD 1 or any other PD. No `syscall` logs.

## Hypothesis
PD 1 is executing, but:
1. It crashes before its first `serial_println!`.
2. `serial_println!` (via `pdx_call(0, 69, ...)`) fails silently or hangs.
3. The entry point code is not what we expect (though bytes match `_start`).

## Probe 1: Earliest possible instrumentation in `syscall_entry`
We will add a raw serial output at the very beginning of `syscall_entry` to confirm userland is at least attempting a syscall.
Already exists: `"mov dx, 0x3f8"`, `"lea rsi, [rip + {enter_msg}]"`, ... in `syscall_entry`.
Wait, why is it not printing? `SYSCALL_STUB_ENTER_RAW` is "syscall.stub.enter.raw\n".

## Step 1: Force immediate serial write in `switch_to` AFTER `swapgs`
Just before `iretq`, we are in a precarious state. 
But wait, if the scheduler re-schedules PD 1, it means the timer interrupt fired *while PD 1 was running* (or supposedly running).
If it fired, `RIP` was saved.
From logs: `task.running id=1 pd_id=1 rip=0x40001620 rsp=0x700000100000`
This `rip=0x40001620` is the *saved* RIP from the timer interrupt.
If it matches the initial entry point, it means PD 1 was interrupted *exactly* at the entry point, or it never moved.
Actually, let's check if RIP changes across ticks for the same PD.

In log:
First PD 1 switch: `Switching to PD 1 (RIP=0x40001620, ...)`
Next PD 1 switch: `task.running id=1 pd_id=1 rip=0x40001620 ...`
It's STUCK at the same RIP.

This means:
- `iretq` returns to `0x40001620`.
- Immediate Fault? Or just spin?
- If Fault, `page_fault_handler` or `general_protection_fault` should trigger.
- Logs show `page_fault_handler` entering for `PD=1` in some cases (rsp fix attempts), but in the "stable" log it just seems to loop.

## Action: Instrument `page_fault_handler` and `general_protection_fault_stub` to be more verbose.
And check `idt` setup.

## Step 2: Canonical build and run.
1. Add `serial_println!` to GPF and PF handlers if not enough.
2. Check if `swapgs` in `switch_to` is balanced.
   `switch_to` does `swapgs` then `iretq`.
   `timer_interrupt_stub` does `swapgs` on entry (if from user) and `swapgs` on exit (if to user).
   This is correct.

Wait! In `switch_to`:
```rust
            "swapgs",
            "iretq",
```
If we are switching from Kernel (first boot) to User, `swapgs` sets GS to user base. Correct.
If we are switching from User A to User B?
`timer_interrupt_stub` (User A) -> `swapgs` (GS=Kernel) -> `timer_interrupt_handler` -> `switch_to(User A, User B)`.
`switch_to` -> `swapgs` (GS=User B) -> `iretq` (User B).
Wait, if `timer_interrupt_stub` already did `swapgs` on entry, GS is already Kernel.
`switch_to` then does `swapgs` -> GS is User.
Then `iretq` returns to User.
BUT `timer_interrupt_stub` has its own `swapgs` at the end!

Let's look at `timer_interrupt_stub` again:
```rust
        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 1f", 
        "swapgs", 
        "1:",
        ...
        "call timer_interrupt_handler",
        ...
        "mov rax, [rsp + 136]", 
        "test al, 3", 
        "jz 2f", 
        "swapgs", 
        "2:",
        "pop rax", ... "iretq"
```
If `timer_interrupt_handler` calls `switch_to`, `switch_to` DOES NOT RETURN to `timer_interrupt_stub`.
It "returns" via its own `iretq`.
So `switch_to` MUST handle `swapgs` if it's going to user.
```rust
            "swapgs",
            "iretq",
```
But `switch_to` doesn't check if it's going to user! It ALWAYS does `swapgs`.
What if it's switching to a KERNEL task? (We don't have those yet, but still).
Currently all tasks are User.

Wait, if `old_ctx` is NOT NULL (i.e. we are interrupting a task):
`timer_interrupt_stub` -> `swapgs` (GS=Kernel).
`switch_to` -> `swapgs` (GS=User).
Then `iretq`.
This seems correct for User->User.

What about First Boot?
`kernel_init` -> `switch_to(NULL, User)`.
Kernel GS is currently... what?
In `CoreLocal::init`:
```rust
        let ptr = alloc::boxed::Box::into_raw(core_local);
        GsBase::write(VirtAddr::from_ptr(ptr));
        x86_64::registers::model_specific::KernelGsBase::write(VirtAddr::from_ptr(ptr));
```
Both `GS_BASE` and `KERNEL_GS_BASE` are set to `CoreLocal`.
So `swapgs` just swaps them... they are the same.
This is likely the bug. `swapgs` depends on `KERNEL_GS_BASE` being the USER GS (or vice versa).
But in SexOS, we use `GS` for `CoreLocal` in kernel.
When in user, `GS` is... what? User doesn't use `GS`?
If user uses `syscall`, it MUST have `swapgs` to get kernel `GS`.
So `KERNEL_GS_BASE` should hold the kernel `CoreLocal` while in userland.
And `GS_BASE` should hold whatever user wants (usually 0).

In `CoreLocal::init`:
`GS_BASE` = `ptr`
`KERNEL_GS_BASE` = `ptr`
This is WRONG. `swapgs` will do nothing.

## FIX
In `CoreLocal::init`:
Only set `GS_BASE` (for kernel use).
`KERNEL_GS_BASE` should be ignored until we have user GS, or set to 0.
Wait, if we are in kernel, `GS_BASE` is active.
On `iretq` to user, we should `swapgs` so `GS_BASE` becomes User GS (0) and `KERNEL_GS_BASE` becomes Kernel GS (`ptr`).
Then on `syscall`, `swapgs` restores `GS_BASE` to `ptr`.

Let's check `CoreLocal::init` in `kernel/src/core_local.rs`.
