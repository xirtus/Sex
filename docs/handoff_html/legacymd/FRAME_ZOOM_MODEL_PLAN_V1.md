# FRAME_ZOOM_MODEL_PLAN_V1

## Status

Design (2026-05-04). Analysis of zoom/maximize feasibility. No code changed.

---

## Verdict

### ZOOM_MODEL_SAFE_NOW ✅

| Requirement | Feasible? | Evidence |
|-------------|-----------|----------|
| Zoom must not destroy surface | ✅ | 0xEC upsert preserves slot, only changes geometry |
| Zoom must be reversible | ✅ | Save normal geometry before zoom, restore via 0xEC |
| Normal geometry must be preserved | ✅ | Add `normal_*` fields to ShellFrame |
| 0xEC can resize (not just move) | ✅ | sexdisplay handler: `slot.w = w; slot.h = h` |
| No renderer changes needed | ✅ | sexdisplay renders whatever geometry shell sends |
| No kernel/ABI changes | ✅ | Reuses existing 0xEC opcode |
| No framebuffer ownership violation | ✅ | Shell already sends 0xEC for all geometry changes |
| No dynamic allocation | ✅ | ShellFrame.normal_* fit in fixed-size struct |

---

## Current Geometry Authority

### Where shell stores surface geometry

| Surface | Position Authority | Size Authority |
|---------|-------------------|----------------|
| 100 (APP) | `WINDOWS[1].desc.{x, y}` | `SURFACE_100_W`, `SURFACE_100_H` (stale — never updated after boot) |
| 101 (STATIC) | `SURFACE_101_{X, Y}` | `SURFACE_101_{W, H}` |
| 102 (TEST3) | `SURFACE_102_{X, Y}` | `SURFACE_102_{W, H}` |
| 103 (TEST4) | `SURFACE_103_{X, Y}` | `SURFACE_103_{W, H}` |

**Note:** Surface 100 has an inconsistency — `get_surface_bounds()` reads `WINDOWS[1].desc.{x, y}` for position but `SURFACE_100_{W, H}` for size. The SnapLeft/SnapRight/Maximize/Center keyboard actions update `WINDOWS[1].desc.{width, height}` but never write to `SURFACE_100_{W, H}`. This means `get_surface_bounds()` returns stale dimensions after any resize operation on surface 100. This is a pre-existing bug, not introduced by zoom.

### Layout functions (`servers/silk-shell/src/main.rs`, lines 147-157)

```rust
fn layout_maximize() -> (i32, i32, u32, u32) {
    (0, P.bar_height, P.width as u32, (P.height - P.bar_height) as u32)
}
// Returns: (0, 50, 1280, 670) on 1280×720 display
```

The maximized rectangle fills the full display width below the 50px SilkBar top strip.

---

## Can 0xEC Resize? ✅ YES

### sexdisplay 0xEC handler (lines 791-844)

```rust
slot.x = x; slot.y = y; slot.w = w; slot.h = h;
```

The 0xEC opcode updates **both position AND size**. It is already used by the shell's SnapLeft, SnapRight, Maximize, and Center keyboard actions to change both position and dimensions. No separate resizing IPC is needed.

### 0xEB (OP_SURFACE_UPDATE) only updates position

```rust
slot.x = new_x;
slot.y = new_y;
```

0xEB does NOT update width/height — only position. Used for drag. Zoom toggle must use 0xEC.

---

## Proposed Zoom Model

### ShellFrame additions

Add to `ShellFrame` struct (line 215):

```rust
struct ShellFrame {
    frame_id: u32,
    active_tab: u8,
    tab_count: u8,
    tabs: [Option<ShellTab>; MAX_TABS_PER_FRAME as usize],
    flags: u32,
    // ── Zoom/Maximize state ──
    /// Saved normal geometry for unzoom. Valid only when FRAME_FLAG_ZOOMED is set.
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
}
```

**Size impact:** 4 fields × 4 bytes = 16 bytes per frame. With MAX_FRAMES=4, total +64 bytes. Static array `FRAMES: [Option<ShellFrame>; 4]` grows from ~584 bytes to ~648 bytes. Negligible.

**Alternative considered — separate statics:**
```rust
static mut ZOOMED_FRAME_ID: u32 = 0;
static mut ZOOMED_NORMAL_X: i32 = 0;  // ... etc
```
Rejected: only supports one zoomed frame. ShellFrame storage is cleaner for multi-frame future and avoids a separate static mut set.

### Frame flag constant (already have FRAME_FLAG_MINIMIZED = 1 << 0)

```rust
/// ShellFrame.flags: frame is zoomed/maximized (fills content area below SilkBar).
const FRAME_FLAG_ZOOMED: u32 = 1 << 1;
```

### Zoom algorithm (toggle)

```
zoom_toggle_frame(frame_id):
    if frame_is_zoomed(frame_id):
        unzoom_frame(frame_id)
    else:
        zoom_frame(frame_id)

zoom_frame(frame_id):
    let surface_id = active_surface_for_frame(frame_id)
    if surface_id is None or dead: return false
    // Save normal geometry BEFORE zoom
    let bounds = get_surface_bounds(surface_id)
    if bounds is None: return false
    frame.normal_x, frame.normal_y = bounds.x, bounds.y
    frame.normal_w, frame.normal_h = bounds.w, bounds.h
    // Set zoomed flag
    frame.flags |= FRAME_FLAG_ZOOMED
    // Send maximized geometry to sexdisplay
    let (zx, zy, zw, zh) = layout_maximize()
    pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
        (zy as u64) << 32 | zx as u64,
        (zh as u64) << 32 | zw as u64)
    // Update local geometry to match display
    update_local_geometry(surface_id, zx, zy, zw, zh)
    emit [shell.frame.zoom] frame=N surface=N

unzoom_frame(frame_id):
    let surface_id = active_surface_for_frame(frame_id)
    if surface_id is None or dead: return false
    if not frame_is_zoomed(frame_id): return false
    // Restore normal geometry
    let nx = frame.normal_x, ny = frame.normal_y
    let nw = frame.normal_w, nh = frame.normal_h
    frame.flags &= !FRAME_FLAG_ZOOMED
    // Send normal geometry to sexdisplay
    pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
        (ny as u64) << 32 | nx as u64,
        (nh as u64) << 32 | nw as u64)
    // Update local geometry to match display
    update_local_geometry(surface_id, nx, ny, nw, nh)
    emit [shell.frame.unzoom] frame=N surface=N
```

### update_local_geometry helper

Needed because the shell's geometry statics must stay in sync with sexdisplay. After sending 0xEC with new geometry, the shell updates:

```rust
unsafe fn update_local_geometry(surface_id: u64, x: i32, y: i32, w: u32, h: u32) {
    match surface_id {
        SURFACE_ID_APP => {
            WINDOWS[1].desc.x = x; WINDOWS[1].desc.y = y;
            WINDOWS[1].desc.width = w; WINDOWS[1].desc.height = h;
            SURFACE_100_W = w; SURFACE_100_H = h;  // fix stale-dimension bug
        }
        SURFACE_ID_STATIC => {
            SURFACE_101_X = x; SURFACE_101_Y = y;
            SURFACE_101_W = w; SURFACE_101_H = h;
        }
        // ... same for 102, 103
        _ => {}
    }
}
```

This helper also fixes the pre-existing stale-dimension bug for surface 100.

---

## Focus/Drag Invariants

| Invariant | How maintained |
|-----------|---------------|
| Focus stays on zoomed surface | Zoom does not change focus. Surface remains visible and focused. |
| Drag on zoomed surface | Rim drag works normally — surface position changes while still at maximized size. Or: drag on zoomed surface triggers unzoom first. **V1 recommendation**: allow drag to move the zoomed surface. Unzoom is explicit via ZOOM light click. |
| Hit-test works on zoomed surface | 0xEC updates sexdisplay geometry. Shell updates local geometry via `update_local_geometry()`. Both sides agree. |
| Resize keyboard actions on zoomed surface | SnapLeft/SnapRight/Maximize work but may produce unexpected results since surface is already at maximized size. **V1**: unzoom before SnapLeft/SnapRight. Or document as edge case. |
| Keyboard Maximize (0x32) on already-zoomed surface | Should unzoom (toggle behavior). The existing Maximize action is one-way (no toggle). With zoom model, ZOOM light provides toggle. Keyboard Maximize can remain one-way or be updated to toggle — **V1 leaves keyboard Maximize unchanged**, ZOOM light is the toggle. |

---

## Minimize Interaction

| Scenario | Behavior |
|----------|----------|
| Zoomed → Minimize | Save zoom flag. After minimize, geometry in sexdisplay is hidden. Normal geometry stored in ShellFrame. |
| Minimized → Restore | Restore to zoomed state if `FRAME_FLAG_ZOOMED` is set (send 0xEC with maximized geometry). Clear zoom flag if normal restore. |
| Minimized → Zoom (impossible) | Cannot zoom a minimized surface — no visible surface to interact with. ZOOM light is not visible. |
| Zoomed → Minimize → Restore → Unzoom | On minimize, zoom flag stays set. On restore, `restore_minimized_frame()` checks `FRAME_FLAG_ZOOMED` and sends either maximized or normal geometry. On subsequent unzoom click, normal geometry is still in `ShellFrame.normal_*`. |

**Implementation note:** `restore_minimized_frame()` already calls `get_surface_bounds()` and sends 0xEC. It does NOT know about zoom state. To support restore-to-zoomed, the restore function should check `FRAME_FLAG_ZOOMED` after clearing `FRAME_FLAG_MINIMIZED`:

```rust
if (frame.flags & FRAME_FLAG_ZOOMED) != 0 {
    // Restore to zoomed geometry
    let (zx, zy, zw, zh) = layout_maximize();
    pdx_call(SLOT_DISPLAY, 0xEC, surface_id, ...);
} else {
    // Normal restore from get_surface_bounds()
    let bounds = get_surface_bounds(surface_id);
    // ... existing code
}
```

---

## ZOOM Light vs. Keyboard Maximize (0x32)

The existing `SurfaceAction::Maximize` (keyboard scancode 0x32) does a one-way maximize with no toggle and no geometry save. The ZOOM light implements a proper toggle with geometry save/restore.

| Aspect | Keyboard Maximize (0x32) | ZOOM Light |
|--------|-------------------------|------------|
| Saves normal geometry | ❌ | ✅ |
| Toggle (unzoom) | ❌ | ✅ |
| Uses layout_maximize() | ✅ | ✅ |
| Per-surface dispatch | ✅ | ✅ (frame-resolved) |
| Marker | `[silk-shell] Surface N maximized` | `[shell.frame.zoom]` / `[shell.frame.unzoom]` |

**V1:** Both coexist. Keyboard Maximize remains one-way (no toggle). ZOOM light is the proper toggle. Future phase can reconcile.

---

## Implementation Files

### Modified

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | +1 constant (`FRAME_FLAG_ZOOMED`), +4 ShellFrame fields, +2 helpers (`zoom_frame`, `unzoom_frame`), +1 helper (`update_local_geometry`), +ZOOM light dispatch in click handler, +budgeted markers |

### NOT Modified

- `kernel/` — no ABI changes
- `crates/sex-pdx/` — no protocol changes
- `crates/silkbar-model/` — no model changes
- `servers/sexdisplay/` — no renderer changes (reuses 0xEC as-is)
- `servers/silkbar/` — no forwarding changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched
- Any framebuffer path — untouched

---

## Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.zoom] frame=N surface=N` | 8 | ZOOM light click maximizes surface |
| `[shell.frame.unzoom] frame=N surface=N` | 8 | ZOOM light click on zoomed surface restores normal |
| `[shell.frame.zoom.reject] frame=N reason=...` | 4 | ZOOM click rejected (no surface, dead, no frame) |

Pre-existing markers that must still fire:

| Marker | Why still fires |
|--------|----------------|
| `[shell.frame.light.close]` | CLOSE path unchanged |
| `[shell.frame.minimize]` | MINIMIZE path unchanged |
| `[shell.frame.restore]` | Restore path unchanged (may add zoom check) |
| `[shell.frame.rim.drag.start]` | Rim drag on non-light clicks unchanged |
| `[shell.frame.light.hover]` | Light hover detection unchanged |

---

## STOP Conditions

If any of these are encountered during implementation, STOP and re-assess:

1. **0xEC does not actually resize on sexdisplay side** — verified that it does (`slot.w = w; slot.h = h`). STOP only if code review reveals a guard that prevents resize.
2. **layout_maximize() does not produce correct maximized rectangle** — verified `(0, 50, 1280, 670)`. STOP if SilkBar height changes or display dimensions change.
3. **ShellFrame.normal_* fields cause alignment/init issues** — all fields are plain integers, no alignment concerns. STOP if `#[repr(C)]` layout changes cause issues.
4. **ZOOM light hit-test fails on zoomed surface** — `frame_light_at()` uses `get_surface_bounds()` which must reflect current geometry. STOP if zoom geometry is not propagated to bounds storage.
5. **Multiple frames conflict on normal_* storage** — each ShellFrame has its own `normal_*`. STOP only if multi-frame zoom is attempted before implementation supports it.

---

## Next Implementation Phase

### FRAME_ZOOM_ACTION_V1

```
MISSION: FRAME_ZOOM_ACTION_V1

IMPLEMENTATION ONLY. Design complete in FRAME_ZOOM_MODEL_PLAN_V1.md.

Files to modify:
- servers/silk-shell/src/main.rs

Changes:
1. Add FRAME_FLAG_ZOOMED = 1 << 1 constant
2. Add normal_x, normal_y, normal_w, normal_h fields to ShellFrame
3. Initialize normal_* to 0 in FRAMES[0] init (line 1345)
4. Add update_local_geometry() helper to sync shell statics after 0xEC
5. Add zoom_frame() helper (save bounds, set flag, 0xEC with layout_maximize())
6. Add unzoom_frame() helper (clear flag, 0xEC with normal_* bounds)
7. Modify click_hit_test_and_focus() ZOOM light arm (currently no-op capture)
   → call zoom_toggle_frame() (zoom if not zoomed, unzoom if zoomed)
8. Budgeted [shell.frame.zoom] and [shell.frame.unzoom] markers
9. Verify restore_minimized_frame() checks FRAME_FLAG_ZOOMED for restore geometry

Forbidden:
- Any ABI/opcode change
- Any sexdisplay change
- Any silkbar/silkbar-model change
- Any framebuffer path change
- Any renderer change
- Close or minimize behavior change
- Changes to keyboard Maximize (0x32) behavior (V1 keeps both)

PASS:
- Default build passes
- Synthetic build passes
- ZOOM light click maximizes frame-owned surface (0xEC with layout_maximize())
- ZOOM light click on already-zoomed surface unzooms to saved normal geometry
- Normal geometry preserved across zoom/unzoom cycle
- CLOSE light still works
- MINIMIZE light still works (document interaction)
- Keyboard PageUp restore works (may restore to zoomed geometry if flag set)
- No panic/#PF/#GP
- Only silk-shell plus handoff doc changed
```
