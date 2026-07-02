# SEXUSB_C2C_RESTART_BUDGETED_REPORT_FETCH_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED — build PASS, runtime no slot2 events (expected)

## Summary

Added bounded slot2 report read + classify in the else-branch of the
event match.  Activates only when `device_count > 1` and a Transfer Event
matches `devices[1]`.  Reads 3 bytes, classifies as all_zero/button/motion.
No forwarding to sexinput.

## 1. Classify Logic

```rust
if device_count > 1 {
    let d2 = &devices[1];
    if d2.active && slot == d2.slot_id && ep == d2.intr_dci
        && (cc == TRB_CC_SUCCESS || cc == TRB_CC_SHORT_PACKET)
    {
        let s2_actual = ...;  // from residue
        let b0,b1,b2 = volatile reads from d2.intr_report_va;
        let class = if all_zero { "all_zero" }
               else if b0 != 0 { "button" }
               else if b1|b2 != 0 { "motion" }
               else { "unknown" };
    }
}
```

Bounded: 3 volatile reads, one classify, inside the existing else-branch
(one event per outer iteration).  No forwarding, no IPC, no allocation.

## 2. Struct Update

Added `intr_report_len: u32` to `HidDevice` and both population points
(devices[0] and devices[1]).

## 3. Runtime Proof

```
C2B events:  15 (slot1 kbd — matching still works)
C2C events:  0  (expected — only 1 device connected, device_count=1)
Yield:       16 (budget 64)
Faults:      0
Windows:     Quil rendered
```

`ports.collect count=1` confirms single-device config.  C2C code is
gated behind `device_count > 1` and will activate when a second USB
device is connected to QEMU.

## 4. Build

```
./scripts/entrypoint_build.sh → PASS, 1767 sectors
```

## 5. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +30 lines: intr_report_len field, classify block, 2 population updates |
| `docs/handoff/SEXUSB_C2C_RESTART_BUDGETED_REPORT_FETCH_V1.md` | Created |

## 6. Invariants

| Check | Status |
|-------|--------|
| C2B match still works | ✅ 15 events |
| Yield still fires | ✅ 16 yields |
| No scheduler freeze | ✅ Windows render |
| Synthetic gate preserved | ✅ |
| C1 marker present | ✅ |
| No forwarding to sexinput | ✅ |

## 7. Next Step

To observe C2C slot2 events, connect both keyboard + tablet to QEMU.
This requires `dev.sh` support for multi-device (`SEXUSB_QEMU_DEVICE=kbd+tablet`)
or a manual QEMU command.

**USB_HID_POINTER_PRODUCER_V1** is unblocked once both devices are connected.

---

*End of SEXUSB_C2C_RESTART_BUDGETED_REPORT_FETCH_V1.md*
