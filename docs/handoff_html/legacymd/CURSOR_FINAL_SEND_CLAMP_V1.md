# CURSOR_FINAL_SEND_CLAMP_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

Multiple cursor update paths sent POINTER_X/Y directly to sexdisplay without final bounds clamping. Raw tablet values (e.g., y=29326) could reach the framebuffer before normalization was applied, or from legacy OP_USB_MOUSE_REPORT path.

## Fix: `send_cursor_checked(x, y, source)`

Single helper that:
1. Clamps x/y to screen bounds
2. Logs clamp events with source tag
3. Sends clamped coordinates to sexdisplay

All 6 cursor send sites now use this helper:

| Source | Path |
|--------|------|
| `rel` | apply_rel_pointer |
| `abs` | handle_hid_event + main dispatch EV_ABS |
| `usb` | OP_USB_MOUSE_REPORT handler |

No direct `pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, ...)` remains outside the helper.
