# POINTER_QUALITY_V2_GAIN_ACCEL

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

Tracker-lite accumulator divided raw deltas by 3-6× before output. For USB HID relative reports (small values like dx=1-20), division made cursor nearly immobile. A full trackpad swipe producing dx=20/frame at 60Hz moved only ~5px per frame = 300px/sec.

## Fix: Per-event gain multiplier

Replaced divider-based accumulator with per-event multiplication. Cursor moves on every nonzero event no batching.

| Raw |abs| | Gain | Output |
|----------|------|--------|
| 0 | — | 0 |
| 1 | 1× | 1 |
| 2-3 | 2× | 4-6 |
| 4-8 | 3× | 12-24 |
| 9-20 | 4× | 36-80 |
| 21+ | 5× (cap 48) | 48 |

## Example: full trackpad swipe

dx=20/frame at 60Hz: 20 → 20×4=80px/frame → 4800px/sec → crosses 1280px screen in ~0.27s

## Preserved
- Cursor bounds clamp (0..W-1, 0..H-1)
- SilkBar zone reachable (y<50)
- REAL_POINTER_SEEN flag
- PENDING accumulators (reset, not used)
