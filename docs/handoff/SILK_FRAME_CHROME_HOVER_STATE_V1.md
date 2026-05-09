# SILK_FRAME_CHROME_HOVER_STATE_V1

**Date:** 2026-05-08
**Status:** MERGED

## Summary

Plumbed silk-shell frame hover state into sexdisplay via existing OP_SURFACE_TAB_INFO (0xFD) chrome flags byte. No new opcodes, no ABI changes. Visual brightening only — no hide/reveal.

## Protocol

Reuses existing 0xFD `arg2 >> 8` chrome flag byte:

| Bit | Name | silk-shell source | sexdisplay consumer |
|-----|------|-------------------|---------------------|
| 0 | TOP_BAR | `frame_has_top_bar()` | `SURFACE_CHROME_TOP_BAR` |
| 1 | FRAME_HOVER | `HOVERED_FRAME_ID == frame_id` | `SURFACE_CHROME_FRAME_HOVER` |
| 2 | LIGHT_HOVER | `HOVERED_FRAME_LIGHT != NONE` | `SURFACE_CHROME_LIGHT_HOVER` |
| 3-4 | LIGHT_KIND | close=0, minimize=1, zoom=2 | `SURFACE_CHROME_LIGHT_KIND_MASK` |

## Changes

### silk-shell
- `send_frame_tab_info()`: packs hover bits into chrome_flags byte
- `update_frame_hover_at()`: on hover change, calls `send_frame_tab_info()` for old and new frames

### sexdisplay
- New constants: `SURFACE_CHROME_FRAME_HOVER`, `SURFACE_CHROME_LIGHT_HOVER`, `SURFACE_CHROME_LIGHT_KIND_MASK`
- 0xFD handler: `[sexdisplay.frame.hover.recv]` marker when hover flags change
- `composite_pixel`: toolbar alpha 200→220 when hovered; light alpha 224→255 when hovered

## Visual effects

| State | Effect |
|-------|--------|
| No hover | Glass toolbar at alpha 200, lights at alpha 224 |
| Frame hovered | Toolbar brightens to alpha 220 |
| Light hovered | Hovered light becomes fully opaque (alpha 255) |

## Forbidden
- No toolbar hide/reveal
- No close/minimize/zoom behavior
- No new opcodes
- No ABI changes
