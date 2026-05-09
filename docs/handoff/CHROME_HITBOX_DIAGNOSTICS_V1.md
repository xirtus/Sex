# CHROME_HITBOX_DIAGNOSTICS_V1

**Date:** 2026-05-08
**Status:** MERGED

## Diagnostics added

| Marker | Budget | Reveals |
|--------|--------|---------|
| `[shell.frame.chrome.bounds] sid=N x=N y=N w=N h=N topbar_h=N` | 8 | Surface position and toolbar height |
| `[shell.frame.light.hitbox] frame=N sx=N sy=N close=(x0,y0)-(x1,y1) min=... zoom=...` | 4 | Exact light rectangle coordinates |
| `[shell.hit_target.chrome] frame=N kind=N x=N y=N` | 6 | Existing — chrome hit confirmed |

## How to interpret

If clicking at (x=200, y=70) on the Quil window hits `kind=app` instead of `kind=chrome`:

1. Check `[shell.frame.chrome.bounds]` for Quil's surface:
   - `sx, sy` = surface origin in screen coords
   - `topbar_h = 28` = toolbar height

2. Lights are at:
   - Close: `sx + gap` to `sx + gap + size` (x-axis), `sy` to `sy + 28` (y-axis)
   - Minimize: next gap + size block
   - Zoom: third block

3. If your click x=200 is outside the light rectangles, you're clicking the toolbar body (tab strip or empty space), not a light.

4. Frame lights are small (10x28 in default mode). You must click precisely within the red/yellow/green squares.
