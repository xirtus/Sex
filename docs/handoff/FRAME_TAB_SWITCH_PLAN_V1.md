# FRAME_TAB_SWITCH_PLAN_V1

## Status

Design (2026-05-04). Tab switching behavior for Silk Frames. No code changed.

---

## Verdict: TAB_SWITCH_SAFE_NOW ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Tab strip click → switch active_tab | ✅ | Extract tab index via `frame_tab_at()`, update `ShellFrame.active_tab` |
| Old tab surface hidden | ✅ | 0xEE old surface before showing new tab |
| New tab surface shown | ✅ | 0xEC new surface at current frame bounds |
| Focus follows active tab | ✅ | `try_set_focus()` on new surface after switch |
| Sexdisplay tab block highlight updates | ✅ | `send_frame_tab_info()` after active_tab change |
| Close/minimize/zoom operate on active tab | ✅ | They already use `active_surface_for_frame()` which respects `active_tab` |
| Rim drag on active surface | ✅ | Already uses `active_surface_for_frame()` |
| No kernel/ABI changes | ✅ | All userland, existing opcodes |
| Per-tab geometry preservation | ✅ | ShellTab stores saved_{x,y,w,h} fields, or use frame's normal_{x,y,w,h} |

---

## Current Tab Model State

### ShellFrame (line 216)

```rust
struct ShellFrame {
    frame_id: u32,
    active_tab: u8,         // index into tabs[]
    tab_count: u8,          // valid entries in tabs[]
    tabs: [Option<ShellTab>; 8],
    flags: u32,
    normal_x: i32, normal_y: i32, normal_w: u32, normal_h: u32,
}
```

### ShellTab (line 201)

```rust
struct ShellTab {
    surface_id: u64,
    title_id: u64,   // Reserved
    flags: u32,      // Reserved
}
```

### FRAMES[0] boot init (line 1647)

```rust
FRAMES[0] = Some(ShellFrame {
    frame_id: 1,
    active_tab: 0,
    tab_count: 1,      // <-- SINGLE TAB
    tabs: [
        Some(ShellTab { surface_id: SURFACE_ID_APP, title_id: 0, flags: 0 }),
        None, None, None, None, None, None, None,
    ],
    ...
});
```

**V1 single-tab constraint:** Only tab 0 exists. Tab switching cannot be observed until `tab_count >= 2`. The switch algorithm is designed for multi-tab but won't trigger until a second tab is added.

### Existing surfaces available for multi-tab

| Surface ID | Constant | Created at boot | In a frame? |
|------------|----------|-----------------|-------------|
| 100 | SURFACE_ID_APP | ✅ (0xEC) | ✅ (FRAMES[0].tabs[0]) |
| 101 | SURFACE_ID_STATIC | ✅ (0xEC) | ❌ (standalone) |
| 102 | SURFACE_ID_TEST3 | ✅ (0xEC) | ❌ (standalone) |
| 103 | SURFACE_ID_TEST4 | ✅ (0xEC) | ❌ (standalone) |
| 200 | SURFACE_ID_LINEN | ✅ (app) | ❌ (standalone) |

For a multi-tab test, surface 101 (SURFACE_ID_STATIC) is the natural second tab candidate — it already has a distinct color (0x00704890, purple when id & 1 != 0) to visually differentiate from surface 100 (0x00303860, dark blue).

---

## Click Path for Tab Strip

### Current flow (tab strip → no-op capture)

```
mouse click → pdx_listen_raw(0) → sexinput dispatch → 
  silk-shell handle_click_state() → click_hit_test_and_focus(px, py, buttons) →
    hit_test_at(px, py) → 
      point_in_surface(px, py, focused) → 
        hit_test_surface_chrome(px, py, focused) →
          frame_tab_at() returns Some(tab_index) → 
            HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_TAB_STRIP } →
    match target:
      FrameChrome { kind: FRAME_CHROME_TAB_STRIP } →
        CAPTURE: [shell.frame.chrome.capture] (no-op) ← CURRENT BEHAVIOR
```

### Proposed flow (tab strip → switch)

```
mouse click → ... HitTarget::FrameChrome { kind: FRAME_CHROME_TAB_STRIP } →
    match target:
      FrameChrome { kind: FRAME_CHROME_TAB_STRIP } →
        tab_index = frame_tab_at(frame_id, px, py) →
          Some(index) → proceed
          None → capture (no-op, shouldn't happen if hit_target produced)
        if index == frame.active_tab → capture (already on this tab)
        if index != frame.active_tab → SWITCH:
          1. old_sid = active_surface_for_frame(frame_id)
          2. Save old surface bounds via get_surface_bounds(old_sid)
          3. 0xEE old_sid (hide old tab surface)
          4. new_sid = frame.tabs[index].surface_id
          5. 0xEC new_sid at saved bounds (show new tab surface)
          6. try_set_focus(new_sid)
          7. Update ShellFrame.active_tab = index
          8. send_frame_tab_info(frame_id) → sexdisplay highlights new tab
          9. Emit [shell.frame.tab.switch] frame=N from=N to=N surface=N
```

---

## Proposed Tab Switch Algorithm

### Helper: `switch_to_tab(frame_id, tab_index) -> bool`

```rust
unsafe fn switch_to_tab(frame_id: u32, tab_index: u32) -> bool {
    // Validate frame exists and tab_index is in range.
    let frame = match FRAMES.iter_mut().find_map(|f| {
        if let Some(frame) = f {
            if frame.frame_id == frame_id { Some(frame) } else { None }
        } else { None }
    }) {
        Some(f) => f,
        None => return false,
    };

    // Already on this tab?
    if tab_index as u8 >= frame.tab_count {
        return false;
    }
    if tab_index as u8 == frame.active_tab {
        return true; // already on this tab — not an error
    }

    // Get old and new tab surfaces.
    let old_surface_id = active_surface_for_frame(frame_id)
        .unwrap_or(0);
    let new_surface_id = match &frame.tabs[tab_index as usize] {
        Some(tab) => tab.surface_id,
        None => return false,
    };
    if new_surface_id == old_surface_id {
        return true; // same surface, no-op
    }

    // Save current geometry from old surface before hiding.
    let bounds = if old_surface_id != 0 {
        get_surface_bounds(old_surface_id)
    } else {
        Some((frame.normal_x, frame.normal_y, frame.normal_w, frame.normal_h))
    };
    let (sx, sy, sw, sh) = match bounds {
        Some(b) => b,
        None => return false,
    };

    // Hide old surface.
    if old_surface_id != 0 && surface_is_alive(old_surface_id) {
        pdx_call(SLOT_DISPLAY, 0xEE, old_surface_id, 0, 0);
    }

    // Show new surface at saved geometry.
    pdx_call(SLOT_DISPLAY, 0xEC, new_surface_id,
        (sy as u64) << 32 | sx as u64,
        (sh as u64) << 32 | sw as u64);
    update_local_geometry(new_surface_id, sx, sy, sw, sh);

    // Update frame state.
    frame.active_tab = tab_index as u8;
    drop(frame); // release mutable borrow

    // Set focus to new surface.
    try_set_focus(new_surface_id);

    // Clear drag if dragging the old surface.
    clear_drag_if_dead();

    // Notify sexdisplay of tab metadata change.
    send_frame_tab_info(frame_id);

    // Emit switch marker.
    unsafe {
        static mut TAB_SWITCH_BUDGET: u32 = 8;
        let b = &mut TAB_SWITCH_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.tab.switch] frame={} from={} to={} surface={}",
                frame_id, old_surface_id, new_surface_id, new_surface_id);
        }
    }
    true
}
```

### Modified click_hit_test_and_focus dispatch

Replace the current tab strip capture (line 1513-1524):

```rust
// BEFORE (current):
} else {
    // Non-rim chrome (tab strip, reserved): capture/no-op.
    unsafe {
        static mut CHROME_CAPTURE_BUDGET: u32 = 4;
        let b = &mut CHROME_CAPTURE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.chrome.capture] frame={} kind={} x={} y={}",
                frame_id, kind, px, py);
        }
    }
}

// AFTER (proposed):
} else if kind == FRAME_CHROME_TAB_STRIP {
    // Tab strip click: switch to tab at pointer position.
    if let Some(tab_index) = frame_tab_at(frame_id, px, py) {
        if !switch_to_tab(frame_id, tab_index) {
            // Capture as no-op if switch fails.
            unsafe {
                static mut TAB_SWITCH_CAPTURE_BUDGET: u32 = 4;
                let b = &mut TAB_SWITCH_CAPTURE_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[shell.frame.tab.switch.reject] frame={} tab={} reason=switch_failed",
                        frame_id, tab_index);
                }
            }
        }
    }
} else {
    // Other non-rim chrome (reserved): capture/no-op.
    unsafe {
        static mut CHROME_CAPTURE_BUDGET: u32 = 4;
        let b = &mut CHROME_CAPTURE_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[shell.frame.chrome.capture] frame={} kind={} x={} y={}",
                frame_id, kind, px, py);
        }
    }
}
```

---

## Active/Inactive Surface Visibility Rule

### Principle: Only one tab surface visible per frame

| State | Active tab surface | Inactive tab surfaces |
|-------|--------------------|----------------------|
| Tab A active | Rendered (focused, Pass 2) | Hidden (0xEE) |
| Tab B active | Rendered (focused, Pass 2) | Hidden (0xEE) |

**Rationale:**
- Sexdisplay renders ALL active surfaces in compositing order
- If both tabs' surfaces are active and overlapping, the inactive one would show through in Pass 1
- Tab switching expects only the active tab's content to be visible
- Using 0xEE/0xEC to toggle visibility is the same mechanism used by minimize/restore

**Alternative considered (rejected):** Keep both surfaces active and rely on z-order/focus compositing. Rejected because:
- Non-overlapping surfaces (e.g., tabs at different positions) would both be visible
- Overlapping surfaces cost compositing time for the hidden tab
- Complex interaction with drag/resize (both surfaces would respond)

### Geometry preservation

When switching tabs, the old surface's geometry is captured via `get_surface_bounds()` and applied to the new surface via 0xEC. This ensures all tabs within a frame share the same position/size at the time of switch.

**Limitation:** If the user moves/resizes the frame, then switches tabs, then switches back, the previously hidden surface gets the current frame geometry (not its pre-hide geometry). This is acceptable for V1 — tabs within a frame share geometry. In the future, if per-tab geometry is needed, ShellTab can store saved_{x,y,w,h} fields.

---

## Focus/Drag Invariants

| Invariant | Enforcement |
|-----------|-------------|
| Focus is always on active tab's surface | `try_set_focus(new_surface_id)` called after every switch |
| Drag is cleared if old surface was being dragged | `clear_drag_if_dead()` called after switch (0xEE old surface triggers it) |
| Inactive tabs never focused | 0xEE'd surfaces return false for `surface_is_alive()`, `try_set_focus()` rejects dead surfaces |
| Frame chrome renders around active tab | Sexdisplay composite_pixel draws rim/lights/tabs around focused surface only |

---

## Interaction with Close/Minimize/Zoom

All three light actions use `active_surface_for_frame(frame_id)` to get the target surface. Since this function already resolves `frame.tabs[frame.active_tab].surface_id`, they automatically operate on the active tab:

| Action | Behavior with multi-tab |
|--------|-------------------------|
| **CLOSE** | Destroys active tab's surface. Future: switch to adjacent tab before closing frame. |
| **MINIMIZE** | Hides entire frame (all tabs). `restore_minimized_frame()` shows active tab. |
| **ZOOM** | Maximizes active tab's surface to fill content area. Normal geometry saved per-frame. |
| **Rim drag** | Drags active tab's surface. All tabs share frame position. |

**CLOSE corner case:** With multi-tab, closing the last tab's surface should close the frame (set FRAMES entry to None, or similar). Closing a non-last tab should:
1. Remove the tab from the array
2. Shift remaining tabs left
3. Focus the next tab (or previous if closing the last index)
4. Update tab_count

This is deferred — V1 tab switch does not implement tab close via tab strip.

---

## Geometry Storage Needs

**Verdict: No additional geometry storage needed for V1.**

The current mechanism is sufficient:
- `get_surface_bounds(old_surface_id)` captures current geometry before hiding
- 0xEC applies captured geometry to new surface
- All tabs in a frame share the same position/size
- ShellFrame.normal_{x,y,w,h} already stores pre-zoom geometry per-frame

**Future extension (if per-tab geometry needed):**
```rust
struct ShellTab {
    surface_id: u64,
    title_id: u64,
    flags: u32,
    // Optional per-tab geometry (zero = unset, use frame geometry)
    saved_x: i32,
    saved_y: i32,
    saved_w: u32,
    saved_h: u32,
}
```

---

## Implementation Files

### Modified

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Add `switch_to_tab()` helper. Modify `click_hit_test_and_focus()` tab strip arm from capture → switch. Add `[shell.frame.tab.switch]` and `[shell.frame.tab.switch.reject]` markers. |

### NOT Modified

- `kernel/` — no kernel ABI changes
- `crates/sex-pdx/src/lib.rs` — no IPC protocol changes
- `servers/sexdisplay/src/main.rs` — no renderer changes (tab block highlight already driven by 0xFD metadata which shell sends after switch)
- `servers/silkbar/` — no forwarding changes
- `crates/silkbar-model/` — no model changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

---

## Proof Markers

### New markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.tab.switch] frame=N from=N to=N surface=N` | 8 | Successful tab switch |
| `[shell.frame.tab.switch.reject] frame=N tab=N reason=...` | 4 | Failed tab switch attempt |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.tab.model]` | Tab strip model proof (boot) |
| `[shell.frame.tab.info.send]` | Tab metadata sent to sexdisplay (boot + after switch) |
| `[sexdisplay.surface.tab.info]` | Sexdisplay receives tab metadata |
| `[shell.frame.light.close/minimize/zoom]` | Light actions still work |
| `[shell.frame.zoom/unzoom]` | ZOOM light toggle |
| `[shell.frame.minimize/restore]` | MINIMIZE/Restore |
| `[shell.drag.start/move/end]` | Rim drag |
| `[shell.focus.set]` | Focus change via try_set_focus |

---

## Multi-Tab Test Model

For V1 testing (optional, before tab switch implementation), a second tab can be added to FRAMES[0] at boot without changing any other behavior:

```rust
// At FRAMES[0] init (line 1647):
FRAMES[0] = Some(ShellFrame {
    frame_id: 1,
    active_tab: 0,
    tab_count: 2,  // was 1
    tabs: [
        Some(ShellTab { surface_id: SURFACE_ID_APP, title_id: 0, flags: 0 }),
        Some(ShellTab { surface_id: SURFACE_ID_STATIC, title_id: 0, flags: 0 }),  // NEW
        None, None, None, None, None, None,
    ],
    ...
});
```

This requires:
- `tab_count = 2` (was 1)
- Second `Some(ShellTab { ... SURFACE_ID_STATIC ... })` entry

Surface 101 (SURFACE_ID_STATIC) is already created at boot via 0xEC (line 1694). It has a different color (purple 0x00704890) for visual differentiation.

**Rendering note:** Without the switch algorithm, both surfaces are active simultaneously. Surface 101 would be composited in Pass 1 (non-focused), surface 100 in Pass 2 (focused). If they overlap at the same geometry, surface 101 is hidden behind surface 100. The tab strip would show two equal-width tab blocks.

This multi-tab setup can be applied in either:
- **Phase A: FRAME_MULTI_TAB_MODEL_V1** (pre-switch model-only phase, recommended if testing before switching)
- **Phase B: FRAME_TAB_SWITCH_V1** (combined with switch algorithm in a single implementation phase)

---

## STOP Conditions

If any of these are encountered during implementation, STOP and re-assess:

1. **No second tab exists** — Tab switching cannot be tested with only 1 tab. Either add synthetic second tab first (FRAME_MULTI_TAB_MODEL_V1) or accept untested V1 switch code.

2. **0xEC on previously 0xEE'd surface fails** — If sexdisplay's 0xEC handler rejects re-creating a surface that was destroyed, the switch mechanism breaks. Check: 0xEC create path uses `!slot.active` which should match any inactive slot. Should work unless all 16 surface slots are exhausted.

3. **Owner authentication on 0xEC create** — The 0xEC create path sets `owner_pd: msg.caller_pd`. If the caller PD changes between boot and tab switch, the old surface's slot becomes owned by a different PD. Since silk-shell is the sole creator of frame surfaces, this is not a concern — the PD identity is consistent.

4. **Surface slot exhaustion** — With MAX_SURFACES=16, creating/destroying tabs repeatedly could exhaust slots if 0xEC always allocates a new slot instead of reusing the old one. Analysis: 0xEC first tries to find `slot.active && slot.surface_id == surface_id` (upsert). This only matches if the surface is still active. After 0xEE, the surface is inactive, so 0xEC enters the create path. The create path finds `!slot.active`. Since 0xEE sets `active = false` in-place, the slot is immediately reusable. No exhaustion.

5. **Drag during tab switch** — If the user is dragging when clicking a tab (unlikely but possible), `clear_drag_if_dead()` handles cleanup. The interaction state machine should prevent tab switch during drag (click drag releases capture).

6. **Focus loss on tab switch** — `try_set_focus()` handles focus change. If the new surface is dead/invalid, focus falls back to the next alive surface via `clear_focus_if_dead()`.

---

## Next Phase

### FRAME_TAB_SWITCH_V1

```
MISSION: FRAME_TAB_SWITCH_V1

Implement tab switching on tab strip click. Shell-only. No renderer changes.

Design complete in FRAME_TAB_SWITCH_PLAN_V1.md.

Pre-requisite: FRAME_MULTI_TAB_MODEL_V1 (add second tab to FRAMES[0])
OR include multi-tab model in this phase.

Changes:

1. servers/silk-shell/src/main.rs:
   a. (Optional) FRAMES[0] boot init: tab_count = 2, add second ShellTab for SURFACE_ID_STATIC
   b. Add switch_to_tab() helper (save bounds, 0xEE old, 0xEC new, try_set_focus,
      clear_drag_if_dead, send_frame_tab_info, budget marker)
   c. Modify click_hit_test_and_focus() tab strip arm:
      Replace no-op capture with:
        - frame_tab_at() → tab_index
        - if tab_index == active_tab → no-op
        - else switch_to_tab()
   d. Add [shell.frame.tab.switch] (budget 8) and [shell.frame.tab.switch.reject] (budget 4)

Forbidden:
- Text rendering
- Renderer changes
- ABI/protocol changes
- Dynamic allocation
- Per-tab geometry tracking (deferred)
- Tab close behavior (deferred)
- Kernel edits

PASS:
- Default build passes
- Synthetic build passes (if applicable)
- Tab strip click on active tab → no-op
- Tab strip click on inactive tab → switch:
  - old surface hidden (0xEE)
  - new surface shown (0xEC at same bounds)
  - focus set to new surface
  - sexdisplay tab block highlight updates
- Frame Lights still work (use active_surface_for_frame which respects active_tab)
- Rim drag still works on active surface
- No panic/#PF/#GP
- No kernel edits
- [shell.frame.tab.switch] fires on switch
- [shell.frame.tab.info.send] fires after switch
```
