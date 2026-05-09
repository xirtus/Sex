# POINTER_ABS_EDGE_GUARD_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

QEMU usb-tablet sends max-edge reports (x=1279, y=719) when the pointer leaves the QEMU window. These were accepted as valid positions, causing cursor to jump to bottom-right corner and triggering accidental clicks at that position.

## Edge guard

Added to main EV_ABS handler after zero-init check:

| Condition | Action |
|-----------|--------|
| `ax >= W-1 && ay >= H-1` (max edge) + `!ABS_SEEN_VALID` | **REJECT** |
| Max edge + ABS valid + huge jump (>W/2 && >H/2 from last) | **REJECT** |
| Max edge + gradual reach (near last known position) | Accept |
| Any other coordinate | Accept normally |

## New markers

| Marker | Budget | Purpose |
|--------|--------|---------|
| `[shell.pointer.abs.reject] reason=edge_max x=N y=N last_x=N last_y=N` | 16 | Max-edge report rejected |
