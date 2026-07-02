# SHELL_BOOT_LAYOUT_GEOMETRY_FIX_V1

Date: 2026-05-06

## Root Cause
Boot layout used legacy fixed offsets/sizes (`Quil 100,100,640x480` and `Linen 900,500,300x150`) rather than a deterministic content rectangle derived from live shell display geometry.

## Content Rect
Shell now computes boot content rectangle from runtime geometry:
- `x = 0`
- `y = P.bar_height`
- `w = P.width`
- `h = P.height - P.bar_height`

Markers:
- `[silk-shell.boot.layout.content] x=<..> y=<..> w=<..> h=<..>`
- `[silk-shell.boot.layout.reject] reason=invalid_content_rect ...` when degenerate

## Boot Geometry Decisions
- Quil (sid=201): now fills the full content rect at boot.
- Linen (sid=200): kept intentionally visible as demo surface, now clamped and placed inside content rect (top-right with margin), not off-screen/stale.

## Additional Consistency
Boot-time tracked geometry state is synchronized for both surfaces:
- `SURFACE_201_{X,Y,W,H}` set from boot content rect
- `SURFACE_200_{X,Y,W,H}` set from boot linen placement

## Z-order/visibility
No policy ownership change. Existing boot pair visibility checks and z-order proof remain in shell-only path.

## Build
- `./scripts/entrypoint_build.sh` passes.

## Expected Runtime Markers
- `[silk-shell.boot.layout.content] ...`
- `[silk-shell.boot.surface.bounds] sid=201 ...` with content-rect dimensions
- `[silk-shell.boot.surface.bounds] sid=200 ...` with in-bounds linen geometry
- `[silk-shell.boot.surface.visible] ...`
- `[silk-shell.boot.zorder] ...` for visible boot pair
