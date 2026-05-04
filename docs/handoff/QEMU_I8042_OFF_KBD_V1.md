# QEMU_I8042_OFF_KBD_V1

## Status: QEMU 11.0.0 USB HID Input Routing Blocked

### Finding

QEMU 11.0.0 on this host has a fundamental USB HID input routing bug.
Even with `-machine q35,i8042=off` to disable the built-in PS/2/i8042
controller AND `-device usb-kbd,bus=xhci.0,display=sdl` to bind the USB
keyboard to the SDL display, host keyboard events do NOT reach the
emulated USB keyboard. The xHCI driver polls the interrupt endpoint
correctly — 8-byte HID boot protocol reports arrive every tick — but
all key bytes are zero (idle reports). QEMU is not injecting keystrokes
into the USB HID device's report buffer.

### What Works (Guest Stack)

- `sexusb.kbd.found = 1` — keyboard detected and enumerated
- `sexusb.kbd.raw = 15` — 8-byte HID reports arriving from interrupt IN
  endpoint (max_packet_size=8, interval=7)
- `sexinput.kbd.recv = 15` — reports forwarded via OP_USB_KEYBOARD_REPORT
  (0x261) through PDX IPC
- `keyboard_cursor.gate = 1` — compile-time gate SEXOS_KEYBOARD_CURSOR=1
  enabled and boot diagnostic printed

### What Doesn't Work (QEMU Input Layer)

- QMP `input-send-event` — no effect (all key bytes zero)
- HMP `sendkey` — no effect (routes to non-existent PS/2)
- `-machine q35,i8042=off` — removes PS/2 but USB still doesn't receive
- `display=sdl` binding — confirmed in qtree but no effect
- `nec-usb-xhci` vs `qemu-xhci` — same result

### dev.sh Additions (committed)

```
SEXOS_QEMU_I8042=off         → -machine q35,i8042=off
SEXUSB_QEMU_DEVICE=kbd-display-sdl  → -device usb-kbd,bus=xhci.0,display=sdl
```

### Same-Class Bug: HOST_INPUT_BACKEND_AUDIT_V1

This is the same issue as the pointer-motion failure: QEMU 11.0.0 on
this host does not deliver ANY host input events to emulated USB HID
devices. Both pointer (USB mouse/tablet) and keyboard (USB keyboard)
are affected. The input path from host → QEMU input subsystem →
emulated USB HID device is broken.

### Next Options

1. **Use `qemu-xhci` instead of `nec-usb-xhci`** — may route input differently
2. **Pass through a real USB controller via VFIO** — hardware access, not emulated
3. **Add synthetic report generation in sexusb** — guest-side key simulation
   for dev proofs (gated by env var)
4. **Try different QEMU version** — 11.0.0-specific regression
5. **Use USB/IP or usbredir** — pass real USB device from host
