# FRAME_LIGHT_HITBOX_EXPAND_V1

**Date:** 2026-05-08
**Status:** MERGED

## Change

Expanded frame light hitboxes from 10px to 20px width each, contiguous from surface left edge.

| Light | Old x-range | New x-range |
|-------|------------|-------------|
| Close | sx+5..sx+15 | sx+0..sx+20 |
| Minimize | sx+20..sx+30 | sx+20..sx+40 |
| Zoom | sx+35..sx+45 | sx+40..sx+60 |

Y-range unchanged: sy..sy+FRAME_TOP_BAR_HEIGHT_PX (28px).

Visual rendering unchanged — only hit-test expanded.
