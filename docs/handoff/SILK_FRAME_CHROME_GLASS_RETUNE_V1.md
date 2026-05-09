# SILK_FRAME_CHROME_GLASS_RETUNE_V1

**Date:** 2026-05-08
**Status:** MERGED

## Summary

Retuned the per-window frame chrome rendering in `composite_pixel` to use the R0-R6 crystalline glass renderer stack. All geometry, flags, tab logic, and surface compositing order preserved. Visual-only — no hover, no new opcodes.

## Changes in composite_pixel

| Zone | Before | After |
|------|--------|-------|
| Top bar bg | Opaque `frame_top_bar_color` | `glass_over_bg(..., alpha=200)` |
| Tab strip (active) | Opaque `active_tab_color` | Glass bg + `alpha_blend(..., alpha=240)` |
| Tab strip (inactive) | Opaque `inactive_tab_color` | Glass bg + `alpha_blend(..., alpha=200)` |
| Frame lights (top bar) | Opaque colored squares | `glass_over_bg(..., alpha=224)` |
| Rim band (focused) | Opaque `frame_rim_color` | `glass_over_bg` + `pulse_alpha(196, 12, 128)` |
| Minimal lights | Opaque colored squares | `glass_over_bg(..., alpha=224)` |
| Minimal tab strip | Opaque colored blocks | `glass_over_bg(..., alpha=220/180)` |

## Constants

| Constant | Value |
|----------|-------|
| TOOLBAR_GLASS_ALPHA | 200 |
| Tab active alpha | 240 |
| Tab inactive alpha | 200 |
| Light glass alpha | 224 |
| Rim base alpha | 196 |
| Rim amp | 12 |
| Rim period | 128 |

## Preserved
- Divider line (opaque, for readability)
- Title text (opaque `toolbar_title_fg_at`)
- Content area (unchanged `fill_rect_color`)
- All geometry, flags, tab logic

## Forbidden
- No new opcodes
- No hover state
- No behavior changes
