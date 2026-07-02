# USB_POINTER_RECEIVE_APPLY_CADENCE_V1

## A) PASS/FAIL
FAIL

## B) Exact root cause / implemented path
- Root cause of the poor control signal was the tablet delta path being too permissive:
  - `DELTA_CLAMP=512` let large abs-to-rel bursts through.
  - idle zero-motion samples were still being forwarded as noop pointer traffic.
- Implemented path:
  - `sexinput` now clamps tablet-derived rel deltas at `64` instead of `512`.
  - `sexinput` now logs `[usb.tablet.delta.clamp] ...` and `[usb.pointer.producer.evrel] ...` for real rel emissions.
  - `sexinput` now drops repeated zero-motion tablet samples with `[usb.pointer.zero_drop] buttons=B ok=1`.
  - `silk-shell` now logs the live EV_REL receive path with `[usb.pointer.shell.recv.evrel] ...`.
  - `silk-shell` now logs the post-apply state with `[usb.pointer.shell.apply] x=X y=Y dx=DX dy=DY ok=1`.
  - `silk-shell` also logs a USB-safe cursor-bounds proof marker with `[usb.pointer.cursor.bounds] x=X y=Y ok=1`.
- I did not change click-focus policy, drag policy, display ownership, kernel, ABI, or sex-pdx.

## C) Files changed
- [servers/sexinput/src/main.rs](/home/xirtus_arch/Documents/microkernel/servers/sexinput/src/main.rs)
- [servers/silk-shell/src/main.rs](/home/xirtus_arch/Documents/microkernel/servers/silk-shell/src/main.rs)
- [docs/handoff/USB_POINTER_RECEIVE_APPLY_CADENCE_V1.md](/home/xirtus_arch/Documents/microkernel/docs/handoff/USB_POINTER_RECEIVE_APPLY_CADENCE_V1.md)

## D) Minimal diff summary
- Lowered tablet delta clamp from `512` to `64`.
- Added tablet delta clamp and producer EV_REL proof markers.
- Removed noop zero-motion forwarding and replaced it with a zero-drop proof marker.
- Added EV_REL receive/apply/bounds proof markers in `silk-shell`.

## E) Proof markers
- Observed in `logs/qemu-latest.log`:
  - `[usb.tablet.delta.clamp] limit=64 dx_in=0 dy_in=0 dx_out=0 dy_out=0 ok=1`
  - `[usb.pointer.zero_drop] buttons=0 ok=1`
- Not observed in this run:
  - `[usb.pointer.producer.evrel] ...`
  - `[usb.pointer.shell.recv.evrel] ...`
  - `[usb.pointer.shell.apply] ...`
  - `[usb.pointer.cursor.bounds] ...`
  - `cursor.motion.bounds`
  - `[usb.hid.pointer.click.recv] ...`

## F) Commands run and results
- `./scripts/entrypoint_build.sh`
  - PASS
  - Build completed successfully with warnings only.
- `./scripts/qemu_harness.sh --timeout 30 --markers || true`
  - Exit `124` from timeout, not a crash.
  - Log written to `logs/qemu-latest.log`.
  - The live run in this terminal session did not produce nonzero tablet motion or button proof; it stayed in zero-drop territory.
- `rg -n "usb.tablet.delta.clamp|usb.pointer.producer.evrel|usb.pointer.shell.recv.evrel|usb.pointer.shell.apply|usb.pointer.cursor.bounds|cursor.motion.bounds|usb.pointer.zero_drop|usb.hid.pointer.button|usb.hid.pointer.click.recv|#PF|#GP|panic|fault.kill" logs/qemu-latest.log`
  - Confirmed the clamp and zero-drop markers.
  - No fault markers were present.

## G) Remaining deferred work
- Live interactive proof with actual SDL mouse movement and click/hold/release.
- If control is still noisy after real stimulus, tune the clamp again, but keep it small and local.
- `PS/2` mouse IRQ12 remains STOP FIRST.
- No gesture, drag policy, or display-rendering changes were made.

## H) Explicit statement
This phase does not implement gesture policy, click-focus policy changes, drag policy changes, display rendering changes, kernel edits, ABI edits, or sex-pdx edits.
