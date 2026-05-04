# FRAME_TOP_BAR_TOGGLE_PLAN_V1

## Status

Design (2026-05-04). Keyboard-driven toggle between default top bar mode and minimal 4px rim mode. Docs-only — no code changed.

---

## Verdict: TOP_BAR_TOGGLE_SAFE_NOW ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Toggle active frame between top bar and minimal rim | ✅ | `set_frame_top_bar()` + `send_frame_tab_info()` via 0xFD |
| No new IPC/opcode | ✅ | Existing 0xFD arg2 bit 8 already carries chrome_flags |
| No sexdisplay changes | ✅ | 0xFD handler already stores and renders chrome_flags |
| No kernel/ABI changes | ✅ | All userland |
| Toggle reversible | ✅ | Flip bit, resend 0xFD |
| Toggle does not destroy/hide surface | ✅ | Only flag and rendering change; surface geometry/focus unchanged |
| Minimal mode renders correctly | ✅ | Already tested in FRAME_TOP_BAR_RENDER_V1 (minimal path preserved) |
| Build passes | ✅ | Add one variant + match arm + helper call |

---

## Toggle Scope: Active Frame Only

**Verdict: Active frame (per-frame toggle), not global default.**

The boot default remains `FRAME_FLAG_TOP_BAR` ON. The user toggles the active/focused frame's chrome mode. Other frames are unaffected.

Rationale:
- Per-frame toggle lets users have some frames in top bar mode, some in minimal mode
- Future "toggle all" or "change default" can be added via settings app
- Simpler implementation — no need to iterate all frames
- `selected_frame_id()` already resolves the frame for the focused surface

Future: A `FRAME_FLAG_TOP_BAR_DEFAULT` global static could control the default for NEW frames, but V1 has only one frame at boot, so this is deferred.

---

## Proposed Action: `ToggleTopBar`

### SurfaceAction variant

```rust
enum SurfaceAction {
    // ... existing variants ...
    RestoreMinimized,
    // NEW:
    ToggleTopBar,
    // ... remaining variants ...
}
```

### Keyboard mapping

**Recommended: F4 (scancode 0x3E)**

| Key | Scancode | Rationale |
|-----|----------|-----------|
| **F4** | `0x3E` | Follows existing F-key pattern (F2=Destroy, F3=Recreate). F4 is conventionally "toggle chrome/fullscreen" in many desktop environments. Adjacent to existing used F-keys. |
| (alt) F11 | `0x57` | Common for fullscreen/chrome toggle in browsers. Further from other F-keys. |

**Recommendation: F4 (0x3E)** — fits the F-key cluster pattern already established.

Add to `scancode_to_action()`:

```rust
0x3E => Some(SurfaceAction::ToggleTopBar),  // F4
```

---

## Toggle Algorithm

### `toggle_top_bar_for_active_frame() -> bool`

```rust
/// Toggle the top bar flag on the frame containing the currently focused surface.
/// On success, updates ShellFrame.flags and notifies sexdisplay via 0xFD.
/// Returns true if the toggle was applied.
unsafe fn toggle_top_bar_for_active_frame() -> bool {
    // 1. Resolve the frame that owns the focused surface.
    let frame_id = match selected_frame_id() {
        Some(fid) => fid,
        None => return false,
    };

    // 2. Toggle the flag.
    let new_state = !frame_has_top_bar(frame_id);
    set_frame_top_bar(frame_id, new_state);

    // 3. Push updated chrome_flags to sexdisplay via 0xFD.
    send_frame_tab_info(frame_id);

    // 4. Emit diagnostic marker.
    unsafe {
        static mut TOP_BAR_TOGGLE_BUDGET: u32 = 8;
        let b = &mut TOP_BAR_TOGGLE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.topbar.toggle] frame={} enabled={}",
                frame_id, new_state as u32);
        }
    }
    true
}
```

### Keyboard dispatch integration

```rust
SurfaceAction::ToggleTopBar => {
    if toggle_top_bar_for_active_frame() {
        mutated = true;
    }
}
```

### What happens after toggle

| Step | Effect |
|------|--------|
| `set_frame_top_bar()` | Clears/sets `FRAME_FLAG_TOP_BAR` bit in ShellFrame.flags |
| `send_frame_tab_info()` | Packs new `chrome_flags` into 0xFD arg2 bit 8 and sends to sexdisplay |
| Sexdisplay 0xFD handler | Updates `Surface.chrome_flags`, triggers `redraw_surface_area()` |
| composite_pixel() Pass 2 | Reads updated `chrome_flags`: if 0 → renders minimal 4px rim path; if 1 → renders 16px top bar |
| Hit targets update | `frame_light_at()`/`frame_tab_at()`/`hit_test_surface_chrome()` all read `frame_has_top_bar()` which reflects the flag change immediately |

### No surface geometry changes needed

The toggle does not:
- Move, resize, hide, or destroy any surface
- Change focus
- Change tab state
- Change drag state
- Change minimize/zoom flags

The only change is which rendering path sexdisplay uses for the focused surface's chrome. Hit targets update automatically because they check `frame_has_top_bar()` on every call.

---

## Interaction with Other Frame States

### Minimized frames

If the active frame is minimized, `selected_frame_id()` still returns its frame_id. Toggling the flag and sending 0xFD is safe — sexdisplay will update the chrome_flags on the minimized frame's surface slot, but the frame is not visible (0xEE'd). When restored, the new chrome mode will be active.

**Recommendation:** Allow toggle even when frame is minimized. The flag change applies silently. No special handling needed.

### Zoomed frames

If the active frame is zoomed, toggling top bar mode changes the visible chrome height (16px vs 4px) but does not affect the surface geometry or zoom state. The surface content area shifts by 12px (less content area in top bar mode). This is identical to the behavior when dragging between modes — acceptable for V1.

**Recommendation:** Allow toggle when zoomed. The chrome height change is a visual only; zoom state is preserved.

### Tabbed frames (multi-tab)

Toggling top bar mode on a frame with multiple tabs affects all tabs within that frame (the chrome mode is per-frame, not per-tab). Tab switching in minimal mode uses the 4px rim tab strip. Tab switching in top bar mode uses the wider 40px exclusion. Both paths work.

**Recommendation:** No special handling. The `frame_tab_at()` function already dispatches on `frame_has_top_bar()`.

### Non-frame focused surfaces

If the focused surface is not owned by any frame (e.g., linen surface 200), `selected_frame_id()` returns `None`. The toggle action does nothing and emits nothing.

**Recommendation:** No-op when no frame is selected. This is correct — OS-owned surfaces have no chrome to toggle.

---

## Toggle Validation Invariants

| Invariant | Enforcement |
|-----------|-------------|
| Toggle only affects the active frame | `selected_frame_id()` resolves frame from `FOCUSED_SURFACE_ID` |
| Toggle does not destroy surfaces | Only flag bit changes, no 0xEE/0xEC calls |
| Toggle does not change focus | No `try_set_focus()` call |
| Toggle does not change drag state | No `clear_drag_if_dead()` call |
| Toggle reversible | Bit is XOR'd; calling again restores previous state |
| Sexdisplay notified immediately | `send_frame_tab_info()` called right after `set_frame_top_bar()` |
| Hit targets update immediately | All hit functions read `frame_has_top_bar()` live |
| Renderer updates immediately | 0xFD handler calls `redraw_surface_area()` |

---

## Protocol Update Path

The entire toggle flows through existing infrastructure:

```
User presses F4
  → scancode 0x3E → SurfaceAction::ToggleTopBar
    → toggle_top_bar_for_active_frame()
      → selected_frame_id() → Option<u32>
      → set_frame_top_bar(frame_id, !current)
      → send_frame_tab_info(frame_id)
        → pdx_call(SLOT_DISPLAY, 0xFD, surface_id, tab_count, (active_tab | chrome_flags << 8))
          → Sexdisplay 0xFD handler
            → slot.chrome_flags = chrome_flags_raw
            → redraw_surface_area()
              → composite_pixel() uses updated chrome_flags
```

**No new IPC. No sexdisplay changes. No protocol changes.**

---

## Diagnostic Markers

### New marker

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.topbar.toggle] frame=N enabled=N` | 8 | Successful toggle of top bar mode on a frame |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.tab.info.send]` | Called after toggle (sends updated chrome_flags) ✅ |
| `[sexdisplay.surface.tab.info]` | Sexdisplay receives updated chrome_flags ✅ |
| `[shell.frame.topbar.model]` | Boot proof (still fires) ✅ |
| `[shell.frame.light.model]` | Unchanged ✅ |
| `[shell.frame.tab.model]` | Unchanged ✅ |
| `[shell.frame.tab.switch]` | Unchanged ✅ |
| `[shell.frame.light.close/minimize/zoom]` | Unchanged ✅ |
| `[shell.drag.start/move/end]` | Unchanged ✅ |

---

## Implementation Files

### Modified: `servers/silk-shell/src/main.rs`

| Change | Details |
|--------|---------|
| Add `ToggleTopBar` variant to `SurfaceAction` enum | After `RestoreMinimized` |
| Add scancode `0x3E => Some(SurfaceAction::ToggleTopBar)` to `scancode_to_action()` | F4 key |
| Add `toggle_top_bar_for_active_frame()` helper | Resolves frame, toggles flag, sends 0xFD, emits marker |
| Add dispatch arm in keyboard handler | `SurfaceAction::ToggleTopBar => { ... }` |
| Add `[shell.frame.topbar.toggle]` marker constant | Budget 8 |

### NOT Modified

- `servers/sexdisplay/src/main.rs` — no renderer changes needed
- `kernel/` — no ABI changes
- `crates/sex-pdx/src/lib.rs` — no opcode changes
- `servers/silkbar/` — no forwarding changes
- `crates/silkbar-model/` — no model changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

---

## Forbidden in FRAME_TOP_BAR_TOGGLE_V1

- Sexdisplay changes
- Sex-pdx changes
- Kernel edits
- Settings app
- Text rendering
- Broad input refactor
- Global default toggle
- Persistent settings storage
- Theme/chrome settings

---

## STOP Conditions

1. **`selected_frame_id()` returns None for frame-owned surfaces** — If the focused surface is not in any frame (e.g., standalone surface 200 linen), the toggle is a no-op. This is correct behavior — only frame-owned surfaces have chrome to toggle. Verify: `selected_frame_id()` uses `frame_for_surface()` which iterates all frames' tabs.

2. **`send_frame_tab_info()` fails if surface is dead** — The function calls `active_surface_for_frame()` which reads `frame.tabs[frame.active_tab].surface_id`. If the active tab's surface is dead, this returns `Some(sid)` (the surface_id is still valid, even if the surface was 0xEE'd). The 0xFD call with a hidden surface updates sexdisplay's Surface metadata without visual effect. When the surface is restored (0xEC), the new chrome mode will be active. This is correct.

3. **Toggle during active drag** — Toggling top bar mode changes hit-target geometry for the frame. If the user is currently dragging the frame rim, the drag anchor points might shift (4px vs 16px top chrome). Mitigation: the drag uses absolute pointer deltas, not chrome-relative positions. The drag visual (surface movement) is unaffected. However, if the user drags the top edge and toggles simultaneously, the drag zone height changes under the cursor. This is an edge case that is acceptable for V1.

4. **Toggle while minimized** — Changing chrome_flags on a minimized surface is deferred until `restore_minimized_frame()` calls 0xEC/0xED. The updated chrome_flags are stored in sexdisplay's Surface metadata from the 0xFD call. When restored, the correct mode renders. No special handling needed.

5. **No visual feedback that toggle occurred** — V1 keyboard toggle has no on-screen indication (no toast, no animation). The user sees the chrome change (or press F4 again to toggle back). Future: settings app or indicator.

---

## Next Phase

### FRAME_TOP_BAR_TOGGLE_V1

```
MISSION: FRAME_TOP_BAR_TOGGLE_V1.

Implement keyboard toggle for top bar mode on active frame.
F4 key toggles between default top bar and minimal 4px rim.
Shell-only change — no sexdisplay, no protocol, no renderer changes.

Design complete in FRAME_TOP_BAR_TOGGLE_PLAN_V1.md.

Changes:

1. servers/silk-shell/src/main.rs:
   a. Add ToggleTopBar variant to SurfaceAction enum
   b. Add 0x3E => Some(SurfaceAction::ToggleTopBar) to scancode_to_action()
   c. Add toggle_top_bar_for_active_frame() helper:
      - selected_frame_id() → resolve frame
      - set_frame_top_bar(frame_id, !frame_has_top_bar(frame_id))
      - send_frame_tab_info(frame_id)
      - emit [shell.frame.topbar.toggle] (budget 8)
   d. Add dispatch arm in keyboard handler match block
   e. Add marker constant for [shell.frame.topbar.toggle]

Forbidden:
- Sexdisplay changes
- Sex-pdx changes
- Kernel edits
- Settings app
- Text rendering
- Broad input refactor

PASS:
- Default build passes
- F4 toggles top bar ON → OFF → ON on the active frame
- [shell.frame.topbar.toggle] fires on each toggle with correct enabled=N
- [shell.frame.tab.info.send] fires after each toggle with updated chrome=N
- [sexdisplay.surface.tab.info] fires with updated chrome=N
- Minimal mode renders 4px rim (visual check)
- Top bar mode renders 16px rim (visual check)
- Lights/tabs/close/minimize/zoom still work in both modes
- Tab switching still works in both modes
- Rim drag still works
- No panic/#PF/#GP
- No sexdisplay/kernel/sex-pdx changes
```
