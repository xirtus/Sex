# SEXUSB_TABLET_REQUEUE_LIVENESS_V2

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

The XHCI interrupt transfer ring advance only incremented `intr_prod` and handled Link TRB wraps, but never wrote a new NORMAL TRB or rang the doorbell. After the initial TRB was consumed by the xHCI, no subsequent transfer was queued. The keyboard path had its own manual re-arm; tablet/mouse paths fell through to the loop advance which didn't requeue.

## Fix

Added TRB write + doorbell ring to the loop advance (after Link TRB wrap handling), matching the keyboard re-arm pattern. The keyboard path is unaffected (uses `skip_advance = true`).

| Path | Before | After |
|------|--------|-------|
| Keyboard | Manual re-arm (skip_advance=true) | Unchanged |
| Tablet/mouse | No requeue (stream stopped after 1 TRB) | Auto requeue via loop advance |
