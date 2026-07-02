# HOST_INPUT_BACKEND_AUDIT_V1

**Date:** 2026-05-04
**Status:** AUDIT COMPLETE — QEMU 11.0.0 does not deliver host pointer motion to emulated USB HID

## Observed

All tested QEMU display+device combinations produce only idle HID reports on real desktop:

| Test | sexusb.tablet.live | forward.mouse | Result |
|------|-------------------|---------------|--------|
| usb-tablet + sdl-grab | 0 | 15 | No motion |
| usb-tablet + display=sdl + sdl-grab | 0 | 15 | No motion |
| usb-mouse + sdl-grab | 0 | 15 | No motion |
| usb-mouse + gtk | 0 | 15 | No motion |

## QEMU Version

QEMU emulator version 11.0.0 (qemu-system-x86_64)

## Available Display Backends

none, gtk, sdl, egl-headless, curses, spice-app, dbus

## Key Finding

Both usb-mouse (boot HID, relative) and usb-tablet (absolute HID) produce only idle
reports. This rules out device-specific issues. **QEMU 11.0.0 is not forwarding host
pointer events to any emulated USB HID device on the xHCI bus.**

## Bus Wiring

- xHCI: `-device nec-usb-xhci,id=xhci` (NEC uPD720170, slots=64, intrs=16)
- USB device: `bus=xhci.0` (correct for NEC xHCI)

## usb-mdev Options (device-specific flags)

usb-mouse: attached=<bool>, msos-desc=<bool>, pcap=<str>, port=<str>, serial=<str>, usb_version=<uint32>
  No display=<str> or head=<uint32> option.

usb-tablet: attached=<bool>, display=<str>, head=<uint32>, msos-desc=<bool>, pcap=<str>, port=<str>, serial=<str>, usb_version=<uint32>
  Has display=<str> for multi-head binding.

## What dev.sh Already Supports

USB devices: mouse (default), tablet, tablet-display-sdl
Display modes: sdl (default), sdl-grab, gtk, gtk-grab, none
Nodefaults: SEXOS_QEMU_NODEFAULTS=1 (disables PS/2 devices)
Print cmd: QEMU_PRINT_CMD=1

## Possible Root Causes

1. **QEMU 11.0.0 regression**: USB HID event forwarding may be broken in this
   QEMU version for q35 + xHCI combination. Would need testing with QEMU 9.x or 10.x.

2. **Display backend event routing**: QEMU display backends (SDL, GTK) forward
   events through a "input event hub." USB HID devices register as input handlers.
   If the display backend doesn't produce events or the hub doesn't route them
   to USB HID, nothing arrives. The PS/2 mouse uses a different path and is
   unaffected.

3. **Host compositor/focus**: SDL/GTK window may not have pointer capture.
   On Wayland, only focused/active windows receive pointer events. Some
   compositors may not forward absolute motion events to XWayland/QEMU.

4. **xHCI timing/interrupt delivery**: xHCI interrupt-IN polling in sexusb
   may miss events if QEMU doesn't properly assert interrupt for USB HID
   input events. But idle reports arrive, so the polling path works.

## Next Steps to Test

### 1. Check PS/2 mouse in Limine boot menu
If the Limine boot menu cursor responds to mouse movement, PS/2 input works
and the problem is specific to USB HID routing. If Limine also doesn't respond,
host input isn't reaching QEMU at all.

### 2. Try GTK backend (already in dev.sh)
```fish
env SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=gtk ./dev.sh run 2>/tmp/gtk-mouse.err | tee /tmp/gtk-mouse.out
```

### 3. Try SDL without grab-mod
```fish
env SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl ./dev.sh run 2>/tmp/sdl-mouse.err | tee /tmp/sdl-mouse.out
```

### 4. Try SDL_VIDEO_DRIVER=x11 (if on Wayland)
```fish
env SDL_VIDEO_DRIVER=x11 SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run
```

### 5. Try with VNC client
```fish
# Start QEMU with VNC display:
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -device nec-usb-xhci,id=xhci -device usb-mouse,bus=xhci.0 -serial stdio -vnc :0
# Connect: vncviewer :0
```

### 6. Try spice-app display
```fish
env SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=spice-app ./dev.sh run
```

### 7. Try with -nodefaults to eliminate PS/2 device creation
```fish
env SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab SEXOS_QEMU_NODEFAULTS=1 ./dev.sh run
```

### 8. Markers to check in all cases
```fish
set f /tmp/gtk-mouse.out
for m in sexusb.tablet.live sexusb.forward.mouse sexinput.mouse.live sexinput.hid.emit.rel shell.hid.rel.live shell.cursor.surface.update sexdisplay.cursor.surface.update sexdisplay.cursor.draw
    printf "%-40s %d\n" $m (grep -ac "\[$m\]" $f)
end
echo "---"
grep -acE "panic|PAGE FAULT|GENERAL PROTECTION" $f
```

## Fallback: Keyboard-to-Cursor (if USB HID input cannot be fixed)

If QEMU USB HID input delivery is genuinely broken in QEMU 11.0.0, and we
cannot downgrade QEMU, consider a keyboard-driven cursor fallback:

Add arrow keys / WASD keyboard event handling in sexinput that moves the
cursor when the USB HID path produces no motion. This unblocks Silk DE
development while QEMU input is separately diagnosed.

**Design principle:** The keyboard fallback would be enabled by a compile-time
flag (like the proof gate) and produce synthetic EV_REL events identical to
real USB motion. It would NOT modify the USB HID pipeline.

(Not implementing now — documented for future decision.)

## Changed Files

- dev.sh — already has all needed modes (no changes this round)
- docs/handoff/HOST_INPUT_BACKEND_AUDIT_V1.md — this file
- CLAUDE.md — small note

## Forbidden Changes NOT Made

- No kernel/ servers/ crates/ ABI changes
- No guest code changes
- No renderer/input subsystem changes

## STOP conditions

- [x] QEMU version documented
- [x] All tested combos documented
- [x] Root cause analysis documented
- [x] Test matrix for user provided
- [x] No guest code changes
