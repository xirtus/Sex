# USB_HID_INTR_TRANSFER_LIVENESS_V1

## A) PASS/FAIL
- FAIL

## B) Exact root cause or exact remaining liveness reason
- The interrupt-IN path is no longer silent. The controller produced a transfer event on the correct xHCI endpoint ID / DCI:
  - `usb_ep=0x81`
  - `ep_num=1`
  - `dir_in=1`
  - `dci=3`
  - `doorbell_target=3`
- The remaining failure is that the transfer event completed with code `13`, not a usable success completion, so no raw HID report was accepted.
- This means the problem is now event completion / device stimulus quality, not doorbell targeting or endpoint identity mapping.

## C) Files changed
- `servers/sexusb/src/main.rs`
- `docs/handoff/USB_HID_INTR_TRANSFER_LIVENESS_V1.md`

## D) Minimal diff summary
- Added explicit endpoint identity proof:
  - `[usb.hid.ep.identity] usb_ep=0x81 ep_num=1 dir_in=1 dci=3 doorbell_target=3 ok=1`
- Switched the interrupt-IN submit and match path to DCI-based ringing and matching.
- Added bounded liveness markers:
  - `[usb.hid.intr_liveness.begin]`
  - `[usb.hid.event.any] ...`
  - `[usb.hid.intr_liveness.timeout] ...`
  - `[usb.hid.intr_liveness.stop] ...`
  - `[usb.hid.intr_liveness.pass] ok=1` on success
- Kept the change local to `servers/sexusb/src/main.rs`; no kernel, ABI, sexdisplay, silk-shell, or sexinput edits.

## E) Proof markers observed
- Observed in `logs/qemu-latest.log`:
  - `[usb.hid.ep.identity] usb_ep=0x81 ep_num=1 dir_in=1 dci=3 doorbell_target=3 ok=1`
  - `[usb.hid.intr_liveness.begin]`
  - `[usb.hid.intr.trb.submit] slot=1 dci=3 len=8 cycle=1 ioc=1`
  - `[usb.hid.event.any] type=32 slot=1 ep=3 code=13`
  - `[usb.hid.intr_liveness.timeout] reason=transfer_event_code_13 slot=1 dci=3 doorbell_target=3 polls=2`
  - `[usb.hid.intr_liveness.stop] actionable=1 reason=transfer_event_code_13`
  - `[usb.hid.boot_mouse.timeout] reason=transfer_event_code_13 slot=1 ep=1 polls=2`
  - `[usb.hid.boot_mouse.stop] actionable=1 reason=transfer_event_code_13`
- Not observed:
  - `[usb.hid.intr.event] ... ok=1`
  - `[usb.hid.boot_mouse.report] ... ok=1`
  - `[usb.hid.intr_liveness.pass] ok=1`
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
- `rg -n "usb.hid|usb.xhci.enum|usb.xhci.cmd|#PF|#GP|panic|fault.kill" logs/qemu-latest.log`
  - Result: found the DCI-targeted event and transfer-code timeout markers; no fault tokens matched.

## G) Remaining deferred work / STOP FIRST notes
- Deferred work:
  - `USB_HID_BOOT_MOUSE_REPORT_V1` if still no usable report
  - `USB_HID_POINTER_PRODUCER_V1` only after a raw report exists
  - `TOUCHPAD_ABS_CONTACT_V1`
  - `TRACKPAD_GESTURES_V1`
- This phase does not implement shell pointer policy, click focus, drag, trackpad gestures, or display rendering changes.
- No kernel edits, sex-pdx edits, ABI changes, scheduler changes, or display-policy changes were needed.

## H) Handoff path
- `docs/handoff/USB_HID_INTR_TRANSFER_LIVENESS_V1.md`
