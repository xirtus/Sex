# LINEN_SURFACE_CONTROL_V1

**Status:** Active  
**Purpose:** Make Linen a first-class shell-managed surface under Scene/Frame/Tab/Tiling model.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.

---

## Current Linen Path (before patch)

Linen (surface_id=200) was a standalone surface:
- Created by `servers/linen/src/main.rs` via 0xEC at boot with hardcoded coords (900, 500, 300, 150)
- Tracked in silk-shell via `SURFACE_200_X/Y/W/H` statics
- Positioned in `tile_visible_frames()` via hardcoded match arm
- **NOT** part of `FRAMES` array — no `ShellFrame` ownership
- `surface_in_active_scene()` returned `true` via fallback (not wrong, but not scene-aware)
- `frame_for_surface(200)` returned `None`
- `is_closeable_surface(200)` returned `false` (correct — OS surface)
- No minimize/restore/zoom/frame-navigation support

---

## Changes

### 1. `update_local_geometry()` — added `SURFACE_ID_LINEN` arm

Linen's position statics (`SURFACE_200_X/Y/W/H`) are now updated whenever geometry changes (zoom, unzoom, minimize restore, tab switch).

### 2. `emit_snapshot()` — added Linen OP_SURFACE_UPDATE

Linen position is now reported via `OP_SURFACE_UPDATE` in the snapshot path.

### 3. New Linen Control Helpers

Inserted before `// ── Frame Chrome Query Helpers ──`:

| Helper | Description |
|--------|-------------|
| `ensure_linen_frame()` | Creates `ShellFrame` with `frame_id=2` in first empty FRAMES slot, wrapped around `ShellTab` with `surface_id=200`. Lazy — only creates if not present. Returns `Some(frame_id)` or `None` if no slot. |
| `open_linen_in_active_scene()` | Opens Linen in current scene: ensures frame exists, un-minimizes if needed, updates scene_id, positions via 0xEC, tiles, focuses. |
| `focus_or_open_linen()` | Focuses Linen if already visible in active scene; otherwise calls `open_linen_in_active_scene()`. |
| `toggle_linen()` | Toggles Linen visibility: minimize if visible, open if not. |
| `linen_frame_id()` | Returns Linen's frame_id if frame exists (`Some(2)` or `None`). |

### Frame allocation

- `LINEN_FRAME_ID = 2` (frame 1 = APP/STATUS)
- Linen frame uses `FRAME_FLAG_TOP_BAR` (matching default frame style)
- Normal geometry: (900, 500, 300, 150) matching Linen's hardcoded boot position
- Scene: assigned to `ACTIVE_SCENE_IDX` on open

### Behavior guarantees

- Boot visual unchanged (frame created lazily, not at boot)
- All existing `surface_is_alive`, `is_focusable`, `is_closeable` rules preserved
- Linen now participates in `tile_visible_frames()` as a frame-owned surface
- Linen participates in `sync_scene_visibility()` (hidden when wrong scene)
- Linen participates in `snap_capture_layout()` / `snap_restore_layout()`
- Linen participates in `focus_next_frame()` / `focus_prev_frame()` navigation
- Linen can be minimized/restored via existing `minimize_frame` / `restore_minimized_frame`
- Linen can be zoomed via `toggle_zoom_focused_frame()` (but zoom is unusual for a file viewer — not called by default)

---

## Build Result

```
Finished release profile [optimized] in 0.04s
Warnings: 196 (all pre-existing)
Errors: 0
```

## Files Changed

- `servers/silk-shell/src/main.rs` — added ~140 lines

## Not In Scope (future)

- Actual file/object browser logic in Linen
- Keyboard shortcut bindings for linen helpers (Ctrl+L, etc.)
- Title/idle behavior for Linen
- Sexstore integration for Linen state
- Multiple Linen instances
- Linen window chrome/tab strip

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Linen as first-class shell-managed surface | LINEN_SURFACE_CONTROL_V1 |
