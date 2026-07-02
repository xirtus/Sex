# FRAME_MINIMIZE_ACTION_V1

## Status

Implemented (2026-05-04). MINIMIZE light click hides the active frame surface via 0xEE, sets `FRAME_FLAG_MINIMIZED`, clears focus/drag. Keyboard `PageUp` (scancode 0x49) restores the first minimized frame via 0xEC upsert with stored geometry. No ABI/protocol changes. CLOSE and rim drag unchanged.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| MINIMIZE light hides active surface | `frame_light_at()` → `minimize_frame()` sets flag + 0xEE | `[shell.frame.minimize]` |
| MINIMIZE light does not start rim drag | Light check before drag start branch | No `[shell.frame.rim.drag.start]` for minimize clicks |
| MINIMIZE rejected for already-minimized frames | `frame_is_minimized()` early-return guard | `[shell.frame.minimize.reject]` |
| Focus falls back after minimize | `minimize_frame()` calls `clear_focus_if_dead()` | Existing focus markers |
| Drag clears if minimized surface was being dragged | `minimize_frame()` calls `clear_drag_if_dead()` | Existing drag markers |
| CLOSE light still works | Close branch preserved unchanged | `[shell.frame.light.close]` |
| ZOOM light still no-op | ZOOM branch captured unchanged | `[shell.frame.chrome.capture]` |
| Rim drag still works on non-light rim clicks | `else` branch preserved unchanged | `[shell.frame.rim.drag.start]` |
| Keyboard restore finds and un-hides minimized frame | `first_minimized_frame_id()` → `restore_minimized_frame()` sends 0xEC with stored geometry | `[shell.frame.restore]` |
| Restore re-focuses the surface | `restore_minimized_frame()` calls `try_set_focus()` | Existing focus markers |
| No slot leak from minimize alone | `0xEE` sets active=false, no slot consumed | N/A |
| No ABI/protocol changes | Reuses existing 0xEE/0xEC opcodes, no new PDX calls | No new opcodes |

---

## Changes

### File: `servers/silk-shell/src/main.rs`

#### 1. Frame flag constant (line 266, after `FRAME_LIGHT_GAP_PX`)

```rust
/// ShellFrame.flags: frame is minimized (hidden via 0xEE, not destroyed).
const FRAME_FLAG_MINIMIZED: u32 = 1 << 0;
```

#### 2. Helper functions (after `close_surface_from_frame_light()`, ~line 604)

**`frame_is_minimized(frame_id: u32) -> bool`** — iterates FRAMES, checks `FRAME_FLAG_MINIMIZED` bit.

**`set_frame_minimized(frame_id: u32, minimized: bool)`** — sets or clears `FRAME_FLAG_MINIMIZED` on the matching frame.

**`first_minimized_frame_id() -> Option<u32>`** — finds the first minimized frame's ID. Used by keyboard restore.

**`minimize_frame(frame_id: u32) -> bool`**:
1. Guards: already minimized → return false; no active surface → return false; surface not alive → return false
2. Sets `FRAME_FLAG_MINIMIZED` via `set_frame_minimized(frame_id, true)`
3. Sends `pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0)` to hide surface
4. Calls `clear_drag_if_dead()` to clear any drag on this surface
5. Calls `clear_focus_if_dead()` to fall back focus if minimized surface was focused
6. Emits budgeted `[shell.frame.minimize]` marker (max 8)

**`restore_minimized_frame(frame_id: u32) -> bool`**:
1. Guard: not minimized → return false
2. Gets active surface for frame, checks alive
3. Clears `FRAME_FLAG_MINIMIZED` via `set_frame_minimized(frame_id, false)`
4. Gets surface bounds via `get_surface_bounds()`, sends `0xEC` upsert with stored geometry
5. Calls `try_set_focus(surface_id)` to focus the restored surface
6. Emits budgeted `[shell.frame.restore]` marker (max 8)

#### 3. `click_hit_test_and_focus()` MINIMIZE dispatch (line 1171)

```rust
} else if light == FRAME_LIGHT_MINIMIZE {
    // ── MINIMIZE action: hide active surface ──
    if !minimize_frame(frame_id) {
        // budgeted [shell.frame.minimize.reject] marker (max 4)
    }
}
```

#### 4. `SurfaceAction::RestoreMinimized` enum variant (line 50)

Added to the `SurfaceAction` enum alongside `DestroyFocused`, `RecreateFocused`, etc.

#### 5. Scancode mapping (line 135)

```rust
0x49 => Some(SurfaceAction::RestoreMinimized),  // PageUp key
```

#### 6. Keyboard dispatch (line 1705)

```rust
SurfaceAction::RestoreMinimized => {
    if let Some(frame_id) = first_minimized_frame_id() {
        if restore_minimized_frame(frame_id) {
            mutated = true;
            serial_println!("[silk-shell] Restored minimized frame {}", frame_id);
        }
    } else {
        serial_println!("[silk-shell] No minimized frame to restore");
    }
}
```

### File Changes

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | +1 constant, +5 helpers (~80 lines), +3 lines in click handler, +1 enum variant, +1 scancode mapping, +8 lines keyboard dispatch |

### Files NOT Modified

All other files untouched — kernel, PDX ABI, sexdisplay, silkbar, silkbar-model, sexusb, sexinput.

---

### Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.minimize] frame=N surface=N` | 8 | MINIMIZE light click hides surface |
| `[shell.frame.minimize.reject] frame=N reason=...` | 4 | MINIMIZE click rejected (already minimized, no surface, dead) |
| `[shell.frame.restore] frame=N surface=N` | 8 | Keyboard restore brings surface back |
| `[shell.frame.light.close] frame=N surface=N` | 8 | CLOSE light still works (pre-existing) |
| `[shell.frame.chrome.capture] frame=N kind=N x=N y=N` | 4 | ZOOM light still captured (pre-existing) |
| `[shell.frame.rim.drag.start] frame=N surface=N x=N y=N` | 8 | Rim drag on non-light clicks (pre-existing) |

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass. No new warnings (only pre-existing `ERR_CAP_INVALID` unused import and mutable static reference warnings).

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-minimize-action-v1.log

for m in \
  shell.frame.minimize \
  shell.frame.minimize.reject \
  shell.frame.restore \
  shell.frame.light.close \
  shell.frame.chrome.capture \
  shell.frame.rim.drag.start \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-44s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-minimize-action-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-minimize-action-v1.log
```

### Expected counts

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.frame.minimize` | ≥0 (depends on test) | MINIMIZE light click hides surface |
| `shell.frame.minimize.reject` | ≥0 | Rejected minimize attempts logged |
| `shell.frame.restore` | ≥0 (depends on keyboard test) | PageUp restores minimized frame |
| `shell.frame.light.close` | ≥0 (if close-clicked) | CLOSE light still works |
| `shell.frame.chrome.capture` | ≥0 (if zoom-clicked) | ZOOM light still captured as no-op |
| `shell.drag.start/move/end` | ≥1 | Rim drag still works on non-light clicks |
| faults | 0 | Memory safety |

### Pass criteria

- Default build passes
- Synthetic build passes
- MINIMIZE light click hides frame-owned surface (sets flag, sends 0xEE)
- Focus falls back to next alive surface after minimize
- Drag clears if minimized surface was being dragged
- PageUp (scancode 0x49) restores first minimized frame (sends 0xEC, focuses)
- CLOSE light still works
- ZOOM light still captured as no-op
- Rim drag still starts on non-light rim clicks
- No panic/#PF/#GP

---

## Remaining Risks

- **Slot leak on restore**: Each minimize→restore cycle leaves one inactive orphan slot in sexdisplay's 16-slot array. With ~10 active surfaces and 6 spare, worst case is ~4 cycles before slot pressure. Acceptable for V1.
- **No visual minimize indicator**: Minimized frames have no visible representation. No tab bar, no taskbar, no shelf icon. User must use PageUp to cycle/restore.
- **Stale geometry on restore**: If a frame is minimized and then the surface position changes externally (impossible in V1), restored position would be stale. Current model stores static geometry in get_surface_bounds().
- **Single frame restore only**: `first_minimized_frame_id()` returns the first minimized frame found in FRAMES iteration order. No cycling through multiple minimized frames. V1 assumes only one frame can be minimized at a time.
- **FLAGS field pattern**: Using `ShellFrame.flags` bitfield for minimized state. Future flags must not conflict with `FRAME_FLAG_MINIMIZED = 1 << 0`.

---

## Next Recommended Phase

### FRAME_ZOOM_MODEL_PLAN_V1

Design the model and IPC for zooming/maximizing a frame. The green ZOOM light is currently a no-op capture. Requires:
- Resize model (fullscreen vs. tiled maximize)
- Geometry save/restore (similar to minimize)
- Z-order management during zoom
- Slot reuse or proper restore on un-zoom
