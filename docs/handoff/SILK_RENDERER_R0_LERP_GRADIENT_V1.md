# SILK_RENDERER_R0_LERP_GRADIENT_V1

**Date:** 2026-05-08
**Status:** MERGED

## Summary

Replaced the hard 4-band desktop background gradient with a smooth fixed-point vertical gradient using integer `lerp_color_xrgb`.

## Primitives added

### `lerp_color_xrgb(a: u32, b: u32, t: u8) -> u32`

Fixed-point linear interpolation between two XRGB colors. t=0 returns a, t=255 returns b. Alpha byte forced to 0xFF. Uses integer arithmetic only — no floats.

### `vertical_gradient_xrgb(y: usize, h: usize, top: u32, bottom: u32) -> u32`

Smooth vertical gradient. y=0 returns top, y=h-1 returns bottom. Uses lerp_color_xrgb internally.

## Changed

### `bg(y, h)` — smooth gradient from `DEFAULT_THEME.bg_top` to `DEFAULT_THEME.bg_bottom`

Before: 4 hard band thresholds (y<200, y<350, y<500, y<650).
After: continuous gradient across full height using Theme endpoints.

## Forbidden (deferred)

- No alpha blending
- No framebuffer read-modify-write
- No blur
- No glow
- No shadows
- No row buffer
- No tick counter

## Files changed

| File | Change |
|------|--------|
| `servers/sexdisplay/src/main.rs` | Added lerp_color_xrgb, vertical_gradient_xrgb; replaced bg() |

## Next: R1 Alpha Blend
