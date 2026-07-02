# ABS_NORMALIZE_ALL_PATHS_V1

**Date:** 2026-05-08
**Status:** MERGED

## Root cause

In `handle_hid_event`, y was normalized then immediately overwritten by raw value due to duplicate `let ay =` statement:

```
let ay = normalize_abs_coord(arg1 as i32, P.height);  // normalized
let ay = arg1 as i32;                                   // raw! overwrites
```

x was fine (single assignment). This caused the `handle_hid_event` ABS path (used by linen_sync_reply and before-linen drain) to pass raw tablet y to `send_cursor_checked`, which clamped it to 719.

## Fix

Removed the duplicate `let ay = arg1 as i32;` assignment. y now uses the normalized value from `normalize_abs_coord`.

## Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs:4466` | Removed shadowing `let ay = arg1 as i32;` |
