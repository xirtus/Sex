# SEXUSB_HID_MULTIDEVICE_POINTER_AUDIT_V1

## Status: COMPLETE (docs only, no code changes)

## Goal

Audit why sexusb only detects the QEMU `usb-kbd` and misses the QEMU
`usb-tablet` when both are connected to the same XHCI controller. Identify
the exact enumeration limitation and the smallest safe patch path.

---

## 1. Topology: QEMU nec-usb-xhci Port Model

### QEMU Command

```
-device nec-usb-xhci,id=xhci
-device usb-kbd,bus=xhci.0
-device usb-tablet,bus=xhci.0
```

### Port Layout (from QEMU `xhci_init`)

QEMU's `nec-usb-xhci` allocates two virtual port ranges backed by shared
physical `uport` connectors:

| uport | USB3 Port (SS only) | USB2 Port (FS/LS/HS) |
|-------|---------------------|----------------------|
| 0     | port=1, speedmask=SS | port=5, speedmask=FS+LS+HS |
| 1     | port=2, speedmask=SS | port=6, speedmask=FS+LS+HS |
| 2     | port=3, speedmask=SS | port=7, speedmask=FS+LS+HS |
| 3     | port=4, speedmask=SS | port=8, speedmask=FS+LS+HS |

Source (`tools/qemu/hw/usb/hcd-xhci.c`, `xhci_init`):

```c
for (i = 0; i < usbports; i++) {
    if (i < numports_2) {
        port = &ports[i + numports_3];
        port->portnr = i + 1 + numports_3;  // 5, 6, 7, 8
        port->speedmask = USB_SPEED_MASK_LOW | USB_SPEED_MASK_FULL | USB_SPEED_MASK_HIGH;
    }
    if (i < numports_3) {
        port = &ports[i];
        port->portnr = i + 1;  // 1, 2, 3, 4
        port->speedmask = USB_SPEED_MASK_SUPER;
    }
    usb_register_port(&bus, &uports[i], ...);
}
```

### Device Assignment

Both `usb-kbd` (Full Speed) and `usb-tablet` (Full Speed) are non-SS
devices. They claim a USB2 port via `usb_claim_port` first-free:

1. **usb-kbd** (created first) → uport[0] → `ports[4]` portnr=5, CCS=1
2. **usb-tablet** (created second) → uport[1] → `ports[5]` portnr=6, CCS=1

**Both ports should show CCS=1 simultaneously after XHCI reset** (QEMU
`xhci_reset` calls `xhci_port_update` for every port, setting CCS for
each uport that has an attached device with matching speed).

---

## 2. sexusb Port Scan Code

### Location

`servers/sexusb/src/main.rs` lines 726–755.

### Logic

```rust
let mut ports_connected: u32 = 0;
for port in 1..=max_ports {
    let portsc = mmio_read32(op_base, portsc_off);
    // log all ports
    if connected == 1 {
        ports_connected = ports_connected.wrapping_add(1);  // counts correctly
    }
    if (portsc & PORTSC_CCS) != 0 {
        target_port = port;        // selects FIRST connected port only
        port_speed = (portsc >> 10) & 0xF;
        break;                     // <--- STOPS HERE
    }
}
```

### Two Distinct Issues

| Issue | Description | Severity |
|-------|-------------|----------|
| **A. `break` on first connected** | Even if multiple ports have CCS=1, only the first (lowest port number) is selected. All subsequent ports are skipped. | Definite |
| **B. Single-port evidence** | The handoff log shows `ports_connected=1` — only 1 port had CCS=1 at scan time. This means QEMU only offers one device initially, OR the second device appears later. | Observed |

**Issue A** is a definite bug: if two devices are both visible, only the
first one is enumerated.

**Issue B** is a QEMU behavior observation. Possible explanations:
- QEMU's XHCI model may not mark the second port as connected until the
  first device is addressed and configured
- The port status change for the second device may be event-driven and
  not reflected in PORTSC until the event ring is set up and running
- This is consistent with how real XHCI hardware behaves: some controllers
  enumerate devices sequentially, revealing the second device only after
  the first is fully configured

**Resolution for Issue B**: Even if QEMU only reveals one device at a
time, the driver must eventually scan again (after the first device is
configured) to discover the second device. Currently it never does.

---

## 3. Single-Device Architecture (not just port scan)

The port scan `break` is the first limitation, but the entire driver
is structured as single-device:

| Step | Current Behavior | Multi-Device Needed |
|------|------------------|---------------------|
| Enable Slot | 1 command at line 652 | N commands (one per device) |
| Port Select | 1 `target_port` at line 750 | Array of target ports |
| Address Device | 1 at line 925 | N (one per slot) |
| Device Descriptor | 1 DMA buffer | N buffers or reuse |
| Config Descriptor | 1 fetch + walk | N (one per device) |
| HID Binding | 1 `SingleHidBind` | Array of bindings |
| Interrupt Ring | 1 TR + 1 report buffer | N rings + N buffers |
| Poll Loop | 1 `loop { ... }` at line 2743 | Event demux across N endpoints |

Every structure is dimensioned for exactly one device:

```rust
struct SingleHidBind {          // one binding
    slot_id: u32,
    port: u64,
    role: HidRole,
    ep_addr: u8,
    ep_dci: u32,
    max_packet: u16,
    interval: u8,
}
```

---

## 4. HID Classification (would work correctly for QEMU usb-tablet)

### Current Classification Logic (lines 1994–2020)

```rust
let is_hid = b_class == 0x03;  // HID device class
if is_hid {
    iface_is_boot_mouse = (b_subclass == 0x01) && (b_protocol == 0x02);
    let is_boot_keyboard = (b_subclass == 0x01) && (b_protocol == 0x01);
    if iface_is_boot_mouse {
        current_hid_role = 2;  // found_hid_mouse
    } else if is_boot_keyboard {
        current_hid_role = 1;  // found_hid_keyboard
    } else if b_protocol != 0x01 {
        current_hid_role = 3;  // found_hid_tablet  <-- QEMU usb-tablet lands here
    } else {
        current_hid_role = 4;  // unknown
    }
}
```

### QEMU usb-tablet interface descriptor

The QEMU usb-tablet device presents:
- bInterfaceClass = 0x03 (HID)
- bInterfaceSubClass = 0x00 (no boot interface)
- bInterfaceProtocol = 0x00 (none)

This matches the `b_protocol != 0x01` condition → `found_hid_tablet = true`
→ `current_hid_role = 3` → binds as `HidRole::PointerTablet`.

**Conclusion**: The HID classification is correct and would detect the
tablet IF its configuration descriptor were fetched and walked. The
barrier is not classification — it's that the descriptor is never fetched.

### Tablet Decode Path (already implemented)

Lines 3002–3088: full tablet decoding, absolute coordinate reporting to
sexinput via `OP_USB_MOUSE_REPORT`. The `decode_tablet_report()` function
correctly handles the 5-byte QEMU usb-tablet HID report:

```
byte 0: buttons[2:0]
byte 1-2: X absolute (u16 LE, 0..32767)
byte 3-4: Y absolute (u16 LE, 0..32767)
```

---

## 5. What Would Need to Change (Multi-Device Architecture)

### Structural Changes Required

```rust
// Current: single-device
struct SingleHidBind { ... }
let single_bind: SingleHidBind;

// Needed: multi-device
const MAX_HID_DEVICES: usize = 4;
struct HidDevice {
    slot_id: u32,
    port: u64,
    role: HidRole,
    ep_addr: u8,
    ep_dci: u32,
    max_packet: u16,
    interval: u8,
    // per-device resources
    intr_ring_va: u64,
    intr_ring_phys: u64,
    intr_report_va: u64,
    intr_report_phys: u64,
    intr_prod: u64,
    intr_pcs: u32,
}
let devices: [Option<HidDevice>; MAX_HID_DEVICES];
```

### Per-Device Resource Allocation

Each HID device needs its own:
- XHCI slot (from Enable Slot command)
- Input context page (Address Device)
- Device context page
- EP0 transfer ring
- Interrupt transfer ring (TRB ring)
- Interrupt report buffer
- Descriptor data buffer (can be reused, but must be serialized)

### Multi-Device Port Scan

```rust
let mut target_ports: [u64; MAX_PORTS] = [0; MAX_PORTS];
let mut num_targets: u32 = 0;
for port in 1..=max_ports {
    if (portsc & PORTSC_CCS) != 0 {
        if num_targets < MAX_PORTS {
            target_ports[num_targets as usize] = port;
            num_targets += 1;
        }
    }
}
// No break — collect all connected ports.
```

### Multi-Device Poll Loop

The single `loop { wait_for_event; decode_device; forward_report; }`
must become an event demux:

```rust
loop {
    // Wait for transfer event on any slot/endpoint
    let (slot, ep) = wait_for_transfer_event(&event_ring, ...);
    // Dispatch to the correct device handler
    if let Some(dev) = find_device(slot, ep) {
        match dev.role {
            HidRole::Keyboard => handle_keyboard_report(dev),
            HidRole::PointerTablet => handle_tablet_report(dev),
            HidRole::PointerMouse => handle_mouse_report(dev),
            HidRole::Unknown => {}
        }
    }
}
```

---

## 6. Smallest Safe Patch Path

### Phase 1: [comment-only] Document port scan limitation

Add a comment at the port scan `break` acknowledging that only the first
connected port is selected and multi-device is not yet supported.

**Risk**: None (comment only).
**Value**: Prevents future developers from assuming multi-device works.

### Phase 2: [minimal code] Fix port scan `break`

Remove the `break` and collect all connected port numbers into a small
array. Add a `MAX_USB_DEVICES` constant.

**Risk**: Low. The loop still works for 1 device. No functional change
until downstream code processes the array.
**Lines changed**: ~10 lines in the port scan loop.

### Phase 3: [larger] Multi-slot device enumeration

Extract the existing single-device enumeration sequence
(Enable Slot → Address Device → GET_DESCRIPTOR → HID walk → SET_CONFIG →
Configure Endpoint → Poll) into a function, then call it in a loop for
each connected port.

**Risk**: Medium. The enumeration sequence is ~2000 lines of sequential
code with many `loop { sys_yield(); }` spin waits. Extracting it without
introducing state leaks requires careful scoping.

### Phase 4: [largest] Multi-device event poll

Convert the single `loop { wait; handle; rearm }` into an event demux
that can handle interrupt transfers from multiple endpoints.

**Risk**: Medium-high. The current poll loop has subtle re-arm timing
(especially the keyboard path which re-arms before IPC at line 2898).
The multi-device loop must not starve one device while another's IPC
is in flight.

---

## 7. Recommended Next Step

**Phase 1 (comment only) + Phase 2 (port scan fix)** are safe to do
immediately. They establish the data structure for multi-device without
changing behavior.

Phase 3 and 4 are significant refactors of the ~3000-line `main()`
function. Recommended approach:

1. **After Phase 2**: Add `MAX_USB_DEVICES` and `target_ports[]` array.
   Verify single-device behavior is unchanged (the array has 1 entry).
2. **Phase 3 substep A**: Extract Enable Slot + Address Device into a
   helper that takes a port number and returns a slot ID.
3. **Phase 3 substep B**: Extract GET_DESCRIPTOR + HID bind into a helper
   that takes a slot ID and returns a `HidRole`.
4. **Phase 3 substep C**: Extract Configure Endpoint + poll setup into a
   helper that takes a slot + role and starts interrupt polling.
5. **Phase 4**: Replace the single poll loop with an event demux.

This phased approach keeps each change reviewable and testable.

---

## 8. Summary

| Finding | Evidence | Severity |
|---------|----------|----------|
| Port scan `break`s on first connected port | Lines 749–753 | Definite bug |
| `target_port` is a single value | Line 731 | Definite limitation |
| Single Enable Slot issued | Line ~652 | Definite limitation |
| Single Address Device issued | Line ~925 | Definite limitation |
| Single `SingleHidBind` struct | Line 2162 | Definite limitation |
| Single interrupt ring + report buffer | Lines 2544–2546 | Definite limitation |
| Single poll loop handles one device | Line 2743 | Definite limitation |
| HID classification correct for QEMU tablet | Lines 2011–2017 | ✅ Correct |
| Tablet decode path fully implemented | Lines 3002–3088 | ✅ Correct |
| QEMU usb-tablet Forward to sexinput | Lines 3067–3073 | ✅ Correct |

**Root cause**: sexusb is architecturally a single-device HID driver.
It can handle one HID device (keyboard OR tablet/mouse) but cannot
enumerate a second device on a different port.  The port scan `break`
is the first gate, but even if removed, the downstream code lacks the
data structures and event demux for multiple devices.

**The pointer would work** IF the second device were enumerated, because
the HID classification correctly identifies QEMU usb-tablet as a pointer
device and the absolute coordinate decode/forward path is fully
implemented and tested (confirmed by the tablet liveness markers).

**No patch in this audit.**  The fix requires either a multi-device
architecture (Phase 3+4) or a different approach (keyboard cursor
fallback, synthetic pointer, or a second sexusb instance).
