# POINTER_QUALITY_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root causes

1. **Flush threshold too high**: required `count>=4 OR (count>=2 AND abs_sum>=24)`, causing micro movements to stall for many frames
2. **Noise floor too aggressive**: dropped `abs<=2` to zero, swallowing single-pixel movements
3. **Cap too low**: max step of 8px made large movements feel chunky and unresponsive

## Fixes in `apply_rel_pointer` tracker-lite

| Parameter | Before | After |
|-----------|--------|-------|
| Flush: count threshold | ≥4 | **≥3** |
| Flush: movement threshold | count≥2 AND abs≥24 | **count≥1 AND abs≥6** |
| Noise floor (dropped) | abs ≤ 2 → 0 | **abs ≤ 1 → 0** |
| Micro movement | abs 3-8 → 1-2 | **abs 2-6 → 1** |
| Low range | abs 9-32 → 2-8 (÷4) | **abs 7-30 → 2-10 (÷3)** |
| Mid range | abs 33-96 → 5-16 (÷6) | **abs 31-60 → 7-15 (÷4)** |
| Cap | 8 | **12** |

## Expected behavior

- Single small USB report (dx=3, dy=0) flushes after 1 frame → cursor moves 1px
- Medium movement (dx=12, dy=12) flushes at abs=24 → cursor moves 4px
- Large burst (dx=100, dy=0) → capped at 12px
- Micro movement (dx=1, dy=1) repeated 3× → flushes on count=3 → intentional 1px

## Cursor bounds

`POINTER_X.clamp(0, P.width-1)`, `POINTER_Y.clamp(0, P.height-1)` — cursor can reach y=0 (SilkBar zone).
