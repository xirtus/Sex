# USB_XHCI_MINIMAL_ENUM_V1

## A) PASS/FAIL
- PASS

## B) Exact root cause
- The `phase=RING` timeout was not in `Enable Slot` or `Address Device`.
- Enumeration completed successfully:
  - `Enable Slot` completion was observed with slot `1`.
  - `Address Device` completion was observed with code `1`.
  - `usb.xhci.enum.done` was emitted for slot `1`.
- The timeout came later, in the interrupt-IN wait loop after endpoint 0 setup and endpoint configuration.
- In this QEMU lane, that loop never saw a transfer event, so it kept emitting bounded `phase=RING` timeout diagnostics instead of stopping with a precise phase reason.

## C) Files changed
- `servers/sexusb/src/main.rs`
- `docs/handoff/USB_XHCI_MINIMAL_ENUM_V1.md`

## D) Minimal diff summary
- Added explicit command submit/completion markers for xHCI enumeration:
  - `[usb.xhci.enum.begin]`
  - `[usb.xhci.cmd.submit] kind=enable_slot`
  - `[usb.xhci.cmd.complete] kind=enable_slot slot=S code=C ok=1`
  - `[usb.xhci.cmd.submit] kind=address_device slot=S`
  - `[usb.xhci.cmd.complete] kind=address_device slot=S code=C ok=1`
  - `[usb.xhci.enum.done] slot=S ok=1`
- Converted the later ring wait timeout into exact actionable diagnostics:
  - `[usb.xhci.enum.timeout] phase=RING detail=interrupt_in_no_transfer_events polls=N ok=0`
  - `[usb.xhci.enum.stop] reason=interrupt_in_no_transfer_events actionable=1`
- Left the rest of xHCI behavior intact.

## E) Proof markers observed
- `logs/qemu-latest.log` contains:
  - `[usb.xhci.enum.begin]`
  - `[usb.xhci.cmd.submit] kind=enable_slot`
  - `[usb.xhci.cmd.complete] kind=enable_slot slot=1 code=1 ok=1`
  - `[usb.xhci.cmd.submit] kind=address_device slot=1`
  - `[usb.xhci.cmd.complete] kind=address_device slot=1 code=1 ok=1`
  - `[usb.xhci.enum.done] slot=1 ok=1`
  - `[usb.xhci.multiport.begin]`
  - `[usb.xhci.multiport.done] ports_seen=8 connected=1 slots_tracked=1`
  - `[usb.xhci.multiport.pass] ok=1`
  - repeated `[usb.xhci.enum.timeout] phase=RING detail=interrupt_in_no_transfer_events polls=N ok=0`
  - repeated `[usb.xhci.enum.stop] reason=interrupt_in_no_transfer_events actionable=1`
- Fault scan found no `#PF`, `#GP`, `panic`, or `fault.kill`.

## F) Commands run and results
- `git status --short`
  - Result: repo already had unrelated local modifications and untracked files; left untouched.
- `./scripts/entrypoint_build.sh`
  - Result: PASS.
  - Build completed successfully with warnings only.
- `./scripts/qemu_harness.sh --timeout 30 --markers || true`
  - Result: exit `124` from timeout, not crash.
  - Log saved at `logs/qemu-latest.log`.
- `rg -n "usb\\.xhci\\.enum|usb\\.xhci\\.cmd|usb\\.xhci\\.multiport|#PF|#GP|panic|fault.kill" logs/qemu-latest.log`
  - Result: required markers present; no fault tokens matched.

## G) Deferred work / STOP FIRST notes
- Deferred work:
  - `USB_HID_BOOT_MOUSE_REPORT_V1`
  - `USB_HID_POINTER_PRODUCER_V1`
  - `TOUCHPAD_ABS_CONTACT_V1`
  - `TRACKPAD_GESTURES_V1`
- This phase does not implement keyboard, mouse, trackpad, gestures, or shell/display policy.
- No kernel edits, sex-pdx edits, ABI changes, scheduler changes, or display/input policy changes were needed.

## H) Handoff path
- `docs/handoff/USB_XHCI_MINIMAL_ENUM_V1.md`
