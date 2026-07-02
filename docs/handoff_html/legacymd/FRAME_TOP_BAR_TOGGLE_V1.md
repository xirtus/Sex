# FRAME_TOP_BAR_TOGGLE_V1

## Status

Implemented (2026-05-04). F4 keyboard toggle for top bar mode on the active frame. Shell-only — no sexdisplay, protocol, or renderer changes.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| F4 toggles top bar on active frame | `scancode 0x3E → SurfaceAction::ToggleTopBar` | `[shell.frame.topbar.toggle]` |
| Active frame resolved safely | `selected_frame_id()` returns `Some(frame_id)` for focused frame-owned surface | `[shell.frame.topbar.toggle.reject]` on failure |
| No-op when no frame focused | `selected_frame_id()` returns `None` → reject marker fires, returns false | `[shell.frame.topbar.toggle.reject] reason=no_active_frame` |
| Flag flipped via existing helper | `set_frame_topbar(frame_id, !frame_has_top_bar(frame_id))` | Build passes |
| Sexdisplay notified immediately | `send_frame_tab_info(frame_id)` after flag change | `[shell.frame.tab.info.send] chrome=N` |
| Toggle reversible | Calling again restores previous state | Bit XOR |
| No surface geometry changed | Only flag bit changes, no 0xEE/0xEC calls | Compile-time proof |
| No focus change | No `try_set_focus()` call | Compile-time proof |
| No drag state change | No `clear_drag_if_dead()` call | Compile-time proof |
| No surface destroyed | No 0xEE call | Compile-time proof |
| No kernel/ABI/sex-pdx changes | Userland only | Build passes |
| Build passes | Default + synthetic | `entrypoint_build.sh` |

---

## Files Changed

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Added `ToggleTopBar` variant to `SurfaceAction`. Added `0x3E => Some(SurfaceAction::ToggleTopBar)` to `scancode_to_action()`. Added `toggle_top_bar_for_active_frame()` helper. Added dispatch arm in keyboard handler. Added `[shell.frame.topbar.toggle]` (budget 8) and `[shell.frame.topbar.toggle.reject]` (budget 4) markers. |

### Files NOT Modified

Kernel, sexdisplay, sex-pdx, silkbar, silkbar-model, sexusb, sexinput — all untouched.

---

## Keyboard Shortcut

| Key | Scancode | Action |
|-----|----------|--------|
| **F4** | `0x3E` | Toggle top bar mode on active frame |

---

## Helper: `toggle_top_bar_for_active_frame() -> bool`

```rust
unsafe fn toggle_top_bar_for_active_frame() -> bool {
    let frame_id = match selected_frame_id() {
        Some(fid) => fid,
        None => {
            // emit [shell.frame.topbar.toggle.reject] reason=no_active_frame
            return false;
        }
    };

    let new_state = !frame_has_top_bar(frame_id);
    set_frame_top_bar(frame_id, new_state);
    send_frame_tab_info(frame_id);

    // emit [shell.frame.topbar.toggle] frame=N enabled=N (budget 8)
    true
}
```

### Behavior

1. **Resolve active frame** via `selected_frame_id()` (reads `FOCUSED_SURFACE_ID`, maps through `frame_for_surface()`)
2. **If no frame** (e.g., linen surface 200 is focused): emit reject marker, return false
3. **Flip flag**: `!frame_has_top_bar(frame_id)` → `set_frame_top_bar(frame_id, new_state)`
4. **Notify sexdisplay**: `send_frame_tab_info(frame_id)` packs updated `chrome_flags` in 0xFD arg2 bit 8
5. **Emit marker**: `[shell.frame.topbar.toggle] frame=N enabled=N` (budget 8)

### Preserved Invariants

- Active tab unchanged
- Focus surface unchanged
- Zoom/minimize state unchanged
- Surface geometry unchanged
- No 0xEE/0xEC calls
- No drag state altered
- No sexdisplay renderer changes

---

## Diagnostic Markers

### New markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.topbar.toggle] frame=N enabled=N` | 8 | Every successful toggle |
| `[shell.frame.topbar.toggle.reject] reason=N` | 4 | When no active frame is selected |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.tab.info.send] chrome=N` | Called after every toggle ✅ |
| `[shell.frame.topbar.model]` | Boot proof ✅ |
| `[shell.frame.light.model]` | Unchanged ✅ |
| `[shell.frame.tab.model]` | Unchanged ✅ |
| `[shell.frame.light.close/minimize/zoom]` | Unchanged ✅ |
| `[shell.frame.tab.switch]` | Unchanged ✅ |
| `[shell.drag.start/move/end]` | Unchanged ✅ |
| `[shell.frame.minimize/restore]` | Unchanged ✅ |
| `[shell.frame.zoom/unzoom]` | Unchanged ✅ |

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Default build passes. Synthetic build passes. No new warning types.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-topbar-toggle-v1.log

for m in \
  shell.frame.topbar.toggle \
  shell.frame.topbar.toggle.reject \
  shell.frame.tab.info.send \
  shell.frame.topbar.model
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-topbar-toggle-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-topbar-toggle-v1.log
```

### Pass criteria

- Default build passes ✅
- Synthetic build passes ✅
- F4 toggles top bar ON → OFF → ON on the active frame ✅
- `[shell.frame.topbar.toggle]` fires on each toggle with correct `enabled=N` ✅
- `[shell.frame.tab.info.send]` fires after each toggle with updated `chrome=N` ✅
- No sexdisplay/protocol/kernel changes ✅
- Only silk-shell + handoff doc changed ✅
- Existing close/minimize/zoom/tab switching unchanged ✅
- No panic/#PF/#GP ✅

---

## Toggle Interaction Notes

| Scenario | Behavior |
|----------|----------|
| Active frame has top bar ON | F4 → turns top bar OFF → `[shell.frame.topbar.toggle] frame=1 enabled=0` |
| Active frame has top bar OFF | F4 → turns top bar ON → `[shell.frame.topbar.toggle] frame=1 enabled=1` |
| No frame (linen focused) | F4 → no-op → `[shell.frame.topbar.toggle.reject] reason=no_active_frame` |
| Frame minimized | Toggle changes flag + sends 0xFD; sexdisplay updates metadata silently. When restored, new chrome mode active. |
| Frame zoomed | Toggle changes chrome height (16px vs 4px); zoom state preserved. |
| Multi-tab frame | Toggle affects all tabs (per-frame flag). Tab switching works in both modes. |
| Active drag | Toggle changes hit-target height; drag uses absolute deltas, not chrome-relative positions. Edge case acceptable for V1. |

---

## Next Recommended Phase

### SCENE_APPEARANCE_CONTROLS_PLAN_V1

Design a settings panel or keyboard-driven appearance controls for chrome mode, rim color, light style, and other visual preferences. See `docs/handoff/SILK_CHROME_SETTINGS_PLAN_V1.md` for roadmap context.
