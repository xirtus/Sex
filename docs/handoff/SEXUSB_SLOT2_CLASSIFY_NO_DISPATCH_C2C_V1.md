# SEXUSB_SLOT2_CLASSIFY_NO_DISPATCH_C2C_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED (build PASS)

## Summary

When a Transfer Event matches slot2/tablet (identified by C2B), read the
slot2 report buffer and log raw bytes + idle classification.  **No
forwarding to sexinput** — pure diagnostic classification.

This is C2C of the three-patch demux plan.

---

## 1. Branch Used

The `else` branch of the existing `slot == single_bind.slot_id` check
in the poll loop's inner event handler.  This is the path that previously
only logged `[sexusb.xhci.intr_in.event.bad]`.

**File:** `servers/sexusb/src/main.rs`, ~line 3772

## 2. Report Read Method

```rust
let d2 = &devices[1];
let s2_actual = if intr_residue <= d2.intr_report_len {
    d2.intr_report_len - intr_residue
} else { 0 };
let s2_ptr = d2.intr_report_va as *const u8;
let b0 = unsafe { core::ptr::read_volatile(s2_ptr.add(0)) };
let b1 = unsafe { core::ptr::read_volatile(s2_ptr.add(1)) };
let b2 = unsafe { core::ptr::read_volatile(s2_ptr.add(2)) };
```

Uses `volatile` reads directly from device VA — no intermediate copy.
`intr_residue` is the per-event residue from the Transfer Event TRB
(already extracted at this point).

## 3. New Markers

| Marker | Budget | Meaning |
|--------|--------|---------|
| `[sexusb.slot2.report.raw] b0=N b1=N b2=N len=N` | 16 | Raw bytes from slot2 report buffer |
| `[sexusb.slot2.report.idle] len=N` | 16 | All-zero report (QEMU 11.0.0 expected) |

## 4. Slot1 Regression Status

**No change.** The `if slot == single_bind.slot_id` check still sets
`intr_ok = true` for slot1 events.  Keyboard dispatch, re-arm-before-IPC,
burst spin — all unchanged.

## 5. Build Result

```
./scripts/entrypoint_build.sh → PASS, 1762 sectors, 0 errors
```

## 6. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +38 lines in else-branch: slot2 classify block |
| `servers/sexusb/src/main.rs` | +23 lines: C2B match helper (re-applied after build revert) |
| `docs/handoff/SEXUSB_SLOT2_CLASSIFY_NO_DISPATCH_C2C_V1.md` | Created |

## 7. Cumulative C2 State (C2A + C2B + C2C)

| Component | Lines | Status |
|-----------|-------|--------|
| `HidDevice` struct + `empty()` | +30 | ✅ |
| `devices` array + `device_count` | +7 | ✅ |
| `devices[1]` population (slot2) | +12 | ✅ |
| `devices[0]` population (slot1) | +12 | ✅ |
| `devices.ready` marker | +5 | ✅ |
| `s2_hid_role` hoist | +2 | ✅ |
| C2B match helper + `demux.match` marker | +23 | ✅ |
| C2C classify block + raw/idle markers | +38 | ✅ |
| **Total** | **~129 lines** | ✅ |

## 8. Invariants

| Check | Status |
|-------|--------|
| C1 marker present | ✅ |
| Poll loop structure unchanged | ✅ |
| Keyboard dispatch unchanged | ✅ |
| No forwarding to sexinput for slot2 | ✅ |
| No sexinput/shell/display/kernel edits | ✅ |

## 9. Next Phase

**SEXUSB_SLOT2_REARM_C2D_V1** — After classifying slot2, re-arm slot2's
interrupt ring (queue next Normal TRB + ring doorbell).  Still no sexinput
forwarding.  This keeps slot2 polling alive across iterations.

---

*End of SEXUSB_SLOT2_CLASSIFY_NO_DISPATCH_C2C_V1.md*
