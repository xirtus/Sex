# ABS_CORNER_SENTINEL_V1

**Date:** 2026-05-08
**Status:** MERGED

## Added near-corner sentinel rejection

QEMU GTK emits ~(36,12) style corner coordinates during grab/ungrab transitions. Extended `abs_sentinel_ok` to reject:

| Zone | Range | Condition |
|------|-------|-----------|
| Top-left near-corner | x≤40, y≤20 | Reject before trust; after trust only if huge jump |
| Bottom-right max-edge | x≥W-1, y≥H-1 | Same |
| Zero-init | x≤1, y≤1 | Reject before trust |
