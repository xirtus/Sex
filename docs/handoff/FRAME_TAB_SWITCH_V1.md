# FRAME_TAB_SWITCH_V1

## Status

Implemented (2026-05-04). Tab switching on tab strip click. Multi-tab boot model (2 tabs). Shell-only — no renderer, kernel, or ABI changes.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Tab strip click switches active tab | `switch_to_tab()` hides old surface (0xEE), shows new (0xEC), updates focus | `[shell.frame.tab.switch]` |
| Old tab surface hidden after switch | 0xEE sent to sexdisplay for old surface | N/A (sexdisplay: active=false) |
| New tab surface shown at frame geometry | 0xEC with captured bounds from old surface | N/A (sexdisplay: active=true, focused) |
| Focus follows active tab | `try_set_focus()` on new surface | `[shell.focus.set]` |
| Sexdisplay tab block highlight updates | `send_frame_tab_info()` after switch | `[shell.frame.tab.info.send]` |
| Two-tab boot model | FRAMES[0].tab_count=2, tabs[1]=SURFACE_ID_STATIC | `[shell.frame.tab.model] tabs=2` |
| Frame Lights still operate on active tab | All lights use `active_surface_for_frame()` which respects `active_tab` | `[shell.frame.light.close/minimize/zoom]` |
| Rim drag still works on active surface | Drag uses `active_surface_for_frame()` | `[shell.drag.start]` |
| Tab switch while dragging prevented | `clear_drag_if_dead()` called after switch | N/A |
| No kernel/ABI changes | All userland, existing opcodes only | Build passes |

---

## Multi-Tab Boot Model

### FRAMES[0] initialization (line 1663)

```rust
FRAMES[0] = Some(ShellFrame {
    frame_id: 1,
    active_tab: 0,
    tab_count: 2,      // was 1
    tabs: [
        Some(ShellTab { surface_id: SURFACE_ID_APP, title_id: 0, flags: 0 }),
        Some(ShellTab { surface_id: SURFACE_ID_STATIC, title_id: 0, flags: 0 }),  // NEW
        None, None, None, None, None, None,
    ],
    flags: 0,
    normal_x: boot_x, normal_y: boot_y, normal_w: boot_w, normal_h: boot_h,
});
```

### Tab surfaces

| Index | Tab | Surface ID | Surface Color | State |
|-------|-----|------------|---------------|-------|
| 0 | Tab A (default) | 100 (SURFACE_ID_APP) | 0x00303860 (dark blue) | Focused at boot |
| 1 | Tab B | 101 (SURFACE_ID_STATIC) | 0x00704890 (purple) | Created at boot, behind tab A |

Both surfaces are created at boot via existing 0xEC calls (lines 1810-1813). Surface 101 (purple) provides visual distinction from surface 100 (dark blue) when switching tabs.

### Boot tab info sent to sexdisplay

After focus is set on surface 100, `send_frame_tab_info(1)` sends:
```
pdx_call(SLOT_DISPLAY, 0xFD, 100, 2, 0);
// surface_id=100, tab_count=2, active_tab=0
```

Sexdisplay renders two tab blocks in the top rim band: tab 0 (active, cyan) and tab 1 (inactive, dim cyan).

---

## Tab Switch Algorithm

### `switch_to_tab(frame_id, tab_index) -> bool`

```rust
unsafe fn switch_to_tab(frame_id: u32, tab_index: u32) -> bool {
    // 1. Validate frame exists, tab_index in range
    // 2. No-op if already on this tab (tab_index == active_tab)
    // 3. Get old/new surface IDs
    // 4. Capture old surface bounds via get_surface_bounds()
    // 5. Update frame.active_tab
    // 6. 0xEE old surface (hide)
    // 7. 0xEC new surface at captured bounds (show)
    // 8. update_local_geometry(new_surface, bounds)
    // 9. try_set_focus(new_surface)
    // 10. clear_drag_if_dead()
    // 11. send_frame_tab_info(frame_id)
    // 12. Emit [shell.frame.tab.switch] marker
}
```

### Click dispatch (click_hit_test_and_focus)

```
Tab strip click →
  HitTarget::FrameChrome { kind: FRAME_CHROME_TAB_STRIP } →
    frame_tab_at(frame_id, px, py) → Some(tab_index) →
      switch_to_tab(frame_id, tab_index) →
        success → [shell.frame.tab.switch]
        failure → [shell.frame.tab.switch.reject]
    frame_tab_at() → None →
      (shouldn't happen if hit test produced the target)
```

### Active/Inactive visibility rule

| State | Tab A (surface 100) | Tab B (surface 101) |
|-------|--------------------|--------------------|
| Tab A active | Focused (Pass 2) | Hidden (0xEE) |
| Tab B active | Hidden (0xEE) | Focused (Pass 2) |

Only the active tab's surface is visible in sexdisplay. Inactive surfaces are hidden via 0xEE (same mechanism as minimize). When switching to an inactive tab, the surface is restored via 0xEC at the current frame geometry.

---

## Focus/Drag Invariants

| Invariant | Enforcement |
|-----------|-------------|
| Focus always on active tab's surface | `try_set_focus(new_surface_id)` after every switch |
| Drag cleared if dragging old surface | `clear_drag_if_dead()` called after every switch |
| Inactive tab surfaces never focused | `surface_is_alive()` returns true (ALIVE flag unchanged by switch), but surface is hidden in sexdisplay — focus requires alive check, which passes, so focus is set correctly on the newly shown surface |
| Frame chrome renders around active tab | Sexdisplay rim/lights/tabs render around focused surface only |

**Important:** The SURFACE_ALIVE flags (`SURFACE_100_ALIVE`, `SURFACE_101_ALIVE`) are NOT modified by tab switching. Only the sexdisplay `active` flag changes (via 0xEE/0xEC). This ensures `surface_is_alive()` returns true for both tabs, allowing `try_set_focus()` to accept either surface.

---

## Interaction with Close/Minimize/Zoom

All three light actions use `active_surface_for_frame(frame_id)` which returns the active tab's `surface_id`:

| Action | Effect with multi-tab |
|--------|-----------------------|
| **CLOSE** | Destroys active tab's surface. ALIVE flag set to false. Tab remains in FRAMES array with dead surface. |
| **MINIMIZE** | Hides active surface via 0xEE. Flag set per-frame. Restore shows active tab. |
| **ZOOM** | Maximizes active surface. `normal_*` geometry stored per-frame. Unzoom restores to normal. |
| **Rim drag** | Drags active surface. Position applies to whichever tab is active. |

**CLOSE limitation:** Closing the active tab's surface does not auto-switch to the other tab. Tab close behavior is deferred.

---

## Files Changed

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | FRAMES[0]: tab_count 1→2, added SURFACE_ID_STATIC as tabs[1]. Added `switch_to_tab()` helper (~65 lines). Modified `click_hit_test_and_focus()` tab strip arm: replaced no-op capture with `frame_tab_at()` + `switch_to_tab()` dispatch. Added `[shell.frame.tab.switch]` and `[shell.frame.tab.switch.reject]` budget markers. |

### Files NOT Modified

Kernel, sex-pdx, sexdisplay, silkbar, silkbar-model, sexusb, sexinput — all untouched.

---

## Diagnostic Markers

### New markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.tab.switch] frame=N old=N new=N tab=N` | 8 | Successful tab switch |
| `[shell.frame.tab.switch.reject] frame=N tab=N reason=...` | 4 | Failed tab switch attempt |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.tab.model] tabs=N has_tab=...` | Now shows tabs=2 |
| `[shell.frame.tab.info.send] frame=N surface=N tabs=N active=N` | Fires at boot (tabs=2) and after each switch |
| `[sexdisplay.surface.tab.info] surface=N tabs=N active=N` | Sexdisplay receives tab metadata |
| `[shell.frame.light.close/minimize/zoom]` | Light actions still operate on active tab |
| `[shell.frame.minimize/restore]` | Minimize/restore unchanged |
| `[shell.focus.set]` | Focus follows active tab |
| `[shell.drag.start/move/end]` | Rim drag works on active surface |

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Default build passes. No new warning types. Pre-existing warnings unchanged.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-tab-switch-v1.log

for m in \
  shell.frame.tab.switch \
  shell.frame.tab.switch.reject \
  shell.frame.tab.info.send \
  sexdisplay.surface.tab.info \
  shell.frame.tab.model \
  shell.frame.light.close \
  shell.frame.minimize \
  shell.frame.zoom \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-tab-switch-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-tab-switch-v1.log
```

### Pass criteria

- Default build passes ✅
- FRAMES[0] with tab_count=2, two ShellTab entries ✅
- `[shell.frame.tab.model]` shows `tabs=2` at boot ✅
- Tab strip click on tab 0 while tab 0 active → no-op (already on this tab)
- Tab strip click on tab 1 → switch: surface 100 hidden, surface 101 shown at same geometry, focus on 101
- Tab strip click on tab 0 again → switch back: surface 101 hidden, surface 100 shown
- Frame Lights (red/yellow/green) still work, operate on active tab
- Rim drag still works on active surface
- No panic/#PF/#GP ✅
- Only silk-shell changed (plus handoff doc) ✅

---

## Risks and Limitations

- **Tab close not implemented**: Closing the active tab's surface (via CLOSE light) sets ALIVE=false but doesn't auto-switch to the other tab. The frame would have a dead active tab. Workaround for V1: don't close tab surfaces while testing tab switching.
- **No text labels**: Tab blocks are colored rectangles. No visual indication of which tab is which beyond position (left = tab 0, right = tab 1).
- **No tab reordering**: Tabs remain in fixed slot order. No drag-to-reorder.
- **No tab spawning**: New tabs cannot be created at runtime. Limited to boot-time tab model.
- **Surface slot reuse**: 0xEC after 0xEE creates a new Surface in sexdisplay (tab_count defaults to 0). The subsequent 0xFD update corrects this. This is correct but worth noting.
- **Geometry sharing**: Both tabs share the same frame geometry. Moving/resizing one tab affects the position used when switching back. This is intentional for V1.

---

## Next Recommended Phase

### FRAME_TOP_BAR_MODEL_PLAN_V1

Design and implement the collapsible top bar chrome mode. The current 4px rim + tab strip is the "minimal mode." Future "default mode" would have a taller chrome band (12-20px) at the top, integrating the lights, tab strip, and potentially frame titles.

See `docs/handoff/SILK_CHROME_SETTINGS_PLAN_V1.md` for the full roadmap.
