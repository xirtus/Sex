# CURSOR_SINGLE_SOURCE_OF_TRUTH_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

In tablet ABS mode, both ABS and REL paths were active simultaneously, both updating POINTER_X/Y and sending cursor surface updates. The REL path (driven by `apply_rel_pointer`) would modify the cursor position using gain-accelerated deltas, fighting the ABS absolute position, causing cursor to jump/freeze.

## Fix

`apply_rel_pointer` now checks `ABS_SEEN_VALID` and returns (0,0) immediately when in ABS mode. ABS position is authoritative for tablet.

| Mode | REL path | ABS path |
|------|----------|----------|
| Mouse (REL only) | Active — moves cursor | Inactive |
| Tablet (ABS active) | **Suppressed** (returns 0,0) | Active — sets position |
