# FRAME_LIFECYCLE_HARDENING_V1

**Status:** Active  
**Purpose:** Harden Silk Frame close paths so that destroying a surface also clears any drag/hover interaction state referencing it.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Prerequisites:** SCENE_LIFECYCLE_HARDENING_V1 (61bfaef)

---

## 1. Frame Lifecycle Invariants

```
┌─────────────────────────────────────────────────┐
│           Frame Lifecycle Invariants             │
├─────────────────────────────────────────────────┤
│ I1: Every live frame belongs to exactly one scene│
│ I2: Every frame has at least one live tab        │
│ I3: A closed surface is removed from its tab     │
│ I4: A tab with zero live surfaces is tombstoned  │
│ I5: Closing a surface clears focus if focused    │
│ I6: Closing a surface clears drag if dragged     │  ← NEW
│ I7: Closing a surface clears hover if hovered    │  ← NEW
│ I8: Tombstoned frames are excluded from tiling   │
│ I9: Frame count never exceeds MAX_FRAMES (4)     │
│ I10: Surface IDs are not reused within tombstone │
└─────────────────────────────────────────────────┘
```

### Invariants I6, I7 (new)

When a surface is closed (via `close_surface_from_frame_light()` or the keyboard `SurfaceAction::DestroyFocused` path), any active drag targeting that surface becomes dangling. Similarly, if the closed surface's frame was hovered, the hover state references a now-invalid frame.

- **I6**: Closing a surface must check `clear_drag_if_dead()` — if the drag target was the closed surface, transition to Idle.
- **I7**: Closing a surface must check `clear_hover_if_wrong_scene()` — if the hovered frame was the closed surface's frame, reset to 0.

---

## 2. Imperfections Found

| # | Issue | Location | Severity |
|---|-------|----------|----------|
| 1 | **Missing `clear_drag_if_dead()` in close surface path** — `close_surface_from_frame_light()` called `clear_focus_if_dead()` but did not clear drag state. If the closed surface was being dragged, the drag state would persist with a dead surface_id. | `close_surface_from_frame_light()` | Medium |
| 2 | **Missing `clear_hover_if_wrong_scene()` in close surface path** — Same function cleared focus but not hover. If the closed surface's frame was hovered, `HOVERED_FRAME_ID` would point to a now-invalid frame. | `close_surface_from_frame_light()` | Low |
| 3 | **Missing both guards in keyboard `DestroyFocused` handler** — The `SurfaceAction::DestroyFocused` dispatch path had no interaction state cleanup before `snap_capture_layout()`. | `SurfaceAction::DestroyFocused` handler | Medium |

---

## 3. Patch Summary

### `close_surface_from_frame_light()` — added 2 guards

Inserted after `clear_focus_if_dead()` (line ~2088):

```rust
// Clear drag if the closed surface was being dragged (surface is now dead).
clear_drag_if_dead();
// Clear hover if the closed surface's frame is no longer valid.
clear_hover_if_wrong_scene();
```

### `SurfaceAction::DestroyFocused` handler — added 2 guards

Inserted before `snap_capture_layout()` (line ~4087):

```rust
clear_drag_if_dead();
clear_hover_if_wrong_scene();
```

Both sites reuse existing guards — no new functions, no new imports.

---

## 4. Negative-Case Checklist

| Scenario | Behavior | Status |
|----------|----------|--------|
| Close surface while dragging it | Drag cancelled to Idle | ✅ **NEW I6** |
| Close surface while its frame is hovered | Hover cleared to 0 | ✅ **NEW I7** |
| Close surface while dragging a different surface | Drag preserved (different surface_id) | ✅ |
| Close surface via keyboard DestroyFocused while dragging | Drag cancelled | ✅ **NEW I6** |
| Close surface via keyboard DestroyFocused while hovered | Hover cleared | ✅ **NEW I7** |
| Close surface (no drag, no hover) | No-op guards, no side effects | ✅ |
| Close surface while focused (pre-existing) | Focus cleared via `clear_focus_if_dead()` | ✅ I5 |
| Close surface in wrong scene (pre-existing) | Hover cleared via existing guard in tiling | ✅ I7 (pre-existing) |
| Drag dead surface (surface destroyed during drag) | Cancelled by `clear_drag_if_dead()` | ✅ (dual coverage) |

---

## 5. Files Changed

- `servers/silk-shell/src/main.rs` — +4 lines: 2 guards in `close_surface_from_frame_light()`, 2 guards in `SurfaceAction::DestroyFocused` handler

## 6. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add clear_drag_if_dead + clear_hover_if_wrong_scene to close surface paths and DestroyFocused handler | FRAME_LIFECYCLE_HARDENING_V1 |
