# SCENE_SHORTCUTS_AND_PERSISTENCE_V1A

**Status:** Active  
**Purpose:** Deterministic shell command helpers for scene/frame/tab actions + in-memory scene layout snapshot.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.

---

## Part A: SCENE_SHORTCUTS_V1

### New Helpers

All in `servers/silk-shell/src/main.rs` under `// ── Scene Shortcut Command Helpers ──` section (inserted after `sync_scene_visibility()`).

| Helper | Description |
|--------|-------------|
| `switch_scene(scene_idx: u8)` | Switch to a workspace, clamped to `WORKSPACE_COUNT-1`. Calls `sync_scene_visibility()`, clears focus/drag/hover, re-tiles. |
| `next_scene()` | Cycle to next workspace (wrapping). |
| `prev_scene()` | Cycle to previous workspace (wrapping). |
| `focus_next_frame()` | Move focus to next non-minimized frame in active scene with alive/tab (wrapping). |
| `focus_prev_frame()` | Move focus to previous non-minimized frame (wrapping). |
| `focus_next_tab()` | Advance to next tab in focused frame (wrapping, no-op if ≤1 tab). |
| `focus_prev_tab()` | Go to previous tab in focused frame (no-op if ≤1 tab). |
| `toggle_minimize_focused_frame()` | Toggle minimize/restore for focused frame. |
| `toggle_zoom_focused_frame()` | Toggle zoom/unzoom for focused frame. |
| `close_focused_tab_or_frame_safe()` | Close focused frame's active surface via `close_surface_from_frame_light()`. Safe: only closeable surfaces. |

All helpers:
- Use existing `Scene`/`ShellFrame`/`ShellTab` model
- Respect `ACTIVE_SCENE_IDX`, tombstones, minimized flags
- Call `try_set_focus()` / `sync_scene_visibility()` / `tile_visible_frames()` as needed
- Log budgeted `[shell.shortcut.*]` markers
- No new IPC, no new ABI, no kernel changes

### How to wire to keyboard

Add new `scancode_to_action()` entries and dispatch to these helpers in the `OP_HID_EVENT` handler's `SurfaceAction` match.

---

## Part B: SCENE_PERSISTENCE_V1A

### Snapshot Structs

```rust
struct FrameSnapshot {
    present: u8,       // 0 = empty, 1 = valid
    frame_id: u32,
    scene_id: u8,
    active_tab: u8,
    tab_count: u8,
    flags: u32,
    normal_x: i32, normal_y: i32,
    normal_w: u32, normal_h: u32,
    tab_surfaces: [u64; 8],  // surface IDs for each tab slot
}

struct SceneLayoutSnapshot {
    magic: u8,         // 0x53 ('S')
    version: u8,       // 0x01
    active_scene: u8,
    frame_count: u8,
    checksum: u8,      // XOR over all bytes (excl. checksum itself)
    _reserved: [u8; 3],
    frames: [FrameSnapshot; 4],
}
```

### Functions

| Function | Description |
|----------|-------------|
| `snap_capture_layout()` | Captures current `FRAMES` + `ACTIVE_SCENE_IDX` into global `SCENE_SNAPSHOT` |
| `snap_validate(snap)` | Checks magic, version, frame_count, flags bounds, XOR checksum |
| `snap_restore_layout()` | Restores from global snapshot: validates, skips dead/tombstoned surfaces, clamps geometry, calls `sync_scene_visibility()`, `tile_visible_frames()`, clears stale focus/hover/drag |

### Capture call sites wired

Snapshot is captured after every layout mutation:

| Call site | Trigger |
|-----------|---------|
| Boot init (after FRAMES[0] setup) | Initial capture |
| `switch_scene()` | Scene switch via shortcut helpers |
| SilkBar `SwitchWorkspace` handler | Scene switch via silkbar click |
| `close_surface_from_frame_light()` | Close via frame light |
| `minimize_frame()` | Minimize |
| `restore_minimized_frame()` | Restore |
| `zoom_frame()` | Zoom/maximize |
| `unzoom_frame()` | Unzoom/restore |
| `toggle_top_bar_for_active_frame()` | Top bar toggle |
| `switch_to_tab()` | Tab switch |
| `SurfaceAction::RecreateFocused` handler | Recreate focused surface |
| `SurfaceAction::DestroyFocused` handler | Keyboard destroy |
| `SurfaceAction::ResetAll` handler | Reset all surfaces |
| `SurfaceAction::SnapLeft|SnapRight|SnapHome|SnapEnd|Maximize|Center` handler | Snap/maximize actions |

### Safety guarantees

- `snap_restore_layout()` returns `false` (no-op) if snapshot invalid
- Skips tombstoned surfaces (checked via `is_tombstoned()`)
- Skips dead surfaces (checked via `surface_is_alive()`)
- Clamps geometry via `clamp_position()` and `clamp_surface_size()`
- Clears focus/drag/hover after restore
- Minimal frame with no valid tabs is skipped (frame_count adjusted)
- `snap_validate()` enforces flags only contain known bits (MINIMIZED | ZOOMED | TOP_BAR)

---

## Build Result

```
Finished release profile [optimized] in 0.73s
Warnings: 175 (all pre-existing: nested unsafe blocks, static_mut_ref, unused import)
Errors: 0
```

## Files Changed

- `servers/silk-shell/src/main.rs` — added ~380 lines (Part A + Part B)

## Not In Scope (future)

- Keyboard shortcut bindings for the new helpers
- Disk/sexstore persistence of snapshots
- Sexstore integration for scene layout snapshots
- Animation/transition support
- Hover/drag state in snapshots
- Alpha/blur/effects in snapshot

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Scene shortcuts + in-memory snapshot | SCENE_SHORTCUTS_AND_PERSISTENCE_V1A |
