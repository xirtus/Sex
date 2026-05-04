# KEYBOARD_DEVICE_MODE_V1

**Date:** 2026-05-04
**Status:** IMPLEMENTED

## Context

QEMU 11.0.0 host pointer backend does not deliver USB HID mouse/tablet motion.
The xHCI driver in sexusb is single-device/single-endpoint. Adding both mouse
AND keyboard simultaneously would require a significant xHCI refactor.

Instead, keyboard replaces mouse as the HID device in dev mode. Since mouse
only produces idle reports anyway, this loses no functionality.

## Implementation

### dev.sh

New device mode: `SEXUSB_QEMU_DEVICE=kbd`

```fish
env SEXUSB_QEMU_DEVICE=kbd SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run
```

Emits: `-device usb-kbd,bus=xhci.0` (replaces usb-mouse or usb-tablet)

### sexusb (servers/sexusb/src/main.rs)

- Added `OP_USB_KEYBOARD_REPORT = 0x261` (companion to 0x260)
- Config descriptor walk: detects HID boot keyboard (subclass=0x01, protocol=0x01)
- Uses same single-endpoint xHCI path as mouse/tablet
- Reads 8-byte boot keyboard reports
- Forwards to sexinput via `OP_USB_KEYBOARD_REPORT`
- arg0=reserved, arg1=modifiers, arg2=first keycode

### sexinput (servers/sexinput/src/main.rs)

- Added `OP_USB_KEYBOARD_REPORT = 0x261` handler
- Decodes 8-byte boot keyboard report
- Under `SEXOS_KEYBOARD_CURSOR=1` gate: maps USB HID usage IDs to EV_REL
- Key mapping (8px step):
  - 0x1a (W) / 0x52 (Up):    dx=0,  dy=-8
  - 0x16 (S) / 0x51 (Down):  dx=0,  dy=8
  - 0x04 (A) / 0x50 (Left):  dx=-8, dy=0
  - 0x07 (D) / 0x4f (Right): dx=8,  dy=0
- Gate unset = no behavior change
- Existing mouse path fully preserved

## Pipeline

```
key press -> QEMU usb-kbd -> xHCI interrupt-IN
  -> sexusb: 8-byte boot keyboard report
  -> OP_USB_KEYBOARD_REPORT -> sexinput
  -> decode HID usage ID -> map to EV_REL dx/dy
  -> OP_HID_EVENT (EV_REL) -> silk-shell
  -> POINTER_X/Y update -> OP_SURFACE_UPDATE -> sexdisplay
  -> cursor drawn at new position
```

## Diagnostic Markers (budget 16 each)

| Marker | Location | Purpose |
|--------|----------|---------|
| [sexusb.kbd.found] | sexusb | Keyboard interface found at config walk |
| [sexusb.kbd.raw] | sexusb | Raw report bytes on each interrupt |
| [sexusb.kbd.forward] | sexusb | Forwarded keycode to sexinput |
| [sexinput.kbd.recv] | sexinput | Keycode+modifiers received |
| [keyboard_cursor.gate] | sexinput | Gate enabled=1/0 at boot |
| [keyboard_cursor.key] | sexinput | Key matched to cursor movement |
| [keyboard_cursor.emit.rel] | sexinput | EV_REL sent to shell |

## Build

```fish
env SEXOS_KEYBOARD_CURSOR=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

## Run

```fish
env SEXUSB_QEMU_DEVICE=kbd SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run \
  2>/tmp/kbd-mode.err | tee /tmp/kbd-mode.out
```

## Expected Log Markers

```
[keyboard_cursor.gate] enabled=1 source=env
[sexusb.kbd.found] intf=0 ep=0x81
[sexusb.kbd.raw] b0=0x0 b2=0x1a b3=0x0 actual=8   ← pressing W
[sexusb.kbd.forward] key=0x1a
[sexinput.kbd.recv] key=0x1a mod=0x0
[keyboard_cursor.key] code=0x1a dx=0 dy=-8
[keyboard_cursor.emit.rel] dx=0 dy=-8
```

## Constraints Honored

- no_std, no heap allocation
- Single-device xHCI architecture (no refactor)
- No kernel/PDX/ABI changes (one new opcode 0x261)
- No sexdisplay/silk-shell changes
- Existing mouse/tablet path preserved for other device modes
- Gate unset = dead-code eliminated by const bool

## Files Changed

- servers/sexinput/src/main.rs (+130 lines: opcode, gate const, boot diagnostic, USB keyboard handler)
- servers/sexusb/src/main.rs (+86 lines: opcode, keyboard detection, decode/forward)
- dev.sh (+1 line: kbd device mode, help text update)
- docs/handoff/KEYBOARD_DEVICE_MODE_V1.md (this file)
- CLAUDE.md (small note)

## STOP Conditions

- [x] Builds with gate disabled: no behavior change
- [x] Builds with gate enabled: keyboard moves cursor
- [x] No kernel/PDX structural changes
- [x] No sexdisplay/silk-shell changes
- [x] Single-device xHCI architecture preserved
- [x] Budgeted diagnostic markers (16 each)
