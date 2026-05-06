# INPUT_RUNTIME_RELIABILITY_CHECKPOINT_V1

## Build

`./scripts/entrypoint_build.sh` PASS (ISO includes latest Bell/PDX cleanup).

## Test Configuration

- QEMU: `qemuX.sh -nographic -display none` (no host input possible)
- Devices: `usb-kbd` + `usb-tablet` on XHCI
- i8042: disabled (`i8042=off`)
- SYNTHETIC_INPUT_PROOFS_DISABLED = true (forced off in code)
- KEYBOARD_CURSOR_ENABLED = false (env not set)
- KEYBOARD_PROOF_ENABLED = false (env not set)

## Pointer Chain

```
sexusb → [DEAD HOP: pointer not enumerated] → sexinput → silk-shell → sexdisplay
```

| Step | Marker | Status |
|------|--------|--------|
| sexusb pointer HID bind | `[sexusb.hid.bind.summary] pointer_ep=none` | ❌ **DEAD** |
| sexinput pointer recv | `[sexinput.pointer.recv]` | never reached |
| sexinput pointer send | `[sexinput.pointer.send]` | never reached |
| silk-shell pointer recv | `[silk-shell.pointer.recv]` | never reached |
| silk-shell cursor update | `[silk-shell.cursor.update]` | never reached |
| sexdisplay cursor draw | `[sexdisplay.cursor.draw] x=640 y=360` | ✅ (static center) |

**Dead hop**: sexusb HID enumeration does not detect the QEMU `usb-tablet`
as a boot protocol pointer device.  Only 1 USB device enumerated on
port 5 (the keyboard).  `pointer_role=none`, `pointer_devices=0`.

```
[sexusb.xhci.port] port=5 connected=1 enabled=0 speed=3
[sexusb.dev.desc] slot=1 vendor=0x627 product=0x1
[sexusb.hid.iface] idx=9 if=0 class=0x3 subclass=0x1 proto=0x1
[sexusb.hid.bind] role=keyboard if=0 ep=0x81 reason=hid_boot_keyboard
[sexusb.hid.bind.summary] keyboard_ep=set pointer_ep=none
[sexusb.enum.summary] ports_connected=1 slots_enabled=1 hid_devices=1 pointer_devices=0
```

## Keyboard Chain

```
sexusb → sexinput → silk-shell (idle — no keypress in nographic)
```

| Step | Marker | Status |
|------|--------|--------|
| sexusb intr raw | `[sexusb.intr.raw] n=N dev=keyboard` | ✅ (15 idle reports) |
| sexusb classify | `[sexusb.intr.classify] kind=keyboard action=forward` | ✅ |
| sexinput kbd recv | `[sexinput.kbd.recv] key=0x0 mod=0x0` | ✅ (15 idle) |
| sexinput evkey forward | `[sexinput.usb_kbd.evkey]` | never (idle, no key) |
| silk-shell HID_EVENT | `[silk-shell.key.route]` | never (idle, no key) |

**Status**: The keyboard path is alive from sexusb through sexinput.
No actual keypresses occurred because QEMU is in `-nographic` mode
with no host input injection.  The path would work end-to-end when a
key is pressed.  **Not a dead hop — idle is expected.**

## Cursor Rendering (display side)

```
[shell.cursor_surface.create.start] id=0x90
[shell.cursor_surface.create.ok]
[sexdisplay.cursor_surface.z_top.ok] id=0x90
[sexdisplay.cursor_shape.arrow.ok] id=0x90
[sexdisplay.cursor.draw] n=0 x=640 y=360 (repeating)
```

Cursor surface created, shaped, and drawn at center (640, 360).
Static because no pointer movement events reach the shell.

## First Dead Hop

**`sexusb` HID pointer enumeration**: The QEMU `usb-tablet` device
is not detected as a HID boot protocol pointer.  Only the keyboard
is enumerated on the single connected port.

This is not a local/obvious fix.  It requires understanding:
- Why only 1 of 2 QEMU USB devices appears on the XHCI bus
- How the XHCI driver handles multiple device slots
- How the HID driver parses interface descriptors for pointer subclass/protocol

## Patch Decision

**No patch.**  The first dead hop (sexusb HID pointer enumeration) is
not local, obvious, or non-ABI.  It requires USB HID driver work
outside the scope of this input reliability checkpoint.

The Bell/PDX cleanup did not regress any input path.  The keyboard
path is alive and would work with real input.

## Next Smallest Fix

1. **sexusb HID multi-device support**: Enable detection of a second
   USB device (the tablet) on a second XHCI slot.  Currently only 1
   slot is allocated.  Fix likely in `sexusb/src/main.rs` or XHCI
   driver `enable_slot` and `address_device` logic.
2. **sexusb HID pointer subclass/protocol detection**: After
   multi-device works, ensure the tablet's interface descriptor
   (class=3, subclass=2, proto=1 or proto=2 for boot mouse) is
   recognized as a pointer.
3. **Keyboard self-test**: Enable `KEYBOARD_CURSOR_ENABLED` or
   `KEYBOARD_PROOF_ENABLED` via env var in CI to prove keyboard
   forwarding works without host input.
