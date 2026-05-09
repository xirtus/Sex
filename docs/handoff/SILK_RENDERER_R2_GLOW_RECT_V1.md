# SILK_RENDERER_R2_GLOW_RECT_V1

**Date:** 2026-05-08
**Status:** MERGED

## Primitives added

### `glow_edge_alpha(dist, spread, max_alpha) -> u8`
Linear edge falloff: dist=0→max_alpha, dist≥spread→0. Integer-only.

## Applied

| Area | Effect | Technique |
|------|--------|-----------|
| SilkBar bottom edge (y∈[46,50)) | Glow falloff to panel_glow | `glow_edge_alpha` 4px spread, 64 max |
| Active workspace indicator | Bright glass tint | White blend 48 + glass_over_bg 224 |

## Constants

| Constant | Value |
|----------|-------|
| BAR_BOTTOM | 50 |
| GLOW_SPREAD | 4 px |
| Edge glow max alpha | 64 |
| Workspace glow tint | 0x00FFFFFF at alpha 48 |

## Forbidden (deferred)
- No framebuffer reads
- No blur kernel
- No row buffer
- No animation

## Next: R3 Row Buffer
