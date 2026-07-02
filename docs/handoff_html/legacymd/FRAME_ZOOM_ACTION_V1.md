# FRAME_ZOOM_ACTION_V1

## Status

Implemented (2026-05-04). ZOOM light click toggles the active frame surface between normal geometry and maximized (full area below SilkBar). No ABI/protocol changes. CLOSE, MINIMIZE, and rim drag unchanged.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| ZOOM light maximizes surface | `toggle_zoom_frame()` → `zoom_frame()` saves normal geometry, sends 0xEC with `layout_maximize()` | `[shell.frame.zoom]` |
| ZOOM light unzooms to saved normal geometry | `toggle_zoom_frame()` → `unzoom_frame()` sends 0xEC with `ShellFrame.normal_*` | `[shell.frame.unzoom]` |
| ZOOM light does not start rim drag | Light check before drag start branch | No `[shell.frame.rim.drag.start]` for zoom clicks |
| Normal geometry preserved across zoom/unzoom cycle | Stored in `ShellFrame.normal_{x, y, w, h}` | Correct geometry on unzoom |
| ZOOM rejected for minimized frames | `zoom_frame()` checks `frame_is_minimized()` | `[shell.frame.zoom.reject]` |
| Focus preserved after zoom/unzoom | Neither `zoom_frame()` nor `unzoom_frame()` changes focus | Existing focus markers |
| restore_minimized_frame checks zoom state | If `FRAME_FLAG_ZOOMED` set, restore to maximized geometry | Restore marker |
| Pre-existing stale-dimension bug fixed | `get_surface_bounds()` and `point_in_surface()` now read `WINDOWS[1].desc.{width, height}` instead of stale `SURFACE_100_W/H` | N/A |
| Local geometry stays in sync with sexdisplay | `update_local_geometry()` called after every 0xEC update | N/A |

---

## Changes

### File: `servers/silk-shell/src/main.rs`

#### 1. Frame flag constant (line 268, after `FRAME_FLAG_MINIMIZED`)

```rust
/// ShellFrame.flags: frame is zoomed/maximized (fills content area below SilkBar).
const FRAME_FLAG_ZOOMED: u32 = 1 << 1;
```

#### 2. ShellFrame fields (after `flags`)

```rust
/// Saved normal (pre-zoom) geometry. Valid when FRAME_FLAG_ZOOMED is set.
normal_x: i32,
normal_y: i32,
normal_w: u32,
normal_h: u32,
```

#### 3. FRAMES[0] boot initialization

```rust
let boot_x: i32 = 100;
let boot_y: i32 = 100;
let boot_w: u32 = 800;
let boot_h: u32 = 500;
FRAMES[0] = Some(ShellFrame {
    // ...
    flags: 0,
    normal_x: boot_x,
    normal_y: boot_y,
    normal_w: boot_w,
    normal_h: boot_h,
});
```

#### 4. Stale-dimension bug fix

`get_surface_bounds()` and `point_in_surface()` for surface 100 now read `WINDOWS[1].desc.{width, height}` instead of `SURFACE_100_{W, H}` (which were never updated after boot).

#### 5. Helpers added (6 functions, ~170 lines)

- **`frame_is_zoomed(frame_id) -> bool`** — checks `FRAME_FLAG_ZOOMED` in FRAMES
- **`set_frame_zoomed(frame_id, zoomed)`** — sets/clears `FRAME_FLAG_ZOOMED`
- **`update_local_geometry(surface_id, x, y, w, h)`** — syncs shell geometry statics after 0xEC update. Fixes stale-dimension bug by updating both `WINDOWS[1].desc.*` and `SURFACE_10x_*`
- **`zoom_frame(frame_id) -> bool`** — saves normal geometry, sets flag, sends 0xEC with `layout_maximize()`, calls `update_local_geometry()`
- **`unzoom_frame(frame_id) -> bool`** — clears flag, sends 0xEC with `ShellFrame.normal_*`, calls `update_local_geometry()`
- **`toggle_zoom_frame(frame_id) -> bool`** — dispatches to zoom or unzoom based on current state

#### 6. `click_hit_test_and_focus()` ZOOM light dispatch (line 1374)

Before: no-op capture with `[shell.frame.chrome.capture]` marker.
After: calls `toggle_zoom_frame(frame_id)`, emits `[shell.frame.zoom.reject]` on failure.

#### 7. `restore_minimized_frame()` zoom-aware

After clearing minimized flag, checks `FRAME_FLAG_ZOOMED`. If zoomed, uses `layout_maximize()` for restore geometry instead of `get_surface_bounds()`.

### File Changes

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | +1 constant, +4 ShellFrame fields, +6 helpers (~170 lines), +ZOOM dispatch, +restore zoom check, +stale-dimension fix |

### Files NOT Modified

All other files untouched — kernel, PDX ABI, sexdisplay, silkbar, silkbar-model, sexusb, sexinput.

---

### Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.zoom] frame=N surface=N` | 8 | ZOOM light click maximizes surface |
| `[shell.frame.unzoom] frame=N surface=N` | 8 | ZOOM light click on zoomed surface restores normal |
| `[shell.frame.zoom.reject] frame=N reason=...` | 4 | ZOOM click rejected (minimized, no surface, dead, no frame) |

Pre-existing markers still firing:

| Marker | Status |
|--------|--------|
| `[shell.frame.light.close]` | Unchanged |
| `[shell.frame.light.hover]` | Unchanged |
| `[shell.frame.minimize]` | Unchanged |
| `[shell.frame.restore]` | Enhanced (zoom-aware) |
| `[shell.frame.rim.drag.start]` | Unchanged |
| `[shell.drag.start/move/end]` | Unchanged |

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass. No new warning types added (only pre-existing "mutable reference to mutable static" pattern, same as rest of codebase).

---

## Stale-Dimension Bug Fix

`get_surface_bounds()` and `point_in_surface()` for surface 100 previously read `SURFACE_100_W` (initialized to 800 at boot) and `SURFACE_100_H` (500), which were **never updated** after any resize operation (SnapLeft/SnapRight/Maximize/Center/ShrinkWidth/GrowWidth). This caused hit-testing to use stale dimensions while the display rendered the correct size.

Fixed: both functions now read `WINDOWS[1].desc.{width, height}`, which are updated by all resize operations. The `update_local_geometry()` helper also writes to `SURFACE_100_{W, H}` for any code that still reads them directly (e.g., drag/clamp_position).

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-zoom-action-v1.log

for m in \
  shell.frame.zoom \
  shell.frame.unzoom \
  shell.frame.zoom.reject \
  shell.frame.minimize \
  shell.frame.restore \
  shell.frame.light.close \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-zoom-action-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-zoom-action-v1.log
```

### Expected counts

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.frame.zoom` | ≥1 (if zoom clicked) | ZOOM light toggles to maximized |
| `shell.frame.unzoom` | ≥1 (if zoom clicked again) | ZOOM light toggles back to normal |
| `shell.frame.zoom.reject` | ≥0 | Rejected zoom attempts logged |
| `shell.frame.minimize` | ≥0 (if minimize clicked) | MINIMIZE still works |
| `shell.frame.restore` | ≥0 (if PageUp pressed) | Restore works (may restore to zoomed) |
| `shell.frame.light.close` | ≥0 (if close clicked) | CLOSE still works |
| `shell.drag.start/move/end` | ≥1 | Rim drag unchanged |
| faults | 0 | Memory safety |

### Pass criteria

- Default build passes
- Synthetic build passes
- ZOOM light click on non-zoomed surface maximizes (0xEC with layout_maximize())
- ZOOM light click on zoomed surface unzooms (0xEC with saved normal_*)
- Normal geometry preserved across zoom/unzoom cycle
- ZOOM rejected for minimized frames
- CLOSE still works
- MINIMIZE still works
- Keyboard PageUp restore works (zoom-aware)
- Rim drag unchanged
- No panic/#PF/#GP

---

## Remaining Risks

- **Single-frame V1**: Only frame 1 (surface 100) is frame-owned. Zoom on multi-frame setups not tested.
- **Keyboard Maximize (0x32) still one-way**: The existing keyboard Maximize action does not save normal geometry or toggle. Using keyboard Maximize on a zoomed surface will re-maximize to the same geometry (no-op effectively). A future phase should reconcile ZOOM light toggle with keyboard Maximize.
- **Z-order not changed by zoom**: Zoomed surface stays at its current z-order. With only one frame in V1, this is fine. Multi-frame zoom may need z-order promotion.
- **Drag on zoomed surface**: Rim drag on a zoomed surface moves the surface position while keeping maximized size. The surface can be dragged off-screen partially. Unzoom restores to the saved normal position (which may not match the dragged position). This is acceptable for V1.
- **SnapLeft/SnapRight on zoomed surface**: These keyboard actions will resize the surface, effectively unzooming it (but the FRAME_FLAG_ZOOMED flag remains set). Unzoom will restore to the pre-zoom normal geometry, which may be stale. Not addressed in V1.

---

## Next Recommended Phase

### FRAME_TAB_STRIP_PLAN_V1

Design a tab strip model for the top rim band of the focused surface. The tab strip is currently disabled (`FRAME_TAB_STRIP_PX = 0`). Enabling it requires:
- Tab strip geometry (height, position within rim)
- Tab label rendering (text pipeline prerequisite, or icon-only)
- Tab hit-target production (distinct from rim and lights)
- IPC for tab metadata (title, icon, state)
- Tab selection via click (switch active tab)
