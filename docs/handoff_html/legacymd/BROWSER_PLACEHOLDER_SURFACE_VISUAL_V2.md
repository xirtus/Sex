# BROWSER_PLACEHOLDER_SURFACE_VISUAL_V2

**Status:** PASS REVIEW ONLY — 98/98 gates, 0 faults.
**Date:** 2026-05-16
**Depends on:** `SURFACE_ID_REGISTRY_FIX_WEBSTUB_V1.md`.

---

## Result: PASS REVIEW ONLY — Surface deferred

SID collision resolved (205), but surface creation requires expanding
`APP_SURFACES[7]` to `[8]` + new frame constants — structural change deferred.

---

## Before/After

| Field | Before (V1) | After (V2) |
|-------|------------|------------|
| sid | 205 | 205 |
| SID collision | 0 | 0 |
| APP_SURFACES registered | No | **Still no** (needs array expansion) |
| focusable | 0 | 0 |
| surface | 0 | 0 |
| rendered | 0 | 0 |
| network | 0 | 0 |

---

## Blocker: APP_SURFACES Expansion

To create a WebStub surface:
1. Add `SURFACE_ID_BROWSER = 205`
2. Add `BROWSER_FRAME_ID` + boot geometry constants
3. Add 8th `AppSurfaceSpec` entry → expand array from `[7]` to `[8]`
4. Add `SURFACE_205_X/Y/W/H` position tracking
5. Register frame in scene layout

This is a structural change affecting frame count, layout, golden hash.

---

## Files Changed: silk-shell (review markers updated), spindle (command text)

## Proof: 98/98 PASS, 0 faults

## Fault Count: **0**

## Next: Create APP_SURFACES entry for Browser (future prompt)
