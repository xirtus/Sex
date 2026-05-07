# SEXUSB_EVENT_MATCH_HELPER_C2B_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED (build PASS)

## Summary

Added an inline event-match helper in the poll loop that checks each
Transfer Event against the `devices` table.  Logs `[sexusb.hid.demux.match]`
when slot2 (or slot1) is identified.  **Zero dispatch change** — slot2
events still fall through to the existing "bad event" path.

This is C2B of the three-patch demux plan.

---

## 1. Helper Location

**Insertion point:** Poll loop, right after Transfer Event slot/ep/cc
are decoded from the event ring, before the existing `slot == single_bind.slot_id`
match check.

**File:** `servers/sexusb/src/main.rs`, ~line 3767

## 2. Code Added (+23 lines)

```rust
// C2B: match event against device table (log only, no dispatch).
{
    for midx in 0..device_count {
        let d = &devices[midx];
        if d.active && slot == d.slot_id && ep == d.intr_dci {
            let kind = if d.role == HidRole::Keyboard { "keyboard" }
                  else if d.role == HidRole::PointerTablet { "tablet" }
                  else { "mouse" };
            unsafe {
                static mut DEMUX_MATCH_BUDGET: u32 = 32;
                let rem = &mut DEMUX_MATCH_BUDGET;
                if *rem > 0 {
                    *rem -= 1;
                    serial_println!(
                        "[sexusb.hid.demux.match] idx={} slot={} dci={} role={}",
                        midx, slot, ep, kind
                    );
                }
            }
            break;
        }
    }
}
```

## 3. New Marker

| Marker | Budget | Meaning |
|--------|--------|---------|
| `[sexusb.hid.demux.match] idx=N slot=N dci=N role=keyboard\|tablet` | 32 | Device table match found for this event |

## 4. Behavior (Unchanged)

| Path | Before C2B | After C2B |
|------|-----------|-----------|
| Slot1 keyboard event | `intr_ok = true`, dispatched | Same + optional match log |
| Slot2 tablet event | `[sexusb.xhci.intr_in.event.bad]`, not dispatched | Same + match log |
| Unrelated event | Consumed silently | Same |

Slot2 events still hit the "bad" path and are NOT forwarded to sexinput.
The match log proves the device table correctly identifies slot2.

## 5. Build Result

```
./scripts/entrypoint_build.sh → PASS, 1761 sectors, 0 errors
```

## 6. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +23 lines: inline match helper in poll loop |
| `docs/handoff/SEXUSB_EVENT_MATCH_HELPER_C2B_V1.md` | Created (this document) |

## 7. Invariants Preserved

| Check | Status |
|-------|--------|
| C1 marker `[sexusb.slot2.poll.start]` | ✅ |
| C2A marker `[sexusb.hid.devices.ready]` | ✅ |
| Poll loop structure unchanged | ✅ `SINGLE-DEVICE POLL LOOP` intact |
| Dispatch unchanged | ✅ `if is_keyboard_device { }` unchanged |
| Re-arm unchanged | ✅ Keyboard re-arm-before-IPC intact |
| Bad-event path unchanged | ✅ `intr_in.event.bad` still fires |
| kernel/sex-pdx/sexinput/shell/display | ✅ No edits |

## 8. Next Phase

**SEXUSB_SLOT2_CLASSIFY_NO_DISPATCH_C2C_V1** — In the else-branch of the
dispatch (which is dead code when slot1 is keyboard), check if the matched
device is slot2/tablet.  If so, read report from `devices[1].intr_report_va`,
decode, log markers.  Do NOT forward to sexinput yet.  ~25 lines.

---

*End of SEXUSB_EVENT_MATCH_HELPER_C2B_V1.md*
