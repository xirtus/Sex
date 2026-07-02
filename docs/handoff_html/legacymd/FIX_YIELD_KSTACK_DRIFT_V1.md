# FIX_YIELD_KSTACK_DRIFT_V1

- date: 2026-05-07
- fixes: PD_RUNTIME_REACHABILITY_PROOF_V1 audit findings R1 (critical) and R2 (dormant)

## Root Cause — R1 (CRITICAL, LIVE)

`yield_and_switch` used `ctx.kstack_top` (TaskContext field) to compute `ksp_base`:

```rust
let ksp_base = ctx.kstack_top.wrapping_sub(168);
...
ctx.kstack_top = ksp_base;
```

`ctx.kstack_top` starts at `kstack_alloc_top - 168` (forged frame pointer).
Each cooperative yield wrote back `ksp_base = ctx.kstack_top - 168`, decrementing by 168 on every call.

After n yields: `ksp_base = kstack_alloc_top - 168*(n+2)`
kstack = 65536 bytes → overflow after ≈ 390 yields.
PD1 yields ≈ 4×/sec → heap corruption in ≈ 97 seconds.

## Root Cause — R2 (HIGH, DORMANT)

Same function set TSS RSP0 via:

```rust
let kstack_top = (*next).kstack_top;   // Task.kstack_top = kstack_alloc_top
update_tss_rsp0(VirtAddr::new(kstack_top + 168));  // = kstack_alloc_top + 168
```

`Task.kstack_top` is already `kstack_alloc_top` (one-past-end = correct RSP0).
Adding 168 set RSP0 168 bytes past the buffer end.
Dormant: SYSCALL uses `gs:[16]`; LAPIC timer never fires.
Any real IRQ or CPL-change exception would have caused a heap corruption.

## Fix (kernel/src/scheduler.rs)

### R1 — ksp_base calculation

```rust
// Before:
let ksp_base = ctx.kstack_top.wrapping_sub(168);

// After:
let ksp_base = (*current).kstack_top.wrapping_sub(168);
// (*current).kstack_top = Task.kstack_top = kstack_alloc_top (fixed at alloc, never changes)
```

With fix: `ksp_base = kstack_alloc_top - 168` every yield — constant. No drift.
`ctx.kstack_top = ksp_base` now settles at `kstack_alloc_top - 168` and stays there.

### R2 — TSS RSP0

```rust
// Before:
crate::gdt::update_tss_rsp0(x86_64::VirtAddr::new(kstack_top + 168));

// After:
crate::gdt::update_tss_rsp0(x86_64::VirtAddr::new(kstack_top));
// Task.kstack_top = kstack_alloc_top (already the correct RSP0 value)
```

Note: timer_interrupt_handler correctly uses `ctx.kstack_top + 168` because in that path
`ctx.kstack_top` = address of rax push on interrupt stack, so `+168` = RSP at interrupt entry.
The yield path uses `Task.kstack_top` (not ctx), so no `+168` is needed.

## Gate Results

| Gate          | Before Fix | After Fix |
|---------------|-----------|-----------|
| BUILD_GATE    | PASS      | PASS      |
| SPAWN_GATE    | PASS      | PASS      |
| SCHED_GATE    | PASS      | PASS      |
| FAULT_GATE    | PASS      | PASS      |
| SEXFILES_GATE | PASS      | PASS      |
| CLOCK_GATE    | FAIL      | FAIL (pre-existing) |
| PD3 silk-shell | 11x      | 11x       |
| PD4 sexinput  | 11x       | 11x       |

No regressions. No `#PF`/`#GP`/`panic`/`fault.kill` markers.

## Remaining Risks

1. **CLOCK_GATE** — `get_ticks()` returns 0 (LAPIC timer never fires in QEMU). Pre-existing.
   Not related to scheduling correctness.

2. **SFMASK = empty** — IF not masked during SYSCALL. If LAPIC timer is ever fixed,
   timer IRQ can preempt mid-`tick()` / mid-`steal()`, causing duplicate task dequeue.
   `WorkStealingQueue::steal()` is NOT ISR-safe. Fix: restore `RFlags::INTERRUPT_FLAG`
   in SFMASK, or make runqueue ISR-safe, before enabling LAPIC timer.

3. **ctx layout coupling** — `yield_and_switch` offsets (`base+0..base+120`) depend on
   `syscall_entry` push order. If that order changes, reaudit all offsets in `yield_and_switch`.
   See comment added at ksp_base calculation.
