# USB_HID_POINTER_PRODUCER_V1

A) PASS
- Raw USB HID report arrival is proven.
- The all-zero boot mouse/tablet report was decoded, normalized, emitted through `OP_HID_EVENT`, and received by `silk-shell`.
- This phase proves the route is alive, not motion/click semantics.

B) Exact root cause / implemented path
- The observed report was all zero:
  - `b0=00 b1=00 b2=00 b3=00`
- That means the proof target here is not pointer motion, but the raw-to-normalized-to-emitted transport path.
- Implemented path:
  - `sexusb` logs the pointer proof entry and the raw zero report marker.
  - `sexinput` decodes the USB mouse report, emits:
    - `[usb.hid.pointer.normalized] ...`
    - `[input.hid.pointer.recv] ...`
  - `sexinput` sends a proof-only no-op `OP_HID_EVENT` for the zero report.
  - `silk-shell` receives the event and logs the existing pointer receive marker.
- No shell policy changes, click-focus changes, drag changes, display changes, or HID parser rewrite were added.

C) Files changed
- [servers/sexusb/src/main.rs](/home/xirtus_arch/Documents/microkernel/servers/sexusb/src/main.rs)
- [servers/sexinput/src/main.rs](/home/xirtus_arch/Documents/microkernel/servers/sexinput/src/main.rs)
- [docs/handoff/USB_HID_POINTER_PRODUCER_V1.md](/home/xirtus_arch/Documents/microkernel/docs/handoff/USB_HID_POINTER_PRODUCER_V1.md)

D) Minimal diff summary
- Added `[usb.hid.pointer.begin]` and `[usb.hid.pointer.raw]` proof markers in `sexusb`.
- Added `[usb.hid.pointer.normalized]` and `[input.hid.pointer.recv]` markers in `sexinput`.
- Added proof-only zero-report emit logic so the zero report reaches `OP_HID_EVENT` and shell receive.
- Added `[usb.hid.pointer.emit]` and one-shot `[usb.hid.pointer.pass]` markers.
- Kept the synthetic pointer path intact.

E) Proof markers
- Observed in `logs/qemu-latest.log`:
  - `[usb.hid.pointer.begin] ok=1`
  - `[usb.hid.pointer.raw] len=0 b0=00 b1=00 b2=00 b3=00`
  - `[usb.hid.boot_mouse.report] len=6 b0=00 b1=00 b2=00 b3=00 ok=1`
  - `[usb.hid.pointer.normalized] buttons=0 dx=0 dy=0 kind=relative_or_zero ok=1`
  - `[input.hid.pointer.recv] source=usb buttons=0 dx=0 dy=0 ok=1`
  - `[usb.hid.pointer.emit] op=OP_HID_EVENT buttons=0 dx=0 dy=0 ok=1`
  - `[usb.hid.pointer.pass] ok=1`
  - `[silk-shell.pointer.recv] class=3 a0=0 a1=0`
- Fault scan was clean:
  - no `#PF`
  - no `#GP`
  - no `panic`
  - no `fault.kill`

F) Commands run and results
- `git status --short`
  - Repo had unrelated local modifications and untracked backups; they were left untouched.
- `./scripts/entrypoint_build.sh`
  - PASS.
  - Build completed successfully with warnings only.
- `./scripts/qemu_harness.sh --timeout 30 --markers || true`
  - Exit `124` from timeout, not a crash.
  - Log written to `logs/qemu-latest.log`.
- `rg -n "usb.hid.pointer|input.hid.pointer|usb.hid.boot_mouse.report|silk-shell.pointer.recv|#PF|#GP|panic|fault.kill" logs/qemu-latest.log`
  - Confirmed the pointer proof markers and a clean fault scan.

G) Deferred work / STOP FIRST notes
- Deferred work:
  - `TOUCHPAD_ABS_CONTACT_V1`
  - `TRACKPAD_GESTURES_V1`
  - keyboard HID later if not already covered
- This phase does not implement gesture policy, click-focus policy changes, drag policy changes, or display rendering changes.
- If the next proof needs actual motion or click evidence, it will require QEMU input stimulus or real hardware movement.

H) Handoff path
- [docs/handoff/USB_HID_POINTER_PRODUCER_V1.md](/home/xirtus_arch/Documents/microkernel/docs/handoff/USB_HID_POINTER_PRODUCER_V1.md)
