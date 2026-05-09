# POINTER_ABS_EDGE_GUARD_V2_ALL_PATHS

**Date:** 2026-05-08
**Status:** MERGED

## Bypass found

The `handle_hid_event` helper (used by linen_sync_reply and before-linen drain) had its own EV_ABS handler WITHOUT the edge guard. Tablet max-edge reports arriving during Linen sync or the pre-paint drain bypassed the guard and set cursor to x=1279,y=719.

## Fix

Added identical edge guard to `handle_hid_event` EV_ABS handler. Now all three ABS paths use the same guard:

| Path | Location | Guard |
|------|----------|-------|
| Main dispatch | line 13153 | ✅ |
| handle_hid_event (linen_sync_reply) | line 4427 | ✅ (fixed) |
| handle_hid_event (before-linen drain) | line 11514 | ✅ |
