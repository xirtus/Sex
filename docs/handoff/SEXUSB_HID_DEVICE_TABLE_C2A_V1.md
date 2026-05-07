# SEXUSB_HID_DEVICE_TABLE_C2A_V1

**Date:** 2026-05-07
**Status:** IMPLEMENTED (build PASS)

## Summary

Added a minimal `HidDevice` metadata struct and bounded two-slot array
for slot1 (keyboard) and slot2 (tablet).  Populated during enumeration.
**Zero behavior change** — poll loop, dispatch, re-arm, IPC all unchanged.

This is C2A of the three-patch demux plan from
`SEXUSB_SLOT2_EVENT_DEMUX_STOP_FIRST_V1.md`.

---

## 1. `HidDevice` Struct

Added after `SingleHidBind`, before `_start()`:

```rust
#[derive(Copy, Clone)]
struct HidDevice {
    active: bool,
    slot_id: u32,
    role: HidRole,
    intr_dci: u32,
    intr_report_phys: u64,
    intr_report_va: u64,
    intr_report_len: u32,
}
```

Minimal: only the fields needed for event matching + report reading.
No `intr_ring_va/phys`, no `intr_prod/pcs`, no `intr_ring_size` —
those stay in the poll loop (C2B/C2C will consume them when needed).

`const fn empty()` provides a zeroed sentinel for unpopulated slots.

---

## 2. Devices Array

Declared before `if target_port_count > 1` (SET_IDLE completion area):

```rust
let mut devices: [HidDevice; 2] = [HidDevice::empty(), HidDevice::empty()];
let mut device_count: usize = 0;
```

---

## 3. Population Points

### devices[1] — Slot2 (tablet)

Inside `if target_port_count > 1`, after C1 poll start TRB is queued:

```rust
devices[1] = HidDevice {
    active: true,
    slot_id: s2_slot_id,
    role: s2_hid_role,
    intr_dci: s2_intr_dci,
    intr_report_phys: s2_intr_report_phys,
    intr_report_va: s2_intr_report_va,
    intr_report_len: s2_intr_report_len,
};
device_count = 2;
```

Requires `s2_hid_role` hoisted to outer `if` scope (done; see diff).

### devices[0] — Slot1 (keyboard)

After `[sexusb.xhci.intr_in.config_ep.ok]`, before synthetic proof block:

```rust
devices[0] = HidDevice {
    active: true,
    slot_id: single_bind.slot_id,
    role: single_bind.role,
    intr_dci,
    intr_report_phys,
    intr_report_va,
    intr_report_len,
};
if device_count == 0 { device_count = 1; }
```

---

## 4. New Marker

| Marker | Budget | Meaning |
|--------|--------|---------|
| `[sexusb.hid.devices.ready] count=N slot1=S slot2=S` | 1 | Device table populated, poll loop entry imminent |

---

## 5. Build Result

```
./scripts/entrypoint_build.sh → PASS, 1761 sectors, 0 errors
```

---

## 6. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | +67 -1: HidDevice struct, devices array, 2x population, ready marker, s2_hid_role hoist |
| `docs/handoff/SEXUSB_HID_DEVICE_TABLE_C2A_V1.md` | Created (this document) |
| `docs/handoff/SEXUSB_SLOT2_EVENT_DEMUX_STOP_FIRST_V1.md` | Reference (C2 abort analysis) |

---

## 7. Invariants Preserved

| Check | Status |
|-------|--------|
| C1 marker `[sexusb.slot2.poll.start]` present | ✅ |
| Poll loop unchanged | ✅ `SINGLE-DEVICE POLL LOOP` comment intact |
| No dispatch changes | ✅ `if is_keyboard_device { ... }` unchanged |
| No re-arm changes | ✅ Keyboard re-arm-before-IPC intact |
| No IPC changes | ✅ `pdx_call_checked` paths unchanged |
| kernel/sex-pdx untouched | ✅ |
| sexinput/silk-shell/sexdisplay untouched | ✅ |
| Single-device mode preserved | ✅ `device_count==1` when only slot1 |

---

## 8. Next Phase

**SEXUSB_EVENT_MATCH_HELPER_C2B_V1** — Add an event-match helper that
identifies which device a Transfer Event belongs to.  Log a marker
when slot2 is matched.  Do NOT dispatch — fall through to existing
"bad event" path.  ~20 lines, no behavior change.

---

*End of SEXUSB_HID_DEVICE_TABLE_C2A_V1.md*
