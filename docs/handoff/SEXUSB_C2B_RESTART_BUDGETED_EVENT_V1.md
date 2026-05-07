# SEXUSB_C2B_RESTART_BUDGETED_EVENT_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED — build + runtime PASS

## Summary

Re-applied C2B (device table + event match helper) under the budgeted
yield poll loop.  Bounded: one match check per outer iteration, inside
the existing inner event loop.  Yield fires between iterations.

## 1. What Was Added

| Component | Lines | Hot-path? |
|-----------|-------|-----------|
| `HidDevice` struct (minimal: 5 fields) | +16 | No (compile-time) |
| `devices` array + `device_count` | +3 | No (init only) |
| `devices[1]` population (slot2) | +3 | No (init, inside `if target_port_count > 1`) |
| `devices[0]` population (slot1) | +2 | No (init, after config_ep.ok) |
| `s2_hid_role` hoist | +1 | No (init) |
| C2B match helper in inner event loop | +13 | **Yes — one bounded loop over <=2 devices** |

## 2. C2B Match Helper

```rust
// Inside inner event loop, after slot/ep extraction:
for midx in 0..device_count {
    let d = &devices[midx];
    if d.active && slot == d.slot_id && ep == d.intr_dci {
        // budgeted log (16)
        serial_println!("[sexusb.c2b.event.seen] idx={} kind={}", midx, ...);
        break;
    }
}
```

Bounded: iterates at most 2 devices. No descriptor walks. No HID parsing.
Runs once per Transfer Event (one per outer iteration).

## 3. Runtime Proof

```
C2B match events:  15 (all idx=0 kbd — slot1 keyboard, expected with kbd-only QEMU)
Yield markers:     16 (budget 64)
Faults:            0
Windows:           Quil rendered (quil.boot.draw.ok)
Slot2 events:      0 (expected — only one device connected with SEXUSB_QEMU_DEVICE=kbd)
```

## 4. Build

```
./scripts/entrypoint_build.sh → PASS, 1766 sectors
```

## 5. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +40 lines: struct + array + populations + match helper |
| `docs/handoff/SEXUSB_C2B_RESTART_BUDGETED_EVENT_V1.md` | Created |

## 6. Invariants

| Check | Status |
|-------|--------|
| Yield still fires | ✅ 16 yields in 25s |
| No scheduler freeze | ✅ Windows render |
| Synthetic gate preserved | ✅ Code intact |
| C1 marker present | ✅ |
| Keyboard path unchanged | ✅ All events idx=0 kbd |

## 7. C2C Unblocked

C2B is proven safe under budgeted yield. C2C (slot2 report read + classify,
no forwarding) can now be re-applied as a bounded extension of the
else-branch in the dispatch — one volatile read per slot2 event, no scans.

**C2C_RESTART_BUDGETED_REPORT_FETCH** is unblocked.

---

*End of SEXUSB_C2B_RESTART_BUDGETED_EVENT_V1.md*
