# USB_HID_SHORT_PACKET_ACCEPT_V1

## A) PASS/FAIL
- PASS

## B) Exact root cause
- The xHCI transfer event for the HID interrupt-IN endpoint completed with completion code `13`, which is a short packet.
- In the fresh QEMU run, the event decoded as:
  - `requested=8`
  - `residual=2`
  - `actual=6`
  - `slot=1`
  - `dci=3`
- That is a valid short-packet completion: the controller delivered 6 useful bytes, so the raw HID report was accepted instead of being treated as a failure.
- The original blocker was the lack of explicit short-packet acceptance and bounded report copying in the interrupt-IN path. That is now fixed locally in `servers/sexusb/src/main.rs`.

## C) Files changed
- `servers/sexusb/src/main.rs`
- `docs/handoff/USB_HID_SHORT_PACKET_ACCEPT_V1.md`

## D) Minimal diff summary
- Added explicit short-packet proof markers:
  - `[usb.hid.short_packet.begin]`
  - `[usb.hid.transfer.decode] code=C requested=N residual=R actual=A slot=S dci=D`
  - `[usb.hid.short_packet.accept] actual=A ok=1`
  - `[usb.hid.short_packet.pass] ok=1`
- Updated the interrupt-IN completion marker to report `actual` bytes, not a generic `len`.
- Copied only the received `actual` bytes from the interrupt buffer into a bounded local array before decode/logging.
- Kept the change local to `servers/sexusb/src/main.rs`; no kernel, ABI, sexdisplay, silk-shell, or sexinput edits.

## E) Proof markers observed
- Observed in `logs/qemu-latest.log`:
  - `[usb.hid.event.any] type=32 slot=1 ep=3 code=13`
  - `[usb.hid.short_packet.begin]`
  - `[usb.hid.transfer.decode] code=13 requested=8 residual=2 actual=6 slot=1 dci=3`
  - `[usb.hid.short_packet.accept] actual=6 ok=1`
  - `[usb.hid.intr.event] slot=1 dci=3 code=13 actual=6 ok=1`
  - `[usb.hid.boot_mouse.report] len=6 b0=00 b1=00 b2=00 b3=00 ok=1`
  - `[usb.hid.short_packet.pass] ok=1`
- Fault scan was clean:
  - no `#PF`
  - no `#GP`
  - no `panic`
  - no `fault.kill`

## F) Commands run and results
- `git status --short`
  - Result: repo already had unrelated local modifications and untracked files; left untouched.
- `./scripts/entrypoint_build.sh`
  - Result: PASS.
  - Release build completed successfully with warnings only.
- `./scripts/qemu_harness.sh --timeout 30 --markers || true`
  - Result: exit `124` from timeout, not crash.
  - Log saved at `logs/qemu-latest.log`.
- `rg -n "usb.hid.short_packet|usb.hid.transfer.decode|usb.hid.boot_mouse.report|usb.hid.intr.event|usb.hid.event.any|#PF|#GP|panic|fault.kill" logs/qemu-latest.log`
  - Result: short-packet accept markers and clean fault scan confirmed.

## G) Deferred work / STOP FIRST notes
- Deferred work:
  - `USB_HID_POINTER_PRODUCER_V1` only after raw report exists and the pointer policy layer is intentionally revisited
  - `TOUCHPAD_ABS_CONTACT_V1`
  - `TRACKPAD_GESTURES_V1`
- This phase does not implement shell pointer policy, click focus, drag, trackpad gestures, or display rendering changes.
- No kernel edits, sex-pdx edits, ABI changes, scheduler changes, or display-policy changes were needed.

## H) Handoff path
- `docs/handoff/USB_HID_SHORT_PACKET_ACCEPT_V1.md`
