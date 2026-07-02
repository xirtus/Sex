# LIFECYCLE_TILING_WIRING_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Wires `tile_visible_frames()` after two lifecycle events that change the visible frame count or tiled geometry but were missing tiling calls: **minimize** and **unzoom**. Restore, close, scene switch, tab switch, and zoom were already wired.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +12 lines (2 tiling call sites + 2 proof markers) |
| `docs/handoff/LIFECYCLE_TILING_WIRING_V1.md` | New handoff doc |

---

## Tiling Call Sites Audit

| Operation | Before | After | Status |
|-----------|--------|-------|--------|
| **minimize_frame()** | ❌ Missing | ✅ `tile_visible_frames()` after `snap_capture_layout()` | **Added** |
| **restore_minimized_frame()** | ✅ Present at line 3286 | ✅ Unchanged | Already wired |
| **unzoom_frame()** | ❌ Missing | ✅ `tile_visible_frames()` after `snap_capture_layout()` | **Added** |
| **zoom_frame()** | ❌ Not added | ❌ Not added | Correct — zoomed frames are excluded from tiling tree (line 823) |
| **close_surface_from_frame_light()** | ✅ Present at line 3106 | ✅ Unchanged | Already wired |
| **DestroyFocused handler** | ✅ Present at line 5453 | ✅ Unchanged | Already wired |
| **switch_scene()** | ✅ Present at lines 2366, 2386 | ✅ Unchanged | Already wired |
| **switch_to_tab()** | ✅ Present at line 3286 | ✅ Unchanged | Already wired |

### Call sites intentionally NOT added

| Operation | Reason |
|-----------|--------|
| **zoom_frame()** | Zoomed frames excluded from tiling (line 823). Fills full content area via `layout_maximize()`. Tiling would be a no-op. |
| **Pointer motion** | No tiling on pointer motion — tiling is a lifecycle-driven layout, not per-frame reactive. |
| **During drag** | No tiling during drag — drag cancels before close/tombstone; minimize/zoom guard with `surface_is_alive()` which fails if drag target is dead. |
| **Atlas navigation** | No tiling during arrow key navigation — only on confirm/cancel which call `atlas_clear_stub()` → `tile_visible_frames()`. |

---

## Code Changes

### minimize_frame() — after snap_capture_layout()
```rust
    // A8: Re-tile after minimize — frame removed from visible set.
    tile_visible_frames();
    static mut TILE_AFTER_MINIMIZE_BUDGET: u32 = 8;
    let b = &mut TILE_AFTER_MINIMIZE_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.tile.after_minimize] frame={}", frame_id); }
    snap_capture_layout();
```

### unzoom_frame() — after snap_capture_layout()
```rust
    // A8: Re-tile after unzoom — frame returns to tiled layout.
    tile_visible_frames();
    static mut TILE_AFTER_UNZOOM_BUDGET: u32 = 8;
    let b = &mut TILE_AFTER_UNZOOM_BUDGET;
    if *b > 0 { *b -= 1; serial_println!("[shell.tile.after_unzoom] frame={}", frame_id); }
    snap_capture_layout();
```

---

## Proof Markers Added

| Marker | Location | Budget | When |
|--------|----------|--------|------|
| `[shell.tile.after_minimize]` | `minimize_frame()` | 8 | After minimize tiling |
| `[shell.tile.after_unzoom]` | `unzoom_frame()` | 8 | After unzoom tiling |

Both budgeted at 8 — tiling on minimize/unzoom is not hot-path but budgets prevent log spam under rapid toggle.

---

## Safety Notes

### Drag safety
- `minimize_frame()` calls `clear_drag_if_dead()` before tiling
- `unzoom_frame()` does not interact with drag (drag only applies to non-zoomed frames via rim drag)
- `close_surface_from_frame_light()` cancels drag before lifecycle transition (line 3047)

### Focus safety
- `minimize_frame()` calls `clear_focus_if_dead()` before tiling
- `unzoom_frame()` preserves focus (comment line 3489)
- `tile_visible_frames()` itself calls `clear_focus_if_dead()` on empty scene (line 836)

### Tombstone safety
- `tile_visible_frames()` skips dead surfaces via `[shell.tile.skip_dead]` guard (line 850-854)
- Minimized frames are excluded from tiling (line 819)
- Zoomed frames are excluded from tiling (line 823)

### Generation safety
- No FocusRef/generation interaction — tiling does not change focus, only layout

---

## Behavior Changes

- **Minimize now tiles:** When a frame is minimized, the remaining visible frames are immediately re-tiled to fill the vacated space. Previously only a layout snapshot was captured (`snap_capture_layout()`).
- **Unzoom now tiles:** When a frame is unzoomed (restored from maximized), it returns to the tiled layout. Previously only the frame's normal geometry was restored without re-tiling, which could leave the unzoomed frame at its normal position while other frames still occupied its tiled slot.

---

## Remaining Gaps

- **Hidden state drift (from A8):** `set_lifecycle_state(sid, LifecycleState::Hidden)` is never called on scene switch. Surfaces in non-active scenes retain `Visible` lifecycle state. Does not affect tiling — `tile_visible_frames()` checks `frame.scene_id` directly. Will be addressed in `HIDDEN_STATE_TRACKING_CLEANUP_V1`.

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
# Warnings: only pre-existing
```

---

## Document References

- `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` — proof scenario audit (pre-requisite)
- `docs/handoff/A6_TOMBSTONE_DEBUG_EVENTS_V1.md` — tombstone guards
- `servers/silk-shell/src/main.rs` — implementation

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Wire tiling after minimize and unzoom. 2 call sites + 2 proof markers. | LIFECYCLE_TILING_WIRING_V1 |
