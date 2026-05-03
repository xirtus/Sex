# Agent Handoff: USB Tablet HID Proof (2026-05-03)

## Summary

Tablet HID detection and absolute position reporting is **proven**.
Button events and click-focus **remain unproven** due to QEMU 11.0 host input routing.

## What Works (Proven)

- Tablet HID interface detected in config walk (`hid_tablet.found`)
- HID report descriptor shape scanned as tablet/pointer (`tablet_shape.ok`)
- SHORT_PACKET (cc=13) accepted in interrupt-IN event handler
- Absolute position reports received and decoded to relative deltas
- Two nonzero reports captured: `x=32741 y=9625` and `x=32741 y=9379`
- Shell pointer state updated: `[shell.pointer.usb_state.nonzero.ok]`
- SDL X11 window visible and addressable via `xdotool`

## What Needs Proof

1. **Button events** — xdotool mouse click injection into QEMU SDL window
   ```
   WID=$(xdotool search --name "QEMU" | head -1)
   xdotool mousemove --window $WID 400 300
   xdotool click 1
   ```
   Check for `buttons=0x01` in serial log.

2. **Click-focus hit-test** — requires button-down edge with nonzero position
   Check for `[shell.click_focus.down/hit/send.ok]`.

## Environment Requirements

- `SDL_VIDEO_DRIVER=x11` must be set (Wayland default creates no X11 window)
- `SEXUSB_QEMU_DEVICE=tablet` selects tablet device
- Serial output must be captured to file (use `-serial file:` for background runs)
- xdotool window search: `xdotool search --name "QEMU"` after boot

## Key Constants

- `OP_USB_MOUSE_REPORT = 0x260`
- Tablet report: 6 bytes (buttons + abs_x u16 LE + abs_y u16 LE + wheel)
- Delta clamp: `-128..127`
- `TRB_CC_SHORT_PACKET = 13`
- `INTR_TR_RING_SIZE = 16` (slots 0-14 Normal, slot 15 Link)

## Files

| File | Purpose |
|------|---------|
| `servers/sexusb/src/main.rs` | All USB/HID logic (2537+ lines) |
| `docs/INPUT_USB_NEXT.md` | Full USB input status and history |
| `CLAUDE.md` | Session memory (updated) |
| `dev.sh` | QEMU launch script with env var selectors |

## Blockers

- QEMU 11.0: QMP/HMP injection does not route to USB device models
- No real display/mouse available in current environment for interactive testing
- xdotool injection works at X11 level but button events may not reach USB emulation

## Fallback If Blocked

1. Real USB passthrough: `-device usb-host,vendorid=0xXXXX,productid=0xXXXX`
2. Internal synthetic proof mode: debug-env gated report injection in sexusb
