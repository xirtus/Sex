# TILING_ENGINE_HARDENING_V1

**Status:** Active  
**Purpose:** Harden the tiling engine so zoomed frames are excluded from layout computation, preventing zoom state corruption.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Prerequisites:** FRAME_LIFECYCLE_HARDENING_V1 (b0f2848)

---

## 1. Tiling Engine Invariants

```
┌─────────────────────────────────────────────────┐
│           Tiling Engine Invariants               │
├─────────────────────────────────────────────────┤
│ I1: Every non-minimized frame in active scene    │
│     with alive surface receives valid geometry   │
│ I2: Layout positions are non-overlapping         │
│ I3: Layout covers the full content area          │
│ I4: Minimized frames are excluded from tiling    │
│ I5: Zoomed frames are excluded from tiling       │  ← NEW
│ I6: Dead surfaces are excluded from 0xEC calls   │
│ I7: Layout is computed deterministically from    │
│     FRAMES array order                           │
│ I8: tile_visible_frames() is called after every  │
│     mutation of the visible frame set            │
└─────────────────────────────────────────────────┘
```

### Invariant I5 (new)

When a frame is zoomed, its surface occupies the full content area via `layout_maximize()` geometry, bypassing the tiling engine. The tiling engine must NOT include zoomed frames in its layout computation. If `tile_visible_frames()` assigns a tiled position to a zoomed surface, the zoom geometry is overwritten, but `FRAME_FLAG_ZOOMED` remains set — creating a zombie zoom state where the flag says zoomed but the geometry is tiled.

**Without this invariant**: Scene switch, surface toggle, surface close, or any other tiling-triggering event while a frame is zoomed silently corrupts the zoom state.

---

## 2. Imperfections Found

| # | Issue | Location | Severity |
|---|-------|----------|----------|
| 1 | **`tile_visible_frames()` does not exclude zoomed frames** — The collection loop skips minimized frames (`FRAME_FLAG_MINIMIZED`) but has no check for `FRAME_FLAG_ZOOMED`. When `tile_visible_frames()` runs while a frame is zoomed, the zoomed surface receives a tiled position via 0xEC, overwriting its full-content-area zoom geometry. The zoom flag persists, leaving the frame in an inconsistent state (flagged zoomed but geometrically tiled). | `tile_visible_frames()` line ~712 | Medium |

### Trigger scenarios

| Scenario | Effect |
|----------|--------|
| User zooms frame, then switches to another scene and back | `switch_scene()` → `tile_visible_frames()` overwrites zoomed position |
| User zooms frame, then toggles Linen/Quil | `toggle_linen()`/`toggle_quil()` → `tile_visible_frames()` overwrites zoomed position |
| User zooms frame, then closes another surface | `close_surface_from_frame_light()` → `tile_visible_frames()` overwrites zoomed position |
| User zooms frame, then restores a minimized frame | `restore_minimized_frame()` → `tile_visible_frames()` overwrites zoomed position |

### Resulting zombie zoom

After corruption:
- Surface is at tiled position (e.g., right half in 2-frame layout)
- `FRAME_FLAG_ZOOMED` is still set (not cleared by tiling)
- `frame_is_zoomed()` returns true
- If user calls unzoom, it restores saved normal geometry (which may now conflict with current layout)
- Subsequent tiling events move the surface to different tiled positions, but zoom flag never clears

---

## 3. Patch Summary

### `tile_visible_frames()` — added zoomed-frame exclusion

After the minimized-frame skip (line ~712):

```rust
// Zoomed frames are excluded from tiling — their surface occupies the
// full content area via layout_maximize(). Tiling them would overwrite
// the zoomed position with a tiled position, corrupting the zoom state.
if (frame.flags & FRAME_FLAG_ZOOMED) != 0 { continue; }
```

This is a one-line addition between the minimized check and the tab access, mirroring the same pattern used for minimized frames.

---

## 4. Negative-Case Checklist

| Scenario | Behavior | Status |
|----------|----------|--------|
| Zoom frame, switch scene away and back | Zoomed frame excluded from tiling, zoom preserved | ✅ **NEW I5** |
| Zoom frame, toggle another surface | Zoomed frame excluded from tiling, zoom preserved | ✅ **NEW I5** |
| Zoom frame, close another surface | Zoomed frame excluded from tiling, zoom preserved | ✅ **NEW I5** |
| Zoom frame, restore minimized frame | Zoomed frame excluded from tiling, zoom preserved | ✅ **NEW I5** |
| Zoom frame, then unzoom | Normal geometry restored, frame re-integrated on next tiling | ✅ |
| Normal (non-zoomed) tiling | Unchanged — all visible frames included | ✅ |
| All frames zoomed | count=0, early return, no 0xEC calls | ✅ |
| Mixed zoomed and non-zoomed frames | Non-zoomed frames tiled normally, zoomed excluded | ✅ |
| Minimized + zoomed frame | Checked for minimized first (continue), never reaches zoom check | ✅ (redundant guard) |

---

## 5. Files Changed

- `servers/silk-shell/src/main.rs` — +3 lines (one-line check + two-line comment) in `tile_visible_frames()`

## 6. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add FRAME_FLAG_ZOOMED exclusion to tile_visible_frames(), invariant I5 | TILING_ENGINE_HARDENING_V1 |
