# Debug Strategy: Verifying Syscall Dispatch

## Findings
PD 1 is scheduled but its RIP remains static at `0x40001620`. No syscall logs appear.
The `syscall_entry` ASM has a hardcoded serial print `SYSCALL_STUB_ENTER_RAW` ("syscall.stub.enter.raw\n").
The fact that this string never appears in the log means **Userland never executes `syscall`**.

## Plan
1. **Verify Userland Entry Point**:
   Check `purple-scanout` disasm to ensure `_start` actually starts with something that should lead to a syscall.
   In `main.rs`, `_start` calls `debug_syscall_probe()` first thing.
   ```rust
   #[inline(always)]
   fn debug_syscall_probe() {
       unsafe {
           core::arch::asm!(
               "syscall",
               in("rax") 0u64,
               in("rdi") 0u64,
               in("rsi") 0x5151u64,
               ...
           );
       }
   }
   ```
   If this `syscall` instruction was hit, we MUST see "syscall.stub.enter.raw\n" OR a Fault.

2. **Verify `CoreLocal` GS mapping**:
   If `swapgs` in `switch_to` works, but `swapgs` in `syscall_entry` fails, it might be due to `KERNEL_GS_BASE` not being set correctly in `CoreLocal::init`.
   Actually, `switch_to` also uses `swapgs`. If it failed there, the kernel would likely GPF immediately on `iretq` or soon after.
   But the kernel continues to live.

3. **Check IRETQ Frame Balance**:
   The logs show `iret.actual q0.rip=0x0` in some failure modes. 
   This means `switch_to` is reading garbage from the stack.
   Wait, if `switch_to` reads `q0.rip=0x0` just before `iretq`, then `iretq` will return to `0x0`.
   This would cause a Page Fault at `0x0`.
   But the logs show `Switching to PD 1 (RIP=0x40001620...)` and then `context_switch.before_switch_to rip=0x40001620`.
   Wait, if the *next* tick sees `rip=0x40001620`, it means either:
   a) PD 1 never ran (stuck in `switch_to`?)
   b) PD 1 was interrupted at exactly `0x40001620`.

If `switch_to` was stuck, the timer interrupt would still fire (it's hardware).
But `switch_to` has `iretq`. It *must* go somewhere.

## The `CoreLocal` `GS` Bug
In `CoreLocal::init`:
```rust
        let ptr = alloc::boxed::Box::into_raw(core_local);
        GsBase::write(VirtAddr::from_ptr(ptr));
        x86_64::registers::model_specific::KernelGsBase::write(VirtAddr::from_ptr(ptr));
```
When we are in Kernel, `GS` points to `CoreLocal`.
On `switch_to`:
`swapgs` -> `GS` points to whatever was in `KERNEL_GS_BASE` (which is also `CoreLocal`!).
So `GS` still points to `CoreLocal`.
Now we are in User. `GS` points to `CoreLocal`.
**User can now read/write `CoreLocal` if they know the offset!** This is a security hole, but more importantly:
If User does `syscall`:
`syscall_entry` -> `swapgs`.
`GS` now points to... what was in `KERNEL_GS_BASE` (which was `CoreLocal`).
So `GS` still points to `CoreLocal`.
This part actually "works" by accident because both bases are the same.

BUT, if we are in User and a Timer Interrupt occurs:
`timer_interrupt_stub` -> `test al, 3`, `jz 1f`, `swapgs`.
It does `swapgs`. `GS` stays `CoreLocal`. Correct.
Then `iretq` back to User: `swapgs` again. `GS` stays `CoreLocal`. Correct.

So why is it stuck?
Maybe `swapgs` IS failing? 
On some CPUs, `swapgs` with invalid/non-canonical addresses in `KERNEL_GS_BASE` can cause issues.
But here they are canonical.

## Wait! IRETQ and alignment.
`switch_to` does `add rsp, 120`.
15 GPRs * 8 = 120.
After `add rsp, 120`, `rsp` should point to the `iretq` frame: `[RIP, CS, RFLAGS, RSP, SS]`.
`Task::new` seeds:
`SS` (top)
`RSP`
`RFLAGS`
`CS`
`RIP`
GPRs (15)
Total 20 qwords = 160 bytes.
`forged_ksp` = `kstack_alloc_top - 160`.
`switch_to` loads `rsp = [rsi + 0xC0]` (which is `forged_ksp`).
`add rsp, 120` -> `rsp` = `forged_ksp + 120`.
This points to `RIP`.
`iretq` consumes 5 qwords (40 bytes).
`120 + 40 = 160`. Correct.

## Is `0xC0` correct?
`TaskContext` struct:
15 GPRs (0..112)
`dummy_error_code` (120) - WAIT. 15*8 = 120. So `dummy` is at 120.
`pkru` (128)
`pd_id` (136)
`rip` (144)
`cs` (152)
`rflags` (160)
`rsp` (168)
`ss` (176)
`pd_ptr` (184)
`kstack_top` (192) - 192 = `0xC0`.
YES, `0xC0` is correct.

## What if `iretq` is returning to a non-canonical address?
`0x40001620` is fine.

## Let's look at `timer_interrupt_handler` saving state.
```rust
            let base = stack_frame as *const _ as *const u64;
            old_ctx.r15 = *base.offset(-2);
```
Wait. `stack_frame` is `InterruptStackFrame`.
In `timer_interrupt_stub`:
```rust
        "push 0", // DUMMY ERROR CODE
        "push r15", ... "push rax",
        "lea rdi, [rsp + 136]", // rdi points to InterruptStackFrame (RIP)
        "call timer_interrupt_handler",
```
`rsp` before `lea` is:
`rax` (0)
...
`r15` (112)
`dummy` (120)
`RIP` (128) <- `lea rdi, [rsp + 128]`? No, code says `136`.
Wait. 15 GPRs + 1 dummy = 16 qwords = 128 bytes.
If `rax` is at `[rsp]`, then `dummy` is at `[rsp + 120]`.
`RIP` is at `[rsp + 128]`.
So `lea rdi, [rsp + 128]` would point to `RIP`.
But the code says `lea rdi, [rsp + 136]`.
This means `timer_interrupt_handler` receives a pointer to `CS`?
Let's check `timer_interrupt_stub` again.
```rust
        "lea rdi, [rsp + 136]", // rdi points to InterruptStackFrame (RIP)
```
Wait, if it's `136`, then `rdi` is `RIP + 8`?
No, `15 * 8 = 120`. `dummy` is at `120`. `RIP` is at `128`.
`136` is `CS`.
This is a BUG in `timer_interrupt_stub`. It should be `128`.

AND in `timer_interrupt_handler`:
```rust
            let base = stack_frame as *const _ as *const u64;
            old_ctx.r15 = *base.offset(-2);
```
If `stack_frame` is at `RIP` (128), then `offset(-1)` is `dummy` (120).
`offset(-2)` is `r15` (112).
`offset(-16)` is `rax` (0).
This matches the `push` order.
But if `stack_frame` is at `CS` (136), then `offset(-1)` is `RIP`.
`offset(-2)` is `dummy`.
`offset(-3)` is `r15`.
This would shift all saved GPRs by 1!

## WAIT!
`switch_to` uses `kstack_top` (offset `0xC0`).
`timer_interrupt_handler` doesn't seem to update `kstack_top`.
It updates `rip`, `cs`, `rflags`, `ss`, and GPRs in `old_ctx`.
BUT `switch_to` RESTORES from `kstack_top`.
If `kstack_top` is never updated to the *current* `rsp` of the interrupted task, then `switch_to` will always restore from the *original* `forged_ksp`!
This explains why RIP is always `0x40001620`. Every time we switch back to PD 1, we start over at the entry point!

## Is `kstack_top` updated?
In `timer_interrupt_handler`:
I don't see `old_ctx.kstack_top = ...`.
Let's check the code again.
