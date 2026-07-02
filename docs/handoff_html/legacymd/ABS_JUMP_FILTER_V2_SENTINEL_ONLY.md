# ABS_JUMP_FILTER_V2_SENTINEL_ONLY

**Date:** 2026-05-08
**Status:** MERGED

## Fix

Replaced `abs_jump_ok` (generic distance threshold W/3, H/3) with `abs_sentinel_ok` (poison rejection only).

ABS tablet reports direct position — any coordinate is valid after trust gate. Only sentinel values are rejected:
- Zero-init (x≤1, y≤1 before ABS_SEEN_VALID)
- Max-edge (x≥W-1, y≥H-1 before ABS_SEEN_VALID)
