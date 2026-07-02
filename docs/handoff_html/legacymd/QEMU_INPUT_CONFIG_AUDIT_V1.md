# QEMU_INPUT_CONFIG_AUDIT_V1

**Date:** 2026-05-04
**Status:** AUDIT COMPLETE

## Observed Symptom

On real local desktop with physical trackpad:
- `SEXOS_PROOFS_DISABLED=1` built correctly
- Run: `SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run`
- User physically moved trackpad in SDL window for 10 seconds
- Result: `sexusb.tablet.live = 0`, `sexusb.forward.mouse = 15`
- **QEMU does not deliver non-idle coordinates to usb-tablet device**

## Guest Pipeline Status

Already proven healthy in TABLET_LIVENESS_TRACE_V1:
- 15 idle reports forwarded from sexusb -> sexinput (all dx=0, dy=0)
- sexinput normalizer correctly suppresses EV_REL when zero delta
- shell and display waiting for EV_REL that never arrives
- Zero panics, page faults, or protection faults
- **Dead layer is outside the guest - QEMU input delivery**

## QEMU Version

QEMU emulator version 11.0.0

## dev.sh Changes (this audit)

Added to dev.sh:

| Variable | Values | Effect |
|----------|--------|--------|
| QEMU_PRINT_CMD=1 | flag | Print exact QEMU argv, do not launch |
| SEXUSB_QEMU_DEVICE | mouse, tablet, tablet-display-sdl | USB HID device on xHCI bus |
| SEXOS_QEMU_DISPLAY | sdl, sdl-grab, gtk, gtk-grab, none | Display backend |
| SEXOS_QEMU_NODEFAULTS=1 | flag | Add -nodefaults (disables PS/2 input) |
| SEXOS_QEMU_INPUT_INJECT=1 | flag | Enable QMP socket |
| SEXUSB_XHCI_TRACE=1 | flag | Enable xHCI trace events |

## Audit Results

### 1. usb-tablet display= suboption

The usb-tablet device has a display=<str> suboption for multi-head display binding.
Confirmed valid (QEMU 11.0.0 accepts without error):
- device usb-tablet,bus=xhci.0 (no display= - current default)
- device usb-tablet,bus=xhci.0,display=sdl
- device usb-tablet,bus=xhci.0,display=gtk
- device usb-tablet,bus=xhci.0,head=0

Caveat: QEMU does not validate the display= value at startup. Invalid values
like display=invalid produce no error. The binding is only checked at runtime
when events arrive.

### 2. xHCI bus wiring

The xHCI controller is created with device nec-usb-xhci,id=xhci.
The usb-tablet is connected with bus=xhci.0.
This is correct - xhci.0 is the default bus name for the xhci controller.

### 3. Default PS/2 device conflict

QEMU q35 machine creates default PS/2 keyboard+mouse on the I8042 controller.
These are separate from USB xHCI bus. Should not conflict with usb-tablet.
-nodefaults removes all default devices including PS/2 input.
Tested and proven to boot successfully.

### 4. SDL grab behavior

-display sdl,grab-mod=lctrl-lalt requires pressing LCtrl+LAlt to grab mouse.
For usb-tablet, absolute coordinates should be forwarded without grab,
but some QEMU versions or display configs may not deliver events until
the window has input focus or pointer is grabbed.

### 5. Key observation: grab-mod=lctrl-lalt

The sdl-grab mode means mouse is NOT grabbed until LCtrl+LAlt is pressed.
For usb-mouse (relative mode): events only arrive after grab.
For usb-tablet (absolute mode): events should arrive without grab,
but this may depend on QEMU version and SDL backend.

## Local Test Matrix

Run each with QEMU_PRINT_CMD=1 first, then remove to launch.

### Test 1: tablet + sdl-grab (current config)
env SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run

### Test 2: tablet-display-sdl + sdl-grab
env SEXUSB_QEMU_DEVICE=tablet-display-sdl SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run

### Test 3: tablet + sdl-grab + nodefaults
env SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab SEXOS_QEMU_NODEFAULTS=1 ./dev.sh run

### Test 4: usb-mouse + sdl-grab (relative, needs grab)
env SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run

### Test 5: tablet + plain sdl (no grab-mod)
env SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl ./dev.sh run

### Test 6: tablet-display-sdl + sdl-grab + nodefaults
env SEXUSB_QEMU_DEVICE=tablet-display-sdl SEXOS_QEMU_DISPLAY=sdl-grab SEXOS_QEMU_NODEFAULTS=1 ./dev.sh run

## Recommended Verdict

### If any test produces sexusb.tablet.live > 0:
The problem is limited to the specific QEMU flag combination.

### If ALL tests produce sexusb.tablet.live = 0:
Problem is outside QEMU flags. Potential causes:
1. SDL window not receiving host input events (check SDL_VIDEO_DRIVER=x11)
2. Display manager compositor blocking input forwarding
3. QEMU 11.0.0 SDL backend bug with usb-tablet
4. Host trackpad produces absolute events SDL doesn't forward as tablet coords

## Fish Shell Commands

Print command only (no launch):
env QEMU_PRINT_CMD=1 SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run

Run interactive test:
env SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/tmp/tablet-live-interactive.trace | tee /tmp/tablet-live-interactive.log

Count markers after run:
for m in sexusb.tablet.live sexusb.forward.mouse sexinput.mouse.live sexinput.hid.emit.rel shell.hid.rel.live shell.cursor.surface.update sexdisplay.cursor.surface.update sexdisplay.cursor.draw
    printf "%-40s %d\n" $m (grep -ac "\[$m\]" /tmp/tablet-live-interactive.log)
end

## Changed Files (diagnostic only)

- dev.sh - added QEMU_PRINT_CMD=1, tablet-display-sdl, QEMU_NODEFAULTS=1, display=none, help text
- docs/handoff/QEMU_INPUT_CONFIG_AUDIT_V1.md - this file
- CLAUDE.md - small note

## Forbidden Changes NOT Made

- No kernel/ changes
- No crates/sex-pdx/ changes
- No servers/ changes
- No renderer/input logic changes
- No ABI changes
- No guest code changes

## STOP conditions met

- [x] QEMU_PRINT_CMD=1 works for all device/display combos
- [x] usb-tablet display= suboption confirmed valid
- [x] -nodefaults confirmed bootable
- [x] No server/kernel/ABI changes
- [x] No guest code changes
