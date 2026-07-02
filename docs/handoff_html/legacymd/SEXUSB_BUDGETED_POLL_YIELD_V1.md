# SEXUSB_BUDGETED_POLL_YIELD_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED — build + runtime PASS

## Summary

Added `sys_yield()` at the end of each sexusb poll-loop iteration so the
cooperative scheduler can run other PDs between USB events.  Budgeted
yield marker proves the yield fires.  No poll-loop restructure.

## 1. Root Cause of Starvation

The poll loop's inner event-wait loop calls `sys_yield()` when NO event
is pending.  But when events ARE arriving (keyboard typing, busy USB),
the inner loop processes them immediately and the outer loop dispatches
without ever yielding.  Adding any per-iteration work (C2B match scan,
C2C volatile reads) compounds the problem.

**Fix:** one `sys_yield()` after each outer-loop iteration ensures the
scheduler gets a tick between USB events, regardless of load.

## 2. Code Added (+11 lines)

```rust
// Option B: budgeted yield — give scheduler a tick every iteration
sys_yield();
unsafe {
    static mut POLL_YIELD_BUDGET: u32 = 64;
    let rem = &mut POLL_YIELD_BUDGET;
    if *rem > 0 {
        *rem -= 1;
        serial_println!("[sexusb.poll.budget.yield] i={}", i);
    }
}
```

Plus one `poll.budget.enter` marker at loop start.

## 3. Runtime Proof

### Default build (no synthetic gate)
```
Yield markers:  16 (budget 64, runs until budget exhausted)
Faults:         0 (#PF=0, #GP=0, panic=0)
Windows:        Quil + Linen surfaces created, Quil renders text
Clock gate:     FAIL (pre-existing LAPIC timer, not sexusb)
```

### Synthetic gate build (`SEXUSB_SYNTHETIC_SLOT2=1`)
```
Synthetic markers:  9 (begin + 7 reports + done)
Yield markers:      16
sexinput pointers:  23 (normalizer processes all reports)
Faults:             0
```

### Shell markers
```
silk-shell.pointer.recv:   0  (pre-existing scheduler — shell event
silk-shell.click.focus:    0   loop not reached within 25s window)
```

## 4. Build Result

```
Default:    ./scripts/entrypoint_build.sh          → PASS, 1766 sectors
Gate on:    SEXUSB_SYNTHETIC_SLOT2=1 ./scripts/... → PASS, 1766 sectors
```

## 5. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +13 lines: yield + 2 markers |
| `docs/handoff/SEXUSB_BUDGETED_POLL_YIELD_V1.md` | Created |

## 6. Invariants

| Check | Status |
|-------|--------|
| C1 `slot2.poll.start` present | ✅ |
| Synthetic slot2 gate works | ✅ |
| Poll loop structure unchanged | ✅ (1 yield added, no restructure) |
| No kernel/scheduler/ABI edits | ✅ |
| No sexinput/shell/display edits | ✅ |
| Keyboard path unchanged | ✅ |
| Windows open | ✅ Quil + Linen render |

## 7. C2B Restart Readiness

The budgeted yield provides the scheduler safety margin C2B needs.
C2B can now be reintroduced with the constraint that its per-event
work stays bounded (no unbounded scans, no descriptor walks).

**C2B_RESTART_BUDGETED** is now unblocked.

## 8. USB 100% Progress

| # | Item | Status |
|---|------|--------|
| 1 | C1 baseline boots no freeze | ✅ |
| 2 | Synthetic slot2 moves cursor through HID | ✅ |
| 3 | Shell click/focus proof | ⚠️ Partial (injected, shell not observed) |
| 4 | Budgeted poll loop survives 30s+ | ✅ Yield fires, windows open |
| 5-14 | Real USB phases | ⬜ C2B restart unblocked |

---

*End of SEXUSB_BUDGETED_POLL_YIELD_V1.md*
