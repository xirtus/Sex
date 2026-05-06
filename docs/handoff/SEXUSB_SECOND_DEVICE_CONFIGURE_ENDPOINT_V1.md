# SEXUSB_SECOND_DEVICE_CONFIGURE_ENDPOINT_V1

## Status: IMPLEMENTED

## Summary

Configure the slot2 usb-tablet interrupt-IN endpoint after SET_CONFIGURATION.
Allocate independent interrupt ring and report buffer, build endpoint context
in slot2 input context, issue Configure Endpoint command. No polling yet,
no pointer events sent.

## Changes

### `servers/sexusb/src/main.rs`

**1. Endpoint descriptor capture in slot2 walk** (modified existing TD4 walk):

Added three variables hoisted to outer scope (before TD4 block) so they're
available for Configure Endpoint:
```rust
let mut s2_intr_ep_addr: u8 = 0;
let mut s2_intr_ep_mps: u16 = 0;
let mut s2_intr_ep_interval: u8 = 0;
```

Added `s2_inside_hid_iface` tracking in the walk. When `b_type == 5`
(ENDPOINT descriptor) and inside a HID interface, captures interrupt-IN
endpoint attributes.

**2. Configure Endpoint phase** (after SET_CONFIGURATION):

Added ~90 lines that:
1. Validate `s2_intr_ep_addr` is an IN endpoint with valid MPS
2. Compute DCI from endpoint address: `ep_num * 2 + 1` for IN
3. Allocate `s2_intr_ring_phys/va` and `s2_intr_report_phys/va`
4. Set up 16-slot interrupt TR ring with Link TRB at slot 15 (TC=1)
5. Build input context in `s2_input_va` (reused from Address Device):
   - ICC add flags: slot (bit 0) + endpoint (bit = DCI)
   - Copy slot context from output device context
   - Update Context Entries in Slot Context DW0
   - Build endpoint context: interval, CERR, Interrupt-IN type, MPS,
     dequeue pointer (DCS=1), avg TRB len, max ESIT payload
6. Issue Configure Endpoint command TRB on shared command ring
7. Poll for Command Completion Event, validate `cc==SUCCESS && slot==s2_slot_id`

### Endpoint Context Values

| Field | Value | Source |
|-------|-------|--------|
| DW0 | Interval in bits 23:16 | From endpoint descriptor bInterval |
| DW1 | CERR=3, EP Type=7 (Interrupt-IN), MPS | From endpoint descriptor wMaxPacketSize |
| DW2-3 | Dequeue pointer = ring phys \| 1 (DCS=1) | Ring phys addr |
| DW4 | avg TRB len \|\| max ESIT payload | MPS in both halves |

### DCI Calculation

For EP1 IN (address 0x81):
- endpoint_number = addr & 0x0F = 1
- direction_in = (addr & 0x80) != 0 → true
- DCI = 1 * 2 + 1 = 3

## Markers

| Marker | Condition |
|--------|-----------|
| `[sexusb.slot2.ep.find] ep=N mps=N interval=N` | Interrupt-IN endpoint found in walk |
| `[sexusb.slot2.ep.ring.ok] phys=N va=N report_phys=N` | Ring+buffer allocated |
| `[sexusb.slot2.configure_endpoint.start] slot=N ep=N dci=N` | Before command TRB |
| `[sexusb.slot2.configure_endpoint.event.bad] cc=N slot=N` | Wrong completion event |
| `[sexusb.slot2.configure_endpoint.reject] reason=...` | Validation/alloc/command failure |
| `[sexusb.slot2.configure_endpoint.ok] slot=N ep=N dci=N` | Configuration confirmed |

## Resource Ownership

| Resource | Owner | Notes |
|----------|-------|-------|
| `s2_intr_ring_phys/va` | Slot2 | 4KB page, independent from first device's intr_ring |
| `s2_intr_report_phys/va` | Slot2 | Report buffer sized to MPS |
| `s2_input_va/phys` | Slot2 (reused) | Previously used for Address Device, now reused for Configure Endpoint |
| `cmd_ring` | Shared | Command ring shared with first device; cmd_idx advanced by 2 |
| `event_ring` | Shared | Events demuxed by slot ID |

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Add endpoint capture in walk + Configure Endpoint after SET_CONFIG |
| `docs/handoff/SEXUSB_SECOND_DEVICE_CONFIGURE_ENDPOINT_V1.md` | Created |

## Build

`./scripts/entrypoint_build.sh` PASS. 43 warnings (all pre-existing
style/marker warnings, 0 errors).

## Regression Check

| Check | Status |
|-------|--------|
| First device Configure Endpoint unchanged | ✅ Lines 3426+, not touched |
| First device poll loop unchanged | ✅ Lines 3442+, not touched |
| SingleHidBind unchanged | ✅ Not touched |
| Command ring shared correctly | ✅ cmd_idx advanced by 2 after slot2 cmd |
| Event ring shared correctly | ✅ Validated by slot ID |
| No polling for slot2 | ✅ Not added |
| No pointer events | ✅ Not sent |
| No array refactor | ✅ Local variables only |
| EP type constant accessible | ✅ `EP_TYPE_INTERRUPT_IN` defined later in function (Rust item scoping) |

## Next Phase

**SEXUSB_SECOND_DEVICE_POLL_V1**: Start interrupt polling for slot2
usb-tablet. Needs:

1. Queue Normal TRB on s2_intr_ring with s2_intr_report_va as buffer
2. Ring doorbell for slot2 interrupt-IN endpoint
3. Demux events by slot ID and endpoint ID
4. Route pointer reports (tablet absolute coordinates) to sexinput
   via `OP_USB_MOUSE_REPORT`
5. Either extend the single poll loop into an event demux, or add a
   second polling loop

Prerequisite: The current poll loop at ~line 3442 is single-device.
Multi-device event demux is needed before slot2 polling works.
