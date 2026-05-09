# SILK_RENDERER_R1_ALPHA_STATIC_BG_V1

**Date:** 2026-05-08
**Status:** MERGED

## Primitives added

### `alpha_blend_xrgb_over_xrgb(fg, bg, alpha) -> u32`
Alpha=0→bg, alpha=255→fg. Integer-only, no framebuffer read.

### `glass_over_bg(fg, y, alpha) -> u32`
Blends fg over `bg(y, FB_H)` using desktop gradient as background. Wrapper around `alpha_blend_xrgb_over_xrgb`.

## Applied

| Area | Alpha | Before | After |
|------|-------|--------|-------|
| SilkBar body (panel_fill) | 192 | Opaque `DEFAULT_THEME.panel_fill` | Glass blend over bg gradient |
| SilkBar glow edge (panel_glow) | 192 | Opaque `DEFAULT_THEME.panel_glow` | Glass blend over bg gradient |
| Net chip (chip_fill) | 208 | Opaque `DEFAULT_THEME.chip_fill` | Glass blend over bg gradient |

## Forbidden (deferred)
- No framebuffer read-modify-write
- No live surface sampling
- No blur/glow/shadow

## Next: R2 Panel Glow Rect
