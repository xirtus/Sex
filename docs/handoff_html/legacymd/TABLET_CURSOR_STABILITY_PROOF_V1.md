# TABLET_CURSOR_STABILITY_PROOF_V1

**Date:** 2026-05-08
**Status:** PASS

## Summary

QEMU usb-tablet interrupt stream now survives transfer-ring wrap. Cursor input is no longer blocked by the old n≈15 freeze.

## Fixes covered

- ABS 0..32767 normalization
- raw-y shadow bug fix
- ABS before BTN ordering
- REL suppression after ABS lock
- final cursor send clamp
- zero/max/corner sentinel guards
- tablet interrupt requeue
- xHCI Link TRB cycle fix at wrap

## Runtime proof

- Previous freeze occurred at tablet report n≈15
- New proof reached n=128, n=192
- `[sexusb.xhci.intr_ring.wrap]` survived across wraps
- `sexusb.tablet.requeue.doorbell` continued past old freeze
- `shell.cursor.final.send` continued past old freeze
- `fault.kill / #PF / #GP / panic / KERNEL PANIC` = 0

## Remaining work

Usability polish and GUI click-target proof, not USB stream liveness.
