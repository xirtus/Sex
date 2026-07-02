# USB_HID_BOOT_MOUSE_REPORT_V1

## A) PASS/FAIL
- FAIL

## B) Exact root cause
- The interrupt-IN endpoint is configured and doorbelled correctly for the primary device:
  - endpoint config marker shows slot `1`, ep `1`, max packet `8`, interval `4`
  - transfer TRB submit marker shows slot `1`, ep `1`, len `8`, cycle `1`, IOC `1`
- The failure is that QEMU never produces a transfer event in this boot window, so the poll loop times out with `interrupt_in_no_transfer_events`.
- This is a runtime/device-liveness failure, not a command-ring, slot, or endpoint-context setup failure.
- In this headless harness, the usb-tablet lane did not emit a boot-mouse/tablet report before the timeout window expired.

## C) Files changed
- `servers/sexusb/src/main.rs`
- `docs/handoff/USB_HID_BOOT_MOUSE_REPORT_V1.md`

## D) Minimal diff summary
- Added endpoint-ID tracking for the interrupt endpoint doorbell/event path.
- Added the required USB HID proof markers:
  - `[usb.hid.boot_mouse.begin]`
  - `[usb.hid.endpoint.config] ... ok=1`
  - `[usb.hid.intr.trb.submit] ...`
  - `[usb.hid.intr.event] ...` on success
  - `[usb.hid.boot_mouse.report] ... ok=1`
  - `[usb.hid.boot_mouse.pass] ok=1`
- Added one-shot timeout diagnostics:
  - `[usb.hid.boot_mouse.timeout] ...`
  - `[usb.hid.endpoint.diag] ...`
  - `[usb.hid.transfer_ring.diag] ...`
  - `[usb.hid.event_ring.diag] ...`
  - `[usb.hid.boot_mouse.stop] actionable=1 reason=interrupt_in_no_transfer_events`
- Kept shell policy, display policy, and broader HID framework untouched.

## E) Raw HID report proof markers or exact actionable failure diagnostics
- Observed in `logs/qemu-latest.log`:
  - `[usb.hid.boot_mouse.begin]`
  - `[usb.hid.endpoint.config] slot=1 ep=1 type=interrupt_in max_packet=8 interval=4 ok=1`
  - `[usb.hid.intr.trb.submit] slot=1 ep=1 len=8 cycle=1 ioc=1`
  - `[usb.hid.boot_mouse.timeout] reason=interrupt_in_no_transfer_events slot=1 ep=1 polls=2`
  - `[usb.hid.endpoint.diag] slot=1 ep=1 dci=3 max_packet=8 interval=4`
  - `[usb.hid.transfer_ring.diag] phys=0x1f92c000 prod=0 cycle=1 len=8`
  - `[usb.hid.event_ring.diag] ev_idx=12 ev_dcs=1 erdp=0x1f9250c0`
  - `[usb.hid.boot_mouse.stop] actionable=1 reason=interrupt_in_no_transfer_events`
- No `usb.hid.intr.event` marker was observed.
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
  - Build completed successfully with warnings only.
- `./scripts/qemu_harness.sh --timeout 30 --markers || true`
  - Result: exit `124` from timeout, not crash.
  - Log saved at `logs/qemu-latest.log`.
- `rg -n "usb\\.hid|usb\\.xhci\\.enum|usb\\.xhci\\.cmd|#PF|#GP|panic|fault.kill" logs/qemu-latest.log`
  - Result: required timeout diagnostics found; no fault tokens matched.

## G) Remaining deferred work / STOP FIRST notes
- Deferred work:
  - `USB_HID_POINTER_PRODUCER_V1`
  - `TOUCHPAD_ABS_CONTACT_V1`
  - `TRACKPAD_GESTURES_V1`
- This phase does not implement shell pointer policy, click focus, drag, trackpad gestures, or display rendering changes.
- No kernel edits, sex-pdx edits, ABI changes, scheduler changes, or display-policy changes were needed.

## H) Handoff path
- `docs/handoff/USB_HID_BOOT_MOUSE_REPORT_V1.md`
