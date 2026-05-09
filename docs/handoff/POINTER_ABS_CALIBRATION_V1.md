# POINTER_ABS_CALIBRATION_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

QEMU usb-tablet sends raw 16-bit coordinates (range 0..32767) that must be scaled to screen dimensions (1280×720). The old code used `raw.clamp(0, screen_dim - 1)` which treated raw values as screen pixels — all values above 1279 were clamped to the right edge, making the cursor uncontrollable.

## Fix: normalize_abs_coord

```
screen_x = raw_x * (screen_width - 1) / 32767
screen_y = raw_y * (screen_height - 1) / 32767
```

Applied at both ABS handler sites (handle_hid_event + main dispatch). Now:
- raw=0 → screen=0
- raw=16383 → screen=639 (mid)
- raw=32767 → screen=1279 (right edge)

## Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added `normalize_abs_coord`, applied to all 4 ABS coordinate set sites |
