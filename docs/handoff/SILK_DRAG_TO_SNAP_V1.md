# SILK_DRAG_TO_SNAP_V1 Handoff

**Status:** Complete  
**Date:** 2026-05-20  
**File patched:** `servers/silk-shell/src/main.rs`  
**Depends on:** `SILK_POINTER_RESIZE_STATE_V1`, `SILK_POINTER_RESIZE_GEOMETRY_V1`

---

## What Was Done

Drag-to-snap at release: dragging a surface to a screen edge and releasing now triggers
snap geometry. Uses existing `0xEC` display path. No new ABI. No sexdisplay edits.
All 3 drag release sites updated.

### New Constants

```rust
const SNAP_EDGE_PX: i32 = 24;  // px from left/right edge → snap
const SNAP_TOP_PX: i32 = 16;   // px below SilkBar → zoom
```

Added near `FRAME_RESIZE_ZONE_PX`.

### New Function: `try_snap_on_drag_release(surface_id, px, py) -> bool`

Called at every drag release. Priority: **top** > **left** > **right** > none.

| Zone | Condition | Action |
|------|-----------|--------|
| Top  | `py < bar_height + 16` | `zoom_frame(frame_id)` if not already zoomed |
| Left | `px < 24` | half-width left: `(0, bar_height, width/2, height-bar_height)` |
| Right | `px >= width - 24` | half-width right: `(width/2, bar_height, width-width/2, height-bar_height)` |
| None | — | `[silk.snap.none]`, return false |

Guards:
- Dead/tombstoned surface → `[silk.snap.reject.dead]` + return false
- No frame → `[silk.snap.none]` + return false
- Minimized frame → `[silk.snap.reject.dead]` + return false

Left/right snap: `pdx_call(SLOT_DISPLAY, 0xEC, ...)` + `update_local_geometry`.  
Top snap: `zoom_frame(frame_id)` which saves `normal_*` and sends maximized geometry.

### Markers

| Marker | When |
|--------|------|
| `[silk.snap.hit.left]` | Left edge zone detected |
| `[silk.snap.hit.right]` | Right edge zone detected |
| `[silk.snap.hit.top]` | Top zone detected |
| `[silk.snap.apply]` | Snap geometry actually applied (includes `kind=left/right/top`) |
| `[silk.snap.reject.dead]` | Dead, tombstoned, or minimized surface |
| `[silk.snap.none]` | No snap zone matched, or top zoom skipped |

### Release Site Coverage

| Site | Path | `mutated` updated |
|------|------|------------------|
| Early HID handler (`handle_hid_event_drain`) | `let _ = try_snap_on_drag_release(...)` | no (function returns void) |
| USB mouse report handler | `if try_snap_on_drag_release(...) { mutated = true; }` | yes |
| HID EV_BTN handler | same | yes |

### What Was NOT Done

- Non-frame surfaces (Quil, Linen, Mesh) don't snap — `frame_for_surface` returns None
- Already-zoomed frames: top snap is skipped (`[silk.snap.none] reason=top_zoom_skip`)
- Left/right snap for zoomed frames: snap IS applied (drag released a zoomed surface to a side edge)
- `normal_*` in ShellFrame: updated by `zoom_frame` for top snap, NOT updated for left/right snap
  (matches existing keyboard SnapLeft/SnapRight behavior which also doesn't update `normal_*`)

---

## Build Verification

Build gate: `bash scripts/entrypoint_build.sh`  
Result: **success** (`[SEXOS ENTRYPOINT] success`)  
No new warnings. No `#PF`, `#GP`, panic, or fault.kill.

All existing drag markers preserved (`[shell.interact.drag.end]`, `[shell.drag.end]`).
Resize markers preserved — Resizing release path unchanged.

---

## Recurrence Notes

- 3 button-release handler sites must ALL be updated together for snap to work on all input paths.
- Top snap priority MUST be above left/right to avoid corner ambiguity at `(px<24, py<66)`.
- `zoom_frame` not `toggle_zoom_frame` — toggle would unzoom on re-drag to top. Use zoom only.
- Left/right snap geometry matches tile_active_scene_frames 2-frame layout (half-width split).
  If DesktopPolicy changes screen dimensions (P.width/P.height), snap automatically follows.
- `update_local_geometry` is mandatory after `pdx_call(0xEC)` — omitting it desyncronizes
  local statics from display state, breaking future hit-test and resize operations.
