# USB_XHCI_MULTI_PORT_DISCOVERY_V1

## A) PASS/FAIL
- PASS

## B) Exact root cause
- `sexusb` discovery was still effectively single-primary-device behavior.
- The port scan now records all bounded root ports, but downstream state was still anchored on one `SingleHidBind` plus the first connected port as the active target.
- Before this patch, the scan/collection path only fed a bounded first-device pipeline and did not keep explicit per-port discovery state. Extra ports were not tracked with stable `connected/enabled/reset_attempted/slot_id` records, so discovery never had a durable multi-port view.
- This patch fixes that by adding fixed-size per-port tracking, explicit scan/slot markers, and bounded skip behavior instead of parking on extra connected ports.

## C) Files changed
- `servers/sexusb/src/main.rs`

## D) Minimal diff summary
- Added `PortDiscoveryState` fixed-size storage for bounded root-port tracking.
- Added explicit multiport markers:
  - `[usb.xhci.multiport.begin]`
  - `[usb.xhci.port.scan] port=N connected=X enabled=Y reset=Z slot=S`
  - `[usb.xhci.port.slot] port=N slot=S ok=1`
  - `[usb.xhci.port.skip] port=N reason=...`
  - `[usb.xhci.multiport.done] ports_seen=N connected=N slots_tracked=N`
  - `[usb.xhci.multiport.pass] ok=1`
- Replaced the old overflow park with bounded skip behavior.
- Preserved first working device behavior and left the later HID pipeline alone.

## E) Proof markers
- Observed in `logs/qemu-latest.log`:
  - `[usb.xhci.multiport.begin]`
  - `[usb.xhci.port.scan] port=1 connected=0 enabled=0 reset=0 slot=0`
  - `[usb.xhci.port.skip] port=1 reason=not_connected`
  - `[usb.xhci.port.scan] port=5 connected=1 enabled=0 reset=0 slot=0`
  - `[sexusb.xhci.addr_ctx.port.connected] port=5 speed=3`
  - `[usb.xhci.port.slot] port=5 slot=1 ok=1`
  - `[usb.xhci.multiport.done] ports_seen=8 connected=1 slots_tracked=1`
  - `[usb.xhci.multiport.pass] ok=1`
- Fault scan on `logs/qemu-latest.log` returned no matches for `#PF`, `#GP`, `panic`, or `fault.kill`.
- Runtime did show repeated bounded xHCI ring timeouts:
  - `[sexusb.xhci.enum.timeout] phase=RING polls=2 ok=0`
  - continued through later bounded polls

## F) Commands run and results
- `git status --short`
  - Result: repo already had unrelated local modifications and many untracked files; I did not touch them.
- `./scripts/entrypoint_build.sh`
  - Result: PASS
  - Build completed successfully; only warnings.
- `./scripts/qemu_harness.sh --timeout 30 --markers`
  - Result: exit `124` from timeout, not crash.
  - Log saved at `logs/qemu-latest.log`.
  - Marker summary showed USB/XHCI markers present.
  - Fault scan of the log found no `#PF`, `#GP`, `panic`, or `fault.kill`.

## G) Deferred work / STOP FIRST notes
- Deferred work:
  - `USB_XHCI_MINIMAL_ENUM_V1`
  - `USB_HID_BOOT_MOUSE_REPORT_V1`
  - `USB_HID_POINTER_PRODUCER_V1`
  - `TOUCHPAD_ABS_CONTACT_V1`
  - `TRACKPAD_GESTURES_V1`
- Explicitly not implemented in this phase:
  - keyboard HID
  - mouse HID
  - trackpad HID
  - gestures
  - shell focus policy
  - sexdisplay policy
  - ABI changes
  - kernel edits
  - sex-pdx edits

## H) Handoff path
- `docs/handoff/USB_XHCI_MULTI_PORT_DISCOVERY_V1.md`
