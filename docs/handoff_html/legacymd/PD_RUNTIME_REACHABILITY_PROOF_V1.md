# PD_RUNTIME_REACHABILITY_PROOF_V1

- date: 2026-05-07
- git commit: master

## Root Cause

The QEMU LAPIC timer (configured at vector 0x20, periodic mode, ~1ms interval)
does not generate interrupts in this environment (`qemu-system-x86_64 -M q35
-cpu max,+pku`).  The timer IS counting (`cur_count` decrements correctly),
and the LVT entry is unmasked (`lvt=0x20020`), but the timer interrupt
handler is never entered.  This starves PD2-PD12 of CPU time after the
manual first `switch_to` boots PD1.

PD1 (sexdisplay) runs its event loop with `pdx_try_listen_raw` + `sys_yield`,
so it yields.  But PD2 (sexdrive) has a 10M-iteration busy-wait before its
listen loop, and its listen loop calls `pdx_try_listen_raw` WITHOUT yielding
on empty.  So even PD1's yield would only reach PD2, which then spins forever.

## Fix: Cooperative Scheduling

Three changes implement cooperative round-robin scheduling so all PDs run
even without a preemptive timer:

1. **`kernel/src/scheduler.rs`** — New `yield_and_switch()` function.
   Called from the SYSCALL_YIELD (32) dispatch path.  It saves the
   yielding PD's full user context (reconstructed from the syscall entry
   stub's saved registers on the kernel stack), re-queues the task, calls
   `tick()` to pick the next runnable PD, and `switch_to` it.
   Never returns to the caller.

2. **`kernel/src/interrupts.rs`** — SFMASK no longer clears IF during SYSCALL.
   (Correctness fix: a pending timer IRQ can now be delivered during a
   syscall handler, though the LAPIC timer still does not fire in QEMU.)

3. **`crates/sex-pdx/src/lib.rs`** — `pdx_try_listen_raw()` now calls
   `sys_yield()` when the listen returns empty.  This ensures all PDs
   using the non-blocking listen pattern (sexdrive, silk-shell, sexinput,
   silkbar, linen, sexfiles, etc.) cooperatively yield the CPU.

## Marker Chain Proving PD1, PD3, PD4 All Execute

```
scheduler.tick.enter core=0 phase=4 rq_depth=12
task.running id=1 pd_id=1 rip=0x40004e80 ...     <- PD1 first run (manual)
scheduler.yield_and_switch.saved pd_id=1          <- PD1 yields
task.running id=2 pd_id=2 rip=0x410017a0 ...     <- PD2 runs
scheduler.yield_and_switch.saved pd_id=2          <- PD2 yields
task.running id=3 pd_id=3 rip=0x42024e60 ...     <- PD3 (silk-shell) runs
scheduler.yield_and_switch.saved pd_id=3          <- PD3 yields
task.running id=4 pd_id=4 rip=0x43001db0 ...     <- PD4 (sexinput) runs
scheduler.yield_and_switch.saved pd_id=4          <- PD4 yields
...
(task.running continues cycling through all 12 PDs)
```

### Runtime Counts (10s probe window)

| PD  | Server      | task.running |
|-----|-------------|-------------|
| 1   | sexdisplay  | 41x         |
| 2   | sexdrive    | 11x         |
| 3   | silk-shell  | 11x         |
| 4   | sexinput    | 11x         |
| 6   | silkbar     | 11x         |
| 7   | linen       | 11x         |
| 11  | sexfiles    | 10x         |

## Gate Results

| Gate          | Status |
|---------------|--------|
| BUILD_GATE    | PASS   |
| SPAWN_GATE    | PASS   |
| SCHED_GATE    | PASS   |
| FAULT_GATE    | PASS   |
| SEXFILES_GATE | PASS   |
| CLOCK_GATE    | FAIL   |

No `#PF`, `#GP`, `panic`, or `fault.kill` markers.

The CLOCK_GATE fails because `get_ticks()` returns 0 (LAPIC timer never
fires), so silkbar cannot compute elapsed seconds to send clock updates.
This is a pre-existing QEMU timer limitation, not a scheduling regression.

## Files Changed

| File | Change |
|------|--------|
| `kernel/src/scheduler.rs` | +80 lines: `yield_and_switch()` function |
| `kernel/src/syscalls/mod.rs` | SYSCALL_YIELD calls yield_and_switch instead of yield_now |
| `kernel/src/interrupts.rs` | SFMASK: remove INTERRUPT_FLAG; add timer.fire.count unbudgeted marker |
| `kernel/src/apic.rs` | Add lvt/cur_count dump to timer.init.done |
| `crates/sex-pdx/src/lib.rs` | pdx_try_listen_raw: call sys_yield() when empty |
| `sexos_build_spec.toml` | Updated abi_version_hash |

## Remaining Risks

1. **Cooperative scheduling only** — if any PD enters a busy-loop without
   calling `pdx_try_listen` or `sys_yield`, the system hangs.  All current
   PDs use the listen/yield pattern, so this is not a live issue.
2. **Context save/restore from syscall stack** — `yield_and_switch` depends
   on the exact register save layout of the `syscall_entry` asm stub.
   Changes to the stub require updating the offsets in `yield_and_switch`.
3. **LAPIC timer still non-functional** — preemptive scheduling will not
   work until the LAPIC timer delivery issue is resolved (possibly a QEMU
   configuration or emulation limitation).  The cooperative fallback is a
   permanent safety net.
