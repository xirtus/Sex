# ABS_JUMP_FILTER_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

QEMU tablet sends huge coordinate discontinuities on window enter/exit/grab events, causing cursor to teleport between opposite corners.

## Fix: abs_jump_ok(screen_x, screen_y)

Rejects single-report jumps where:
- dx > screen_width/3 OR dy > screen_height/3
- First valid ABS always accepted (no LAST_VALID yet)

Applied at all 4 LAST_VALID update sites (handle_hid_event ×2, main dispatch ×2).

## Threshold

| Axis | Max allowed jump |
|------|-----------------|
| X | 1280/3 ≈ 426 px |
| Y | 720/3 = 240 px |
