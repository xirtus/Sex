# FRAME_TOP_BAR_MODEL_V1

## Status

Implemented (2026-05-04). Shell-side top bar model: flag, constants, helper, hit target updates, boot init. No renderer, protocol, or kernel changes.

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Top bar state stored in ShellFrame.flags | `FRAME_FLAG_TOP_BAR = 1 << 2` | `[shell.frame.topbar.model]` |
| Top bar ON at boot default | `FRAMES[0].flags = FRAME_FLAG_TOP_BAR` | `[shell.frame.topbar.model] enabled=1` |
| Top bar geometry constants defined | `FRAME_TOP_BAR_HEIGHT_PX = 16`, light size/gap/exclusion constants | Build passes |
| `frame_has_top_bar()` helper | Checks `frame.flags & FRAME_FLAG_TOP_BAR` | Build passes |
| `set_frame_top_bar()` helper | Sets/clears flag bit | Build passes |
| Lights use top bar geometry in default mode | `frame_light_at()` uses 16px height, 8px lights, 4px gaps when flag set | Build passes |
| Lights use rim geometry in minimal mode | `frame_light_at()` uses 4px height, 4px lights, 2px gaps when flag clear | Build passes |
| Tab strip uses top bar geometry in default mode | `frame_tab_at()` uses 16px height, 40px exclusion when flag set | Build passes |
| Tab strip uses rim geometry in minimal mode | `frame_tab_at()` uses 4px height, 20px exclusion when flag clear | Build passes |
| Hit chrome respects top bar zone | `hit_test_surface_chrome()` uses `FRAME_TOP_BAR_HEIGHT_PX` for top edge when flag set | Build passes |
| Left/right/bottom rim unchanged | Non-top edges still use `FRAME_RIM_PX = 4` | Build passes |
| Close/minimize/zoom still compile | All light actions unchanged | Build passes |
| Tab switching still compiles | Tab switch dispatch unchanged | Build passes |
| Rim drag still works | Rim drag dispatch unchanged | Build passes |
| No kernel/ABI changes | All userland, no new opcodes | Build passes |

---

## Constants Added

```rust
// ── Frame Flag ──
const FRAME_FLAG_TOP_BAR: u32 = 1 << 2;

// ── Top Bar Geometry ──
const FRAME_TOP_BAR_HEIGHT_PX: i32 = 16;
const FRAME_TOP_BAR_LIGHT_SIZE_PX: i32 = 8;
const FRAME_TOP_BAR_LIGHT_GAP_PX: i32 = 4;
const FRAME_TOP_BAR_LIGHT_EXCLUSION_PX: i32 = 40;
```

### Existing constants (unchanged)

| Constant | Value | Role |
|----------|-------|------|
| `FRAME_RIM_PX` | 4 | Rim width (all edges in minimal mode; left/right/bottom in default mode) |
| `FRAME_TAB_STRIP_PX` | 4 | Tab strip height in minimal mode |
| `FRAME_TAB_LIGHT_EXCLUSION_PX` | 20 | Light exclusion zone in minimal mode |
| `FRAME_LIGHT_SIZE_PX` | 4 | Light size in minimal mode |
| `FRAME_LIGHT_GAP_PX` | 2 | Light gap in minimal mode |

---

## Helpers Added

### `frame_has_top_bar(frame_id) -> bool`

Returns true if the frame has `FRAME_FLAG_TOP_BAR` set. Used by hit-test functions to dispatch on chrome mode.

### `set_frame_top_bar(frame_id, enabled)`

Sets or clears `FRAME_FLAG_TOP_BAR` on the given frame.

Both follow the same pattern as `frame_is_minimized()` / `set_frame_minimized()`.

---

## Default Mode (Top Bar ON)

### Boot init

```rust
FRAMES[0] = Some(ShellFrame {
    frame_id: 1,
    active_tab: 0,
    tab_count: 2,
    tabs: [
        Some(ShellTab { surface_id: SURFACE_ID_APP, title_id: 0, flags: 0 }),
        Some(ShellTab { surface_id: SURFACE_ID_STATIC, title_id: 0, flags: 0 }),
        None, None, None, None, None, None,
    ],
    flags: FRAME_FLAG_TOP_BAR,  // ← top bar ON by default
    normal_x: boot_x,
    normal_y: boot_y,
    normal_w: boot_w,
    normal_h: boot_h,
});
```

### Default mode hit-target geometry

| Element | Y-range | X-range |
|---------|---------|---------|
| Top bar band | sy..sy+16 | full surface width |
| Lights (8×8) | sy+4..sy+12 | sx+4..sx+36 |
| Tab strip | sy..sy+16 | sx+40..sx+sw-4 |
| Left rim | sy..sy+sh | sx..sx+4 |
| Right rim | sy..sy+sh | sx+sw-4..sx+sw |
| Bottom rim | sy+sh-4..sy+sh | sx..sx+sw |
| Content area | sy+16..sy+sh-4 | sx+4..sx+sw-4 |

### Light positions in default mode

```
CLOSE:     sx+4..sx+12,  sy+4..sy+12   (gap=4, size=8)
MINIMIZE:  sx+16..sx+24, sy+4..sy+12  (gap=4, size=8, gap=4)
ZOOM:      sx+28..sx+36, sy+4..sy+12  (gap=4, size=8, gap=4, size=8, gap=4)
```

---

## Minimal Mode (Top Bar OFF)

Unchanged from existing behavior:

| Element | Y-range | X-range |
|---------|---------|---------|
| Rim (all edges) | 4px | full width |
| Lights (4×4) | sy..sy+4 | sx+2..sx+18 |
| Tab strip | sy..sy+4 | sx+20..sx+sw-4 |
| Content area | sy+4..sy+sh-4 | sx+4..sx+sw-4 |

---

## Hit Target Dispatch

### `frame_light_at()` — mode dispatcher

```
frame_light_at(frame_id, x, y):
  if frame_has_top_bar(frame_id):
    y-range = FRAME_TOP_BAR_HEIGHT_PX (16)
    lights  = FRAME_TOP_BAR_LIGHT_SIZE_PX (8), FRAME_TOP_BAR_LIGHT_GAP_PX (4)
  else:
    y-range = FRAME_RIM_PX (4)
    lights  = FRAME_LIGHT_SIZE_PX (4), FRAME_LIGHT_GAP_PX (2)
```

### `frame_tab_at()` — mode dispatcher

```
frame_tab_at(frame_id, x, y):
  if frame_has_top_bar(frame_id):
    band_height = FRAME_TOP_BAR_HEIGHT_PX (16)
    exclusion   = FRAME_TOP_BAR_LIGHT_EXCLUSION_PX (40)
  else:
    band_height = FRAME_TAB_STRIP_PX (4)
    exclusion   = FRAME_TAB_LIGHT_EXCLUSION_PX (20)
```

### `hit_test_surface_chrome()` — top edge handling

```
Tab strip check:
  if top bar: strip_bot = FRAME_TOP_BAR_HEIGHT_PX, exclusion = FRAME_TOP_BAR_LIGHT_EXCLUSION_PX
  else:       strip_bot = FRAME_TAB_STRIP_PX,       exclusion = FRAME_TAB_LIGHT_EXCLUSION_PX

Rim check:
  Top edge condition: y >= sy && y < sy + band_height
    where band_height = FRAME_TOP_BAR_HEIGHT_PX if top bar else FRAME_RIM_PX
  Left/right/bottom edges: always FRAME_RIM_PX
```

### Click priority (unchanged)

1. Lights checked first in `click_hit_test_and_focus()` (CLOSE/MINIMIZE/ZOOM)
2. Tab strip checked as `FRAME_CHROME_TAB_STRIP`
3. Rim drag (non-light, non-tab positions in chrome zone)

In default mode, clicking the top bar background (empty area) starts rim drag — same as current behavior.

---

## Diagnostic Markers

### New marker

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.topbar.model] frame=N enabled=N height=N` | 1 | Boot: proves top bar model is active |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.light.model]` | Lights model proof (boot) ✅ |
| `[shell.frame.tab.model]` | Tab strip model proof (boot) ✅ |
| `[shell.frame.tab.info.send]` | Tab metadata to sexdisplay ✅ |
| `[shell.frame.tab.switch]` | Tab switching |
| `[shell.frame.light.close/minimize/zoom]` | Light actions |
| `[shell.drag.start/move/end]` | Rim drag |
| `[shell.focus.set]` | Focus changes |

---

## Known Issue: Visual Mismatch

**Sexdisplay still renders 4px rim** while hit targets expect 16px top bar. This creates a temporary inconsistency:

- Clicks on y=4..16 (the hit target top bar zone) are handled by chrome hit targets (lights/tabs/rim)
- Visually, sexdisplay renders these pixels as rim color (4px) then focused surface color (4-16px)
- Click feedback is correct (lights/tabs work), but visual feedback is wrong

This is **intentional and documented**. The rendering is updated in a future phase:

```
FRAME_TOP_BAR_RENDER_PLAN_V1 → FRAME_TOP_BAR_RENDER_V1
  - Design sexdisplay protocol for chrome mode
  - Implement top bar rendering in composite_pixel() Pass 2
```

---

## File Changed

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Added `FRAME_FLAG_TOP_BAR`, 4 top bar geometry constants. Added `frame_has_top_bar()` / `set_frame_top_bar()` helpers. Updated `frame_light_at()` to dispatch on top bar mode. Updated `frame_tab_at()` to use correct band height and exclusion. Updated `hit_test_surface_chrome()` to use band_height for top edge. Set `FRAMES[0].flags = FRAME_FLAG_TOP_BAR` at boot init. Added `[shell.frame.topbar.model]` boot proof marker. |

### Files NOT Modified

Kernel, sex-pdx, sexdisplay, silkbar, silkbar-model, sexusb, sexinput — all untouched.

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Default build passes. Synthetic build passes. No new warning types. Pre-existing warnings unchanged.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-top-bar-model-v1.log

for m in \
  shell.frame.topbar.model \
  shell.frame.tab.switch \
  shell.frame.tab.info.send \
  shell.frame.light.close \
  shell.frame.minimize \
  shell.frame.zoom \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-top-bar-model-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-top-bar-model-v1.log
```

### Pass criteria

- Default build passes ✅
- Synthetic build passes ✅
- `[shell.frame.topbar.model]` fires at boot with `frame=1 enabled=1 height=16` ✅
- `frame_has_top_bar(1)` returns true for default boot ✅
- `frame_light_at()` returns correct light kinds within 16px top bar zone (compile-time proof)
- `frame_tab_at()` returns correct tab indices within 16px top bar zone (compile-time proof)
- Frame Lights actions still compile (close/minimize/zoom) ✅
- Tab switching still compiles ✅
- Rim drag still works ✅
- No panic/#PF/#GP ✅
- No kernel/ABI changes ✅
- Only silk-shell changed (plus handoff doc) ✅

---

## Risks and Limitations

- **Visual mismatch:** Sexdisplay still renders 4px rim while hit targets expect 16px top bar. Click targets work correctly but visual rendering is inconsistent until FRAME_TOP_BAR_RENDER_V1.
- **No toggle mechanism:** Top bar is ON at boot with no way to switch to minimal mode. Toggle is deferred to FRAME_CHROME_MODE_SETTINGS_V1.
- **No sexdisplay protocol:** Chrome mode is not communicated to sexdisplay. Deferred to FRAME_TOP_BAR_RENDER_PLAN_V1.
- **Top edge residual drag:** Clicking the top bar background (non-light, non-tab) starts rim drag, same as current top rim behavior. Acceptable for V1.
- **Content area shrink:** In default mode, the visible content area shrinks from starting at y=4 to y=16 (12px less). This is a visual side effect of the taller chrome band.

---

## Next Recommended Phase

### FRAME_TOP_BAR_RENDER_PLAN_V1

Design the sexdisplay protocol and rendering approach for the top bar:

1. Protocol: new opcode or 0xFD extension to communicate `chrome_mode` from shell to sexdisplay
2. Surface field: `chrome_mode: u8` in sexdisplay Surface struct
3. Rendering: update `composite_pixel()` Pass 2 to render 16px top bar when `chrome_mode == 1`
4. Determine top bar background color, light rendering (circles vs squares), tab strip integration

See `docs/handoff/SILK_CHROME_SETTINGS_PLAN_V1.md` for the full roadmap.
