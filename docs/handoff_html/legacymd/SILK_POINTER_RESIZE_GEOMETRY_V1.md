# SILK_POINTER_RESIZE_GEOMETRY_V1 Handoff

**Status:** Complete  
**Date:** 2026-05-20  
**File patched:** `servers/silk-shell/src/main.rs`  
**Depends on:** `SILK_POINTER_RESIZE_STATE_V1`

---

## What Was Done

Applied pointer resize geometry to the display. Pointer dragging on frame edges
now actually resizes surfaces via the existing `0xEC` display path. No new ABI.
No sexdisplay edits. No kernel/sex-pdx edits.

### New Functions

#### `compute_resize_rect(cur_x, cur_y, cur_w, cur_h, edge, dx, dy) -> (i32, i32, u32, u32)`

Pure function. Computes new geometry from current bounds + edge + pointer delta.
- Translates edge semantics: Left/Top = leading edge moves (origin shifts), Right/Bottom = trailing edge moves
- Clamps size via existing `clamp_surface_size` (respects `P.min_width`/`P.min_height`)
- Adjusts leading-edge position when size clamping occurs (opposite edge stays fixed)
- Final position clamped via `clamp_position` (stays on-screen above SilkBar)

#### `apply_resize_geometry(dx: i32, dy: i32) -> bool`

Unsafe. Called every pointer move event while `Resizing` state is active.
- Reads current `INTERACTION` for `surface_id` and `edge`
- Checks surface alive; if dead → `[silk.resize.reject.dead]` + transition Idle
- Checks frame zoom state; if zoomed → skip (preserve zoomed semantics)
- Gets current bounds via `get_surface_bounds`
- Calls `compute_resize_rect` for new geometry
- Short-circuits if geometry unchanged (no display IPC)
- Logs `[silk.resize.clamp]` if size was clamped to minimum or screen boundary
- Logs `[silk.resize.apply]` unconditionally when geometry changes
- Sends `pdx_call(SLOT_DISPLAY, 0xEC, sid, (ny<<32|nx), (nh<<32|nw))`
- Calls `update_local_geometry(sid, nx, ny, nw, nh)` to keep all statics in sync
- Logs `[silk.resize.flush]` after display call (budgeted ×16)

### Display Path

Same opcode as keyboard resize and zoom: `0xEC` = move+resize upsert.
No new ABI. `update_local_geometry` handles all surface-specific statics:
- `SURFACE_ID_APP`: updates `WINDOWS[1].desc.{x,y,width,height}` + `SURFACE_100_W/H`
- `SURFACE_ID_STATIC`: updates `SURFACE_101_{X,Y,W,H}`
- `SURFACE_ID_TEST3`: updates `SURFACE_102_{X,Y,W,H}`
- `SURFACE_ID_TEST4`: updates `SURFACE_103_{X,Y,W,H}`

### Geometry Semantics Per Edge

| Edge | x changes? | y changes? | w changes? | h changes? | Formula |
|------|-----------|-----------|-----------|-----------|---------|
| Right | no | no | yes | no | w += dx |
| Left | yes | no | yes | no | x += dx; w -= dx |
| Bottom | no | no | no | yes | h += dy |
| Top | no | yes | no | yes | y += dy; h -= dy |
| BottomRight | no | no | yes | yes | w += dx; h += dy |
| BottomLeft | yes | no | yes | yes | x += dx; w -= dx; h += dy |
| TopRight | no | yes | yes | yes | w += dx; y += dy; h -= dy |
| TopLeft | yes | yes | yes | yes | x += dx; w -= dx; y += dy; h -= dy |

### State Preserved

- Dragging, ClickPending, PanelActive, frame lights, keyboard resize: unchanged
- Zoomed frames: resize hits detected but geometry not applied (skip in `apply_resize_geometry`)
- Minimized frames: chrome hit targets blocked upstream by `frame_accepts_input`
- `normal_x/y/w/h` in ShellFrame: NOT updated during pointer resize (only written on `zoom_frame`)
  Consequence: after a pointer resize followed by zoom, unzoom restores to PRE-RESIZE geometry.
  This matches the pattern from keyboard resize (keyboard resize also doesn't update normal_*).

### Markers

| Marker | When | Budget |
|--------|------|--------|
| `[silk.resize.apply]` | Every geometry-changing move event | unlimited |
| `[silk.resize.clamp]` | When min-size or screen-boundary clamping occurs | 16 |
| `[silk.resize.flush]` | After pdx_call(0xEC) sent to display | 16 |
| `[silk.resize.reject.dead]` | Dead surface or no bounds available | unlimited |
| (from V1) `[silk.resize.hit]` | Zone detected on click | unlimited |
| (from V1) `[silk.resize.begin]` | Resizing state entered | 8 |
| (from V1) `[silk.resize.delta]` | Each move while Resizing | 16 |
| (from V1) `[silk.resize.end]` | Pointer release | unlimited |

### Call Sites

`apply_resize_geometry` called in:
1. USB mouse report handler (after `drag_move_focused` and `resize_accumulate_delta`)
2. HID EV_REL handler (same pattern)

---

## Build Verification

Build gate: `bash scripts/entrypoint_build.sh`  
Result: **success** (`[SEXOS ENTRYPOINT] success`)  
No new warnings. No `#PF`, `#GP`, panic, or fault.kill.

---

## Recurrence Notes

- `update_local_geometry` is the correct helper for keeping all surface statics in sync.
  Using it avoids the 4-way match duplicated in old keyboard resize code.
- `0xEC` is the only surface move/resize opcode (create + update). No `0xED` exists.
- `compute_resize_rect` does NOT update `ShellFrame.normal_*`. If zoom/unzoom behavior
  after pointer resize needs to use the resized geometry, `normal_*` must be updated
  after `update_local_geometry`. Currently matches keyboard resize behavior (no update).
- Zoomed-frame check uses `frame_is_zoomed` directly. If a resize starts on an unzoomed
  frame and the user zooms during drag (unlikely but possible), `apply_resize_geometry`
  will start skipping on the next move event. The Resizing FSM state remains until release.
