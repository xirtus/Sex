# SHELL_BOOT_TILED_DEMO_LAYOUT_V1

Date: 2026-05-06

## Root Cause
Confirmed: fullscreen Quil boot geometry (full content rect) made boot visually incorrect for the desired demo model.

## Boot Layout Chosen
Dynamic tiled two-surface boot layout inside content rect:
- content rect: `x=0, y=P.bar_height, w=P.width, h=P.height-P.bar_height`
- Quil (sid=201): left/main tile
- Linen (sid=200): right/secondary tile
- Non-overlapping by construction (with gutter)
- Focus remains Quil (`sid=201`)

## Bounds Policy
- No hardcoded legacy `100,100,640,480` placement.
- Tile widths derived from content width (main ~72%, side remainder) with min-size clamps.
- Full content height used for both tiles for clean side-by-side demo.

## Marker Updates
Added/updated:
- `[silk-shell.boot.layout.content]`
- `[silk-shell.boot.layout.tiled] mode=2pane ...`
- `[silk-shell.boot.surface.bounds]` for both 201 and 200
- `[silk-shell.boot.surface.visible]` for both 201 and 200
- `[silk-shell.boot.zorder] visible_count=2 first=201 second=200`
- `[silk-shell.boot.zorder.reject]` boot pair visibility failure
- `[silk-shell.boot.ui.ready] surfaces=2 focus=201`

## Focus/Z-order Semantics
Composition semantics unchanged:
- non-focused first, focused on top.
With non-overlapping tiles, focused Quil no longer hides Linen.

## Build
- `./scripts/entrypoint_build.sh` passes.

## Remaining Visual Gap
If visuals are still off at runtime, remaining work is spacing/theme polish (tile proportions/margins/chrome), not boot ownership, ABI, kernel, or renderer semantics.
