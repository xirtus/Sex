# USB_HID_POINTER_CLICK_PROOF_V1

## A) PASS/FAIL
FAIL

## B) Exact click/button path
- `servers/sexinput/src/main.rs`
  - `normalize_pointer_report_v1(...)` now emits:
    - `[usb.hid.pointer.button] buttons=B left=L ok=1`
  - Existing button-edge normalization remains intact:
    - raw tablet `buttons` bits are XORed against `last_buttons`
    - bit 0 maps to left button (`btn=1`)
    - `EV_BTN` edges are forwarded through `OP_HID_EVENT`
  - Existing pointer emission remains unchanged:
    - `[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=B dx=DX dy=DY ok=1`
- `servers/silk-shell/src/main.rs`
  - Existing `EV_BTN` receive path already existed.
  - Added proof-only receive marker:
    - `[usb.hid.pointer.click.recv] left=1 value=1 ok=1`
  - No click-focus, drag, or policy logic changed.

## C) Files changed
- [servers/sexinput/src/main.rs](/home/xirtus_arch/Documents/microkernel/servers/sexinput/src/main.rs)
- [servers/silk-shell/src/main.rs](/home/xirtus_arch/Documents/microkernel/servers/silk-shell/src/main.rs)
- [docs/handoff/USB_HID_POINTER_CLICK_PROOF_V1.md](/home/xirtus_arch/Documents/microkernel/docs/handoff/USB_HID_POINTER_CLICK_PROOF_V1.md)

## D) Minimal diff summary
- Added a USB-side proof marker for tablet button state changes.
- Added a shell-side receive marker for left-button press on the existing EV_BTN path.
- Preserved the existing pointer pipeline and all policy behavior.

## E) Proof markers
- Observed in `logs/qemu-latest.log`:
  - `[qemu.input.args] display=none usb_device=tablet args="-display none -device usb-tablet,bus=xhci.0"`
  - `[qemu.input.binding] mode=tablet ok=1`
  - `[usb.tablet.raw] len=6 b0=00 b1=00 b2=00 b3=00 b4=00 b5=00`
  - `[usb.tablet.abs.decode] x_raw=0 y_raw=0 buttons=0 ok=1`
  - `[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=0 dx=0 dy=0 ok=1`
- Not observed in this run:
  - `[usb.hid.pointer.button] buttons=1 left=1 ok=1`
  - `[usb.hid.pointer.click.recv] left=1 value=1 ok=1`

## F) Commands and results
- `./scripts/entrypoint_build.sh`
  - PASS
- `./scripts/qemu_harness.sh --timeout 30 --markers || true`
  - Exit `124` from timeout
  - No crash, reboot loop, `#PF`, `#GP`, `panic`, or `fault.kill`
  - Live tablet stream stayed zero-only in this environment

## G) Deferred work
- Live click proof still needs an actual left-button transition from SDL mouse input or hardware.
- `PS/2` mouse IRQ12 work remains STOP FIRST.
- Touchpad / gesture work remains later.

## H) Explicit statement
No kernel, ABI, sexdisplay, or shell-policy changes were made.
