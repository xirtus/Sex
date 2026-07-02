# LIVE_CURSOR_TABLET_SDL_BINDING_PROOF_V1

## A) PASS/FAIL
FAIL

## B) Exact QEMU args before/after
### Before
- `dev.sh` already accepted `SEXUSB_QEMU_DEVICE=tablet-display-sdl`
- The resolved interactive path was intended to be:
  - `-display sdl`
  - `-device usb-tablet,bus=xhci.0,display=sdl`

### After
- `dev.sh` now prints the resolved input args before launch and in `QEMU_PRINT_CMD=1` mode:
  - `[qemu.input.args] display=... usb_device=... args="..."`
  - `[qemu.input.usb] mode=... args="..."`
  - `[qemu.input.binding] mode=... ok=1`
- Verified resolved argv examples:
  - `SEXUSB_QEMU_DEVICE=tablet-display-sdl SEXOS_QEMU_DISPLAY=sdl ./dev.sh run`
    - `[qemu.input.args] display=sdl usb_device=tablet-display-sdl args="-display sdl -device usb-tablet,bus=xhci.0,display=sdl"`
  - `SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run`
    - `[qemu.input.args] display=sdl-grab usb_device=tablet args="-display sdl,grab-mod=lctrl-lalt -device usb-tablet,bus=xhci.0"`
  - `SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl ./dev.sh run`
    - `[qemu.input.args] display=sdl usb_device=mouse args="-display sdl -device usb-mouse,bus=xhci.0"`
- Normal harness log now also includes:
  - `[qemu.input.args] display=none usb_device=tablet args="-display none -device usb-tablet,bus=xhci.0"`

## C) Whether nonzero live tablet packets were observed
- No.
- The live log still showed only zero tablet packets:
  - `[usb.tablet.raw] len=6 b0=00 b1=00 b2=00 b3=00 b4=00 b5=00`
  - `[usb.tablet.abs.decode] x_raw=0 y_raw=0 buttons=0 ok=1`
  - `[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=0 dx=0 dy=0 ok=1`
- The proof environment did not provide an interactive SDL mouse stimulus that could be validated from this terminal session.

## D) Files changed
- [dev.sh](/home/xirtus_arch/Documents/microkernel/dev.sh)
- [docs/handoff/LIVE_CURSOR_TABLET_SDL_BINDING_PROOF_V1.md](/home/xirtus_arch/Documents/microkernel/docs/handoff/LIVE_CURSOR_TABLET_SDL_BINDING_PROOF_V1.md)

## E) Proof commands and markers
- `./scripts/entrypoint_build.sh`
  - PASS
  - ISO rebuilt successfully
- `QEMU_PRINT_CMD=1 SEXUSB_QEMU_DEVICE=tablet-display-sdl SEXOS_QEMU_DISPLAY=sdl ./dev.sh run`
  - Printed the exact SDL/tablet argv
- `QEMU_PRINT_CMD=1 SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run`
  - Printed the tablet + SDL grab argv
- `QEMU_PRINT_CMD=1 SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl ./dev.sh run`
  - Printed the usb-mouse + SDL argv
- `./scripts/qemu_harness.sh --timeout 30 --markers || true`
  - Exit `124` from timeout
  - Log path: `logs/qemu-latest.log`
  - Observed markers:
    - `[qemu.input.args] display=none usb_device=tablet args="-display none -device usb-tablet,bus=xhci.0"`
    - `[qemu.input.usb] mode=tablet args="-device usb-tablet,bus=xhci.0"`
    - `[qemu.input.binding] mode=tablet ok=1`
    - `[usb.tablet.raw] len=6 b0=00 b1=00 b2=00 b3=00 b4=00 b5=00`
    - `[usb.tablet.abs.decode] x_raw=0 y_raw=0 buttons=0 ok=1`
    - `[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=0 dx=0 dy=0 ok=1`
- Fault scan was clean:
  - no `#PF`
  - no `#GP`
  - no `panic`
  - no `fault.kill`

## F) Remaining deferred work
- click proof
- PS/2 mouse IRQ12 STOP FIRST
- touchpad / gesture work later

## G) Explicit statement
No kernel, ABI, sexdisplay, or shell-policy changes were made.
