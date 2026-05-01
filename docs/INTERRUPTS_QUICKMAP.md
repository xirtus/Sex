# Interrupts Quickmap

**File**: `kernel/src/interrupts.rs`
**Purpose**: IDT dispatch + exception/interrupt control flow only.
**Rule**: Debug tracing lives in external helpers. Do NOT add temporary trace
`serial_println!` calls here — use `kernel/src/debug_trace.rs` instead.

---

## Section Index

| Line  | Symbol | Kind | What |
|-------|--------|------|------|
| 1–8 | — | imports | `InterruptStackFrame`, `RingBuffer`, atomics |
| 11–29 | `PageFaultEvent`, `TICKS`, `SEXT_QUEUE`, `INPUT_RING` | statics | Global event queues |
| 31–56 | `register_irq_route()`, `init_idt()`, `IDT` | init | IDT setup, vector registration |
| 58–89 | `init_idt()` (cont) | init | SYSCALL LSTAR/STAR MSR setup |
| 108–112 | `SyscallRegs` | struct | Register layout for syscall dispatch |
| 131–293 | `syscall_entry` | naked asm | SYSCALL entry/exit (swapgs, saves, wrpkru, dispatch, iret) |
| 295–336 | `page_fault_stub` | naked asm | #PF entry: push GPRs, swapgs, wrpkru, call handler |
| 337–360 | `general_protection_fault_stub` | naked asm | #GP entry: push GPRs, swapgs, call handler |
| 361–456 | `timer_interrupt_stub` + `timer_interrupt_handler` | asm + fn | Timer: save ctx, tick(), context switch |
| 458–465 | `faulted_task_halt()` | fn | Kernel halt trampoline for killed user tasks |
| 466–618 | `page_fault_handler` | fn | #PF dispatch: null-jump kill, PKU warden, forward, tick, switch |
| 620–725 | `general_protection_fault_handler` | fn | #GP dispatch: GDTR/TR dump, iret frame analysis, stack bounds |
| 727–741 | `breakpoint_handler`, `double_fault_handler`, etc. | fn | Misc exception handlers |

---

## Critical Invariants

1. **Never return to a poisoned user IRET frame.** If a user task faults
   fatally (e.g., RIP=0x0), the handler must rewrite the IRET frame to
   `faulted_task_halt` (kernel mode, IF=1) so the scheduler can continue.
   See `USER_FAULT_CONTAINMENT_V1` in `docs/manual_sex.md`.

2. **`page_fault_stub` stack layout** (after `sub rsp, 8` alignment):
   ```
   [rsp+0..+119]:  rax..r15  (15 x 8 = 120 bytes)
   [rsp+128]:      error code (8 bytes)
   [rsp+136]:      IRET frame start -> RIP, CS, RFLAGS, RSP, SS (40 bytes)
   ```
   `stack_frame` argument to handler = `rsp + 136`.

3. **timer_interrupt_handler** saves old_ctx from `stack_frame` and GPRs
   using `base.offset(-N)` arithmetic matching the stub layout above.

4. **`switch_to`** (in `scheduler.rs`) loads `rsp = next_ctx.kstack_top`,
   pops 15 GPRs + dummy, then iretq. The IRET frame must be at
   `kstack_top + 128`.

---

## Common Debug Entry Points

| What to find | rg pattern |
|---|---|
| Page fault handler | `rg "page_fault_handler" -n` |
| Timer + context switch | `rg "timer_interrupt_handler\|switch_to" -n` |
| Null-jump kill path | `rg "fault.kill\|faulted_task_halt\|Exited" -n` |
| IRET frame redirect | `rg "iret_redirect\|IRET frame" -n` |
| PKU warden | `rg "pku_warden" -n` |
| GPF handler | `rg "general_protection_fault" -n` |
| SEXT queue forward | `rg "forward_page_fault" -n` |
| IDT registration | `rg "set_handler_addr\|set_handler_fn" -n` |

---

## Do Not

- Add temporary `serial_println!` debug traces here. Use `debug_trace.rs`.
- Read the whole file. Use `rg` + `sed -n 'N,Mp'` to inspect only what you need.
- Paste or summarize the full file in responses. Reference line ranges instead.
