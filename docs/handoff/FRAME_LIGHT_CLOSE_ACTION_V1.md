# FRAME_LIGHT_CLOSE_ACTION_V1

## Status

Implemented (2026-05-04). CLOSE light click destroys the active frame surface using the same safe path as keyboard `SurfaceAction::DestroyFocused`. No ABI/protocol changes. MINIMIZE and ZOOM lights remain no-op (captured, no drag).

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| CLOSE light destroys active surface | `frame_light_at()` → `close_surface_from_frame_light()` | `[shell.frame.light.close]` |
| CLOSE light does not start rim drag | Light check before drag start branch | No `[shell.frame.rim.drag.start]` for close clicks |
| CLOSE rejected for non-closeable surfaces | `is_closeable_surface()` rejects linen, cursor, panels | `[shell.frame.light.close.reject]` |
| Focus falls back after close | `close_surface_from_frame_light()` calls `clear_focus_if_dead()` | Existing focus markers |
| MINIMIZE/ZOOM lights still no-op | Captured in `else if light != FRAME_LIGHT_NONE` branch | `[shell.frame.chrome.capture]` |
| Rim drag still works on non-light rim clicks | `else` branch preserved unchanged | `[shell.frame.rim.drag.start]` |
| No ABI/protocol changes | Reuses existing `0xEE` opcode, no new PDX calls | No new opcodes |

---

## Changes

### File: `servers/silk-shell/src/main.rs`

#### 1. `is_closeable_surface()` helper (after `selected_window_options_mask()`, ~line 566)

```rust
unsafe fn is_closeable_surface(surface_id: u64) -> bool
```

Returns false for OS-owned surfaces (linen, cursor, launcher, status, clock, bell). Otherwise delegates to `surface_is_alive()`.

#### 2. `close_surface_from_frame_light()` helper (~line 578)

```rust
unsafe fn close_surface_from_frame_light(surface_id: u64) -> bool
```

1. Checks `surface_is_alive()` guard
2. Sets `SURFACE_*_ALIVE = false` for the matched surface
3. Calls `pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0)` to notify sexdisplay
4. Calls `clear_focus_if_dead()` for automatic focus fallback
5. Returns true if surface was destroyed

Reuses the same destroy mechanism as keyboard `SurfaceAction::DestroyFocused` (lines 1372-1416) but is a clean reusable helper.

#### 3. `click_hit_test_and_focus()` FrameChrome arm modified (~line 1026)

Restructured to check `frame_light_at()` before rim drag:

| Condition | Behavior |
|-----------|----------|
| `light == FRAME_LIGHT_CLOSE` | Close action via helper |
| `light != FRAME_LIGHT_NONE` (MINIMIZE/ZOOM) | Capture/no-op (no drag) |
| `light == FRAME_LIGHT_NONE` | Rim drag (existing behavior) |
| Non-rim chrome (tab strip) | Capture/no-op (existing) |

### File Changes

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | +2 helper functions (~35 lines), +1 restructured match arm (~45 lines) |

### Files NOT Modified

All other files untouched — kernel, PDX ABI, sexdisplay, silkbar, silkbar-model, sexusb, sexinput.

---

### Diagnostic Markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.light.close] frame=N surface=N` | 8 | CLOSE light click destroys surface |
| `[shell.frame.light.close.reject] frame=N surface=N reason=...` | unbudgeted | CLOSE click rejected (not_closeable, no_active_surface, failed) |
| `[shell.frame.chrome.capture] frame=N kind=N x=N y=N` | 4 | MINIMIZE/ZOOM light clicks (no-op capture) |
| `[shell.frame.rim.drag.start] frame=N surface=N x=N y=N` | 8 | Rim drag on non-light rim clicks (existing) |
| `[shell.frame.light.hover] frame=N light=N` | 8 | Light hover detection (pre-existing) |

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass. No new warnings.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-light-close-action-v1.log

for m in \
  shell.frame.light.close \
  shell.frame.light.close.reject \
  shell.frame.chrome.capture \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.selected.options.send
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-light-close-action-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-light-close-action-v1.log
```

### Expected counts

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.frame.light.close` | ≥0 (depends on test) | CLOSE light click destroys surface |
| `shell.frame.light.close.reject` | ≥0 | Rejected close attempts logged |
| `shell.frame.chrome.capture` | ≥0 | MINIMIZE/ZOOM lights captured as no-op |
| `shell.drag.start/move/end` | ≥1 | Rim drag still works on non-light clicks |
| faults | 0 | Memory safety |

### Pass criteria

- Default build passes
- Synthetic build passes
- CLOSE light click destroys focused frame-owned surface
- Non-closeable surfaces (linen, cursor) rejected with reason
- Rim drag still starts on non-light rim clicks
- MINIMIZE/ZOOM light clicks captured as no-op (no drag)
- Focus falls back to next alive surface after close
- No panic/#PF/#GP

---

## Remaining Risks

- **MINIMIZE/ZOOM still no-op**: Both lights are captured without action. Clicking yellow or green does nothing visible.
- **Single-frame V1**: Only frame 1 (surface 100) exists. Close behavior for multi-frame tabs not tested.
- **No undo/restore**: Closed surface cannot be restored. The `SURFACE_*_ALIVE` flag is one-way (no undelete).
- **Frame model not updated**: `ShellFrame.tabs[]` still references the closed surface's ID. `surface_is_alive()` guards against use, but the frame model itself is stale. Future phases should clean up tabs when a surface is closed.

---

## Next Recommended Phase

### FRAME_MINIMIZE_MODEL_PLAN_V1

Design the model and IPC for minimizing/collapsing a frame. Required before MINIMIZE light can have behavior. Includes:
- Frame-level "minimized" flag
- Hide/reveal IPC (or minimize state in sexdisplay)
- Tab bar or workspace shelf for minimized frames
- Z-order management
- No timer/clock/taskbar until a later phase
