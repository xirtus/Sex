# POINTER_ABS_TABLET_SANITY_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

QEMU usb-tablet sends x=0,y=0 init reports before the tablet has valid position data. These were accepted as real cursor positions, causing cursor to jump to upper-left corner. If a button-down coincided with the zero-position report, accidental clicks/open/close could trigger on whatever window was at (0,0).

The old `REAL_POINTER_SEEN` gate blocked ALL ABS after first REL event, which is wrong for tablet mode (ABS is the primary input).

## Fix: ABS trust gate

Added `ABS_SEEN_VALID` flag and `LAST_VALID_ABS_X/Y` tracking.

**Reject rule:** Before first valid position, reject ABS reports where `x <= 1 && y <= 1`.

**After first valid:** Accept all ABS (including legitimate x=0,y=0 at screen edges).

**Button safety:** Button-down uses `LAST_VALID_ABS_X/Y` (always valid after trust gate passes).

## Changed files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `ABS_SEEN_VALID`, `LAST_VALID_ABS_X/Y`; replaced EV_ABS handlers in main dispatch, handle_hid_event, and before-linen drain |

## New markers

| Marker | Budget | Purpose |
|--------|--------|---------|
| `[shell.pointer.abs.reject] reason=zero_init x=N y=N` | 16 | Rejected zero-init ABS report |
| `[shell.pointer.abs.accept] x=N y=N` | 4 | First valid ABS position accepted |
