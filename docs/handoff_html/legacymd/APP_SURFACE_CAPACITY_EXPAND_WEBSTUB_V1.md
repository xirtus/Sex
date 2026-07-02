# APP_SURFACE_CAPACITY_EXPAND_WEBSTUB_V1

**Status:** PASS IMPLEMENTED — 97 PASS, 1 SKIP, 0 faults.
**Date:** 2026-05-16
**Depends on:** `APP_SURFACE_CAPACITY_AUDIT_V1.md`.

---

## Result: PASS — WebStub surface created

APP_SURFACES expanded [7]→[8]. Browser surface at SID 205, Frame 8.
Golden hash unchanged (surface below 50-row hash strip).

---

## Surface Expansion Table

| Field | Before | After |
|-------|--------|-------|
| APP_SURFACES entries | 7 | **8** |
| WebStub SID | 205 | 205 |
| WebStub Frame | — | **8** |
| focusable | 0 | **1** |
| surface | 0 | **1** |
| rendered | 1 (shell registers) | 1 |
| Geometry | — | (500,100,400,300) |
| network/engine | 0/0 | 0/0 |

## Golden Hash: MATCH — 0xFD6093AC9ADE7B4D (unchanged)

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +30 — SURFACE_ID_BROWSER, BROWSER_FRAME_ID, boot geometry, APP_SURFACES[8], SURFACE_205 tracking, bounds lookups, truth markers |
| `apps/spindle/src/main.rs` | browser-surface command updated |

## Proof: 97 PASS, 1 SKIP, 0 faults

## Fault Count: **0**

## Commit
```bash
git add servers/silk-shell/src/main.rs apps/spindle/src/main.rs docs/handoff/APP_SURFACE_CAPACITY_EXPAND_WEBSTUB_V1.md
git commit -m "feat(silk): WebStub surface APP_SURFACES[8] frame 8"
```
