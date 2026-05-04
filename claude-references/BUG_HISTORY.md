# Bug History & Fixed Bugs

> Referenced from CLAUDE.md (offloaded reference).
> Do not reintroduce fixed bugs.

---

## Scheduler — BUG HISTORY & ACTIVE STALL

- Round-robin via `WorkStealingQueue`. Uses `steal()` on local queue (should be `pop()` but functionally identical on single core).
- Timer IRQ fires → `timer_interrupt_stub` → `timer_interrupt_handler`.
- **"Fresh Frame" model enforced (Phase 28):** `switch_to` loads `kstack_top` as clean slate, pushes IRETQ frame manually. `add rsp, 8` removed. `TaskContext` offsets 0x90-0x98.

### BUG 1 (FIXED 2026-04-23 — was: kernel panic on any pdx_listen/safe_pdx_call):
**`current_pd_id` is NEVER updated by the scheduler.**
`set_pd()` is only called from `jump_to_userland()` which is NEVER called (dead code).
`current_pd_id` stays 0 forever. Any call path that hits `CoreLocal::current_pd_ref()`
(syscall 28 = pdx_listen, `safe_pdx_call` for slot>0) does:
```rust
DOMAIN_REGISTRY.get(0)  // domains[0] is null — PDs start at ID 1
    .expect("CoreLocal: Current PD lost")  // KERNEL PANIC
```
**Fix:** In `timer_interrupt_handler`, after `sched.tick()` returns `(old, next)`,
add `crate::core_local::CoreLocal::get().set_pd(unsafe { (*next_ctx_ptr).pd_id });`
before calling `switch_to`.

### BUG 2 (FIXED 2026-04-23 — was: corrupts callee-saved registers on context restore):
**`switch_to` saves KERNEL callee-saved registers into old task context, not user's.**
When timer fires from userland, `timer_interrupt_stub` pushes user registers to kernel
stack but DOES NOT restore them to the CPU register file before calling
`timer_interrupt_handler` → `switch_to`. The naked `switch_to` does:
```asm
"mov [rdi + 0x00], r15"  // saves KERNEL r15, not user r15!
```
User r15-rbp are sitting on the kernel stack (pushed by stub) but switch_to ignores them.
On restore, user gets kernel garbage in r15-rbp.
**Fix:** In `timer_interrupt_handler`, before calling `switch_to`, extract the user
callee-saved registers from the kernel stack frame and write them into `old_ctx.r15` etc.

### BUG 3 (FIXED 2026-04-23 — was: pdx_call always returns wrong value):
**`syscall_entry` discards `dispatch()` return value.** `pop rax` after
`call syscall_handler` restores the PUSHED original rax (= syscall number),
NOT the Rust function's return value. Dispatch must write `regs.rax = result`
to communicate return values to userland.
**Fix:** In `dispatch()`, write results via `regs.rax = value` and return 0.

### BUG 4 (minor — potential layout mismatch):
**`TaskContext` lacks `#[repr(C)]`** but `switch_to` uses hardcoded offsets.
Add `#[repr(C)]` to `TaskContext`.

### BUG 5 (ACTIVE — Phase 28 stall — scheduler returns None every tick):
**`Scheduler::tick()` never finds a task to switch to.** `steal()` returns `None`
for all cores despite `pdx_spawn` logging successful task registration.
`SWITCH` log lines never appear. `timer_tick` spam continues indefinitely.
**Diagnosis:** runqueue push and steal/pop operate on different state, or tasks
are registered after scheduler init but before runqueue is live.

---

## Known Fixed Bugs (do not reintroduce)

| File                        | Bug                                               | Fix |
|-----------------------------|---------------------------------------------------|-----|
| `kernel/src/interrupts.rs`  | `_wrpkru` used directly                           | Use `crate::pku::wrpkru` |
| `kernel/src/syscalls/mod.rs`| `opcode` referenced (undefined)                   | Use `num` |
| `kernel/src/gdt.rs`         | `kernel_tss_selector` used (wrong name)           | Use `tss_selector` |
| `kernel/src/memory/manager.rs` | `let next += 1` (syntax error)               | Use `self.next += 1` |
| `kernel/src/memory/manager.rs` | Unused imports `MEMMAP_REQUEST`, `HHDM_REQUEST` | Line deleted |
| `kernel/src/gdt.rs`         | `unsafe {}` around `interrupts::disable()`        | Remove unsafe block |
| `kernel/src/elf.rs`         | `let mut flags` (flags never mutated)             | Remove `mut` |
| `CLAUDE.md` (old note)      | "serial_println! must go through pdx_call(0,69)" | WRONG: sex-pdx uses direct asm syscall rax=69. Kernel handles natively. |
| `servers/sexusb/src/main.rs` | xHCI interrupt-IN Transfer Ring dequeue stuck at slot 1 forever | Circular ring: 15 Normal slots + Link TRB at slot 15 with TC=1. Track `intr_prod`/`intr_pcs`. |
| `servers/sexusb/src/main.rs` | Bounded 512-attempt outer poll exhausted before user interaction | Changed to unbounded `loop` with wrapping `u32` counter. |
| `servers/sexinput/src/main.rs` | Synthetic drag proof wraps forever via `% 3`, storms shell | Added `SYNTHETIC_DRAG_PROOF_DONE` one-shot gate. |
| `servers/sexdisplay/src/main.rs` | Stale boot-time SetClock(ss=0) arrives seconds late, permanently disables fallback | Added stale-time gate: `clock_from_silkbar=true` only if incoming ss ≥ fallback ss. |
| `servers/silkbar/src/main.rs` | Initial SetClock(ss=0) with stale time; `last_uptime_seconds` = `u64::MAX` | Removed init SetClock; changed `last_uptime_seconds` init to `0`. |
| `servers/sexdisplay/src/main.rs` | `clock_from_silkbar` one-way latch — clock freezes at last received time | Added 5-second liveness timeout. |
| `servers/silk-shell/src/main.rs` | `is_focusable_surface()` defined but never called | Added `try_set_focus()` guard. |
