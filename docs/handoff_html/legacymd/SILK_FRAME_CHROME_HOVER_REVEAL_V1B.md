# SILK_FRAME_CHROME_HOVER_REVEAL_V1B

**Date:** 2026-05-08
**Status:** MERGED

## Summary

Hover reveal for single-tab frame chrome using proven V1 hover state flags. Uses dim/bright approach (not hide/show) for safety. No geometry changes, no new opcodes.

## Behavior

| Frame type | Hover state | Chrome alpha | Visual effect |
|-----------|-------------|--------------|---------------|
| Multi-tab | any | 100% (dim=10) | Always fully visible |
| Single-tab | not hovered | 20% (dim=5) | Very dim, nearly invisible |
| Single-tab | hovered | 100% (dim=10) | Full glass brightness |

## Implementation

`scale_alpha(base, dim)` — scales alpha by dim/10. In `composite_pixel`:

- `chrome_dim = if single_tab && !frame_hovered { 5 } else { 10 }`
- Toolbar bg: `scale_alpha(base_alpha, chrome_dim)`
- Frame lights: `scale_alpha(light_alpha, chrome_dim)`
- Tab strip, divider, title: naturally dims with toolbar

## Proof markers

- `[sexdisplay.frame.hover.reveal] mode=v1b single_tab=1 hovered=0/1`

## Forbidden
- No geometry changes
- No hide/show (dim only)
- No new opcodes
- No behavior/action changes
