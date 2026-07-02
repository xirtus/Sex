# XHCI_TABLET_INTR_RING_WRAP_V1

**Date:** 2026-05-08
**Status:** DIAGNOSTIC

## Added

Ring wrap marker: `[sexusb.xhci.intr_ring.wrap] n=N pcs=N` fires when producer wraps past slot 14 back to 0.

Requeue doorbell now logs `prod` and `pcs` for debugging.

## Runtime verification

```bash
grep -E "sexusb.xhci.intr_ring.wrap|sexusb.tablet.requeue.doorbell|sexusb.tablet.active" "$LOG" | tail -80
```

If `wrap` fires at n=1 and then `active` stops → wrap timing/cycle issue confirmed.
If `wrap` fires and `active` continues → ring wrap is not the problem.
