# FRAME_TOP_BAR_MODEL_PLAN_V1

## Status

Design (2026-05-04). Shell-side model for collapsible top bar chrome mode. Docs-only — no code changed.

---

## Verdict: TOP_BAR_MODEL_SAFE_NOW ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Top bar state live in ShellFrame | ✅ | `FRAME_FLAG_TOP_BAR = 1 << 2` in existing `flags` field |
| Top bar geometry constants | ✅ | `FRAME_TOP_BAR_HEIGHT_PX = 16` for default mode height |
| Lights respect top bar y-range | ✅ | `frame_light_at()` checks `FRAME_TOP_BAR_HEIGHT_PX` when flag set, `FRAME_RIM_PX` when clear |
| Tab strip respects top bar y-range | ✅ | `frame_tab_at()` checks `FRAME_TOP_BAR_HEIGHT_PX` when flag set |
| Top edge rim replaced by top bar zone in default mode | ✅ | `hit_test_surface_chrome()` skips top edge rim when top bar ON |
| Left/right/bottom rim unchanged | ✅ | Only top edge affected; non-top edges keep `FRAME_RIM_PX` |
| Sexdisplay does NOT need model-phase changes | ✅ | Model is shell-only; sexdisplay protocol deferred to render phase |
| No kernel/ABI changes | ✅ | All userland, existing opcodes, no new IPC in model phase |
| Boot default = top bar ON | ✅ | `FRAMES[0].flags |= FRAME_FLAG_TOP_BAR` at init |

---

## Current State

### Existing Chrome Geometry (Minimal Mode, always-on)

```
┌──────────────────────────────────────────────────┐
│ ⬤ ⬤ ⬤  [tab strip]                              │ sy..sy+4   (4px rim, ALL edges)
│                                                    │
│ Surface content area                               │ sy+4..sy+sh-4
│  (solid color + optional fill rect)                │
│                                                    │
│                                                    │
└──────────────────────────────────────────────────┘
```

| Element | Y-range | X-range |
|---------|---------|---------|
| FRAME_RIM_PX (all edges) | 4px | full width |
| Frame Lights | sy..sy+4 | sx+2..sx+18 |
| Tab strip | sy..sy+4 | sx+20..(sx+sw-4) |
| Content area | sy+4..sy+sh-4 | sx+4..sx+sw-4 |

Constants:
- `FRAME_RIM_PX = 4` (sexdisplay + shell)
- `FRAME_TAB_STRIP_PX = 4` (shell, equals FRAME_RIM_PX in minimal mode)
- `FRAME_LIGHT_SIZE_PX = 4`, `FRAME_LIGHT_GAP_PX = 2`
- `FRAME_TAB_LIGHT_EXCLUSION_PX = 20`
- Lights: CLOSE at x=2..6, MINIMIZE at x=8..12, ZOOM at x=14..18

### Flag Usage in ShellFrame.flags

| Bit | Constant | Implemented |
|-----|----------|-------------|
| 0 | `FRAME_FLAG_MINIMIZED` | ✅ minimize/restore |
| 1 | `FRAME_FLAG_ZOOMED` | ✅ zoom/unzoom |
| 2 | `FRAME_FLAG_TOP_BAR` | ⬜ THIS PHASE |

### Rendering Locus

Both rim and lights/tab strip are rendered in `sexdisplay`'s `composite_pixel()` Pass 2 using local surface coordinates. The shell has no direct rendering authority — it controls chrome policy (position, visibility) by sending metadata opcodes to sexdisplay and by managing hit-target geometry for mouse dispatch.

---

## Proposed Top Bar Model

### State: ShellFrame.flags bit 2

```rust
/// ShellFrame.flags: frame has top bar chrome band (default mode).
/// When clear (minimal mode), only 4px neon rim is rendered.
const FRAME_FLAG_TOP_BAR: u32 = 1 << 2;
```

### Default rule: ON at boot

All frames boot with `FRAME_FLAG_TOP_BAR` set. This means:

```rust
FRAMES[0] = Some(ShellFrame {
    frame_id: 1,
    active_tab: 0,
    tab_count: 2,
    tabs: [ ... ],
    flags: FRAME_FLAG_TOP_BAR,  // ← SET by default
    normal_x: boot_x,
    normal_y: boot_y,
    normal_w: boot_w,
    normal_h: boot_h,
});
```

The minimal mode (flag clear) is the fallback. Future settings or per-frame toggle clears the flag to collapse the top bar.

### Top bar height constant

```rust
/// Height of the top bar chrome band in default mode (replaces top rim).
const FRAME_TOP_BAR_HEIGHT_PX: i32 = 16;
```

**Why 16px:**
- Accommodates 8px-tall lights with 4px top/bottom padding (y=4..12)
- Leaves ~8px for tab strip content adjacent to lights
- Consistent with typical thin titlebar height (Apple OS X traffic light style)
- Even multiple of pixels for alignment

### Default Mode Top Bar Layout (y=0..16 relative to surface top)

```
┌──────────────────────────────────────────────────┐ sy..sy+16  (top bar band)
│ ←lights→  ←tab strip→                            │
│  ⬤⬤⬤    [tab 0] [tab 1]                         │
├──────────────────────────────────────────────────┤ sy+16..sy+sh-4  (content area)
│                                                    │
│ Surface content area                               │
│  (solid color + optional fill rect)                │
│                                                    │
└──────────────────────────────────────────────────┘ sy+sh-4..sy+sh (bottom rim)
```

| Element | Y-range | X-range |
|---------|---------|---------|
| Top bar background | sy..sy+16 | sx..sx+sw |
| Lights (8px tall) | sy+4..sy+12 | sx+4..sx+36 |
| Tab strip | sy..sy+16 | sx+40..sx+sw-4 |
| Left rim (4px) | sy..sy+sh | sx..sx+4 |
| Right rim (4px) | sy..sy+sh | sx+sw-4..sx+sw |
| Bottom rim (4px) | sy+sh-4..sy+sh | sx..sx+sw |
| Content area | sy+16..sy+sh-4 | sx+4..sx+sw-4 |

### Light Geometry in Default Mode

```rust
// Default mode: lights are 8x8px, centered in top bar, with 4px gaps.
const FRAME_TOP_BAR_LIGHT_SIZE_PX: i32 = 8;
const FRAME_TOP_BAR_LIGHT_GAP_PX: i32 = 4;

// CLOSE:  sx+4..sx+12,  sy+4..sy+12
// MINIMIZE: sx+16..sx+24, sy+4..sy+12
// ZOOM:    sx+28..sx+36, sy+4..sy+12
// Tab strip start: sx+40
```

### Tab Strip in Default Mode

The tab strip spans the full top bar height (16px) but is limited to equal-width colored blocks (same logic as current, just taller y-range). The `FRAME_TAB_LIGHT_EXCLUSION_PX` grows from 20 to 40 to account for larger lights + gaps.

```rust
// In default mode, the light exclusion zone is wider because lights are bigger.
const FRAME_TOP_BAR_LIGHT_EXCLUSION_PX: i32 = 40;
// (was FRAME_TAB_LIGHT_EXCLUSION_PX = 20 in minimal mode)
```

### Minimal Mode (Top Bar OFF)

Unchanged from current behavior:
- 4px rim on all edges
- Lights within top 4px rim band (4x4px squares)
- Tab strip within top 4px rim band
- All existing constants (`FRAME_RIM_PX`, `FRAME_TAB_STRIP_PX`, etc.) remain valid

---

## Helper: `frame_is_top_bar()`

```rust
/// Returns true if the given frame has the top bar enabled (default mode).
unsafe fn frame_is_top_bar(frame_id: u32) -> bool {
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                return (frame.flags & FRAME_FLAG_TOP_BAR) != 0;
            }
        }
    }
    false
}
```

---

## Hit Target Model Changes

### `frame_light_at()` — y-range depends on top bar mode

```rust
unsafe fn frame_light_at(frame_id: u32, x: i32, y: i32) -> u32 {
    let surface_id = active_surface_for_frame(frame_id)?;
    let (sx, sy, _sw, _sh) = get_surface_bounds(surface_id)?;
    
    // Choose appropriate height: top bar height or rim height.
    let chrome_h = if frame_is_top_bar(frame_id) {
        FRAME_TOP_BAR_HEIGHT_PX
    } else {
        FRAME_RIM_PX
    };
    let top_band_bottom = sy + chrome_h;
    if y < sy || y >= top_band_bottom {
        return FRAME_LIGHT_NONE;
    }
    
    let lx = x - sx;
    if frame_is_top_bar(frame_id) {
        // Default mode: 8px lights at 4px gaps.
        if lx >= FRAME_TOP_BAR_LIGHT_GAP_PX
            && lx < FRAME_TOP_BAR_LIGHT_GAP_PX + FRAME_TOP_BAR_LIGHT_SIZE_PX {
            return FRAME_LIGHT_CLOSE;
        }
        let l2_start = FRAME_TOP_BAR_LIGHT_GAP_PX + FRAME_TOP_BAR_LIGHT_SIZE_PX + FRAME_TOP_BAR_LIGHT_GAP_PX;
        if lx >= l2_start && lx < l2_start + FRAME_TOP_BAR_LIGHT_SIZE_PX {
            return FRAME_LIGHT_MINIMIZE;
        }
        let l3_start = l2_start + FRAME_TOP_BAR_LIGHT_SIZE_PX + FRAME_TOP_BAR_LIGHT_GAP_PX;
        if lx >= l3_start && lx < l3_start + FRAME_TOP_BAR_LIGHT_SIZE_PX {
            return FRAME_LIGHT_ZOOM;
        }
    } else {
        // Minimal mode: current 4px light geometry (unchanged).
        if lx >= FRAME_LIGHT_GAP_PX && lx < FRAME_LIGHT_GAP_PX + FRAME_LIGHT_SIZE_PX {
            return FRAME_LIGHT_CLOSE;
        }
        // ... MINIMIZE, ZOOM checks unchanged ...
    }
    FRAME_LIGHT_NONE
}
```

### `frame_tab_at()` — y-range depends on top bar mode

```rust
unsafe fn frame_tab_at(frame_id: u32, x: i32, y: i32) -> Option<u32> {
    let surface_id = active_surface_for_frame(frame_id)?;
    let bounds = get_surface_bounds(surface_id)?;
    let (sx, sy, sw, _sh) = bounds;
    
    // Choose y-range: top bar height or tab strip height.
    let strip_h = if frame_is_top_bar(frame_id) {
        FRAME_TOP_BAR_HEIGHT_PX
    } else {
        FRAME_TAB_STRIP_PX
    };
    if y < sy || y >= sy + strip_h {
        return None;
    }
    
    // Choose exclusion zone: larger in default mode (bigger lights).
    let exclusion = if frame_is_top_bar(frame_id) {
        FRAME_TOP_BAR_LIGHT_EXCLUSION_PX
    } else {
        FRAME_TAB_LIGHT_EXCLUSION_PX
    };
    
    let tab_strip_start = sx + exclusion;
    if x < tab_strip_start { return None; }
    
    let right_rim_start = sx + sw as i32 - FRAME_RIM_PX;
    if x >= right_rim_start { return None; }
    
    // Equal-width tab slot computation (unchanged).
    let tab_count = frame_tab_count(frame_id);
    if tab_count == 0 { return None; }
    let available_width = (right_rim_start - tab_strip_start).max(0);
    if available_width < FRAME_TAB_MIN_WIDTH_PX { return Some(0); }
    let slot_w = available_width / tab_count as i32;
    let lx = x - tab_strip_start;
    let tab_index = (lx / slot_w.max(1)).min(tab_count as i32 - 1);
    Some(tab_index as u32)
}
```

### `hit_test_surface_chrome()` — top edge rim vs top bar

Current behavior: top edge IS rim (draggable). Default mode: top edge is top bar zone (not draggable — it has lights + tabs).

```rust
unsafe fn hit_test_surface_chrome(x: i32, y: i32, sid: u64) -> Option<HitTarget> {
    let (sx, sy, sw, sh) = get_surface_bounds(sid)?;
    let frame_id = frame_for_surface(sid)?;
    
    let top_bar_active = frame_is_top_bar(frame_id);
    let chrome_top_h = if top_bar_active {
        FRAME_TOP_BAR_HEIGHT_PX  // top bar zone replaces top rim
    } else {
        FRAME_RIM_PX             // normal top rim
    };
    
    // Tab strip check: y-range depends on mode.
    if FRAME_TAB_STRIP_PX > 0 {
        let strip_bot = sy + if top_bar_active {
            FRAME_TOP_BAR_HEIGHT_PX
        } else {
            FRAME_TAB_STRIP_PX
        };
        let exclusion = if top_bar_active {
            FRAME_TOP_BAR_LIGHT_EXCLUSION_PX
        } else {
            FRAME_TAB_LIGHT_EXCLUSION_PX
        };
        if y >= sy && y < strip_bot
            && x >= sx + exclusion
            && x < sx + sw as i32 - FRAME_RIM_PX
        {
            if frame_tab_at(frame_id, x, y).is_some() {
                return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_TAB_STRIP });
            }
        }
    }
    
    // Rim check: top edge condition depends on mode.
    let right = sx + sw as i32 - 1;
    let bottom = sy + sh as i32 - 1;
    let in_rim =
        (x >= sx && x < sx + FRAME_RIM_PX)                                  // left edge (always)
        || (x > right - FRAME_RIM_PX && x <= right)                         // right edge (always)
        || (y >= sy && y < sy + chrome_top_h)                              // top edge (or top bar zone)
        || (y > bottom - FRAME_RIM_PX && y <= bottom);                      // bottom edge (always)
    if in_rim {
        return Some(HitTarget::FrameChrome { frame_id, kind: FRAME_CHROME_RIM });
    }
    
    None
}
```

**Important:** In default mode, the top edge rim check (`y >= sy && y < sy + chrome_top_h`) captures the 16px top bar as rim for drag purposes. But the lights and tab strip checks happen BEFORE the rim check (in `click_hit_test_and_focus()`, lights are checked first within the `FRAME_CHROME_RIM` arm). So the effective priority is:

1. Lights (checked first in click handler)
2. Tab strip (checked second, as `FRAME_CHROME_TAB_STRIP`)
3. Rim drag (only non-light, non-tab positions in the chrome zone)

This means clicking on the top bar zone but NOT on a light or tab block would start a rim drag in default mode. This is acceptable for V1 — the top bar zone acts like rim for residual clicks.

---

## Render/Protocol Needs

### Sexdisplay must know chrome mode

Sexdisplay's `composite_pixel()` renders the rim/lights/tab strip using local surface coordinates. To render the top bar correctly, sexdisplay needs to know whether the surface is in default mode or minimal mode.

**Deferred to `FRAME_TOP_BAR_RENDER_PLAN_V1`:**

1. **Protocol:** Either extend `OP_SURFACE_TAB_INFO` (0xFD) with a `chrome_mode` field, or add a new opcode `OP_SURFACE_CHROME_MODE` (e.g., 0xFC). Option: repurpose 0xFD arg2 as bitfield `(active_tab & 0x7F) | (chrome_mode << 7)` — but this is ugly. A separate opcode is cleaner.

2. **Surface field:** Add `chrome_mode: u8` to Surface struct (0=minimal, 1=default/top bar).

3. **Rendering in composite_pixel():**
   - If `chrome_mode == 1` (default):
     - Top 16px = top bar background color (rim color or slightly different)
     - Lights rendered at 8x8px with 4px gaps
     - Tab strip rendered at full 16px height
     - Left/right/bottom rim = 4px (unchanged)
   - If `chrome_mode == 0` (minimal):
     - Current 4px rim + lights + tab strip behavior (unchanged)

4. **Shell send:** `send_frame_tab_info()` extended to also send chrome mode, or a separate send function called at boot and on mode toggle.

### Model-to-Render Phase Boundary

| Phase | Shell Changes | Sexdisplay Changes |
|-------|--------------|-------------------|
| FRAME_TOP_BAR_MODEL_V1 (THIS doc) | Constants, flag, helpers, hit targets | None |
| FRAME_TOP_BAR_RENDER_PLAN_V1 | Protocol design (which opcode, payload) | Protocol design, rendering design |
| FRAME_TOP_BAR_RENDER_V1 | Send chrome mode to sexdisplay | New opcode handler, composite_pixel() rendering |

---

## Interaction with Close/Minimize/Zoom

| Action | Default Mode (top bar ON) | Minimal Mode (top bar OFF) |
|--------|--------------------------|---------------------------|
| **CLOSE** | Lights in top bar at larger geometry. Same action. | Lights in 4px rim. Same action. |
| **MINIMIZE** | Same. Frame flag unaffected by top bar mode. | Same. |
| **ZOOM** | Same. Zoom geometry unaffected by top bar height. | Same. |
| **Rim drag** | Left/right/bottom edges only. Top bar zone not draggable (lights + tabs). | All 4 edges draggable. |
| **Tab switch** | Tab strip in taller top bar zone. Same switch algorithm. | Tab strip in 4px top rim. Same switch algorithm. |

---

## Toggle Mechanism (Future)

**Not implemented in V1.** The model provides the flags and constants, but no user-facing toggle. Future options:

- **Settings app toggle:** Scene/Theme settings app sends opcode to toggle top bar per-frame or globally.
- **Keyboard shortcut:** e.g., `Ctrl+Shift+T` toggles top bar on focused frame.
- **Per-frame double-click:** Double-clicking the top bar zone toggles to minimal mode (future gesture).

For V1, the boot default is top bar ON. To test minimal mode, change the boot init to `flags: 0` instead of `flags: FRAME_FLAG_TOP_BAR`.

---

## Files Changed (in FRAME_TOP_BAR_MODEL_V1)

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Add `FRAME_FLAG_TOP_BAR = 1 << 2` constant. Add `FRAME_TOP_BAR_HEIGHT_PX = 16`, `FRAME_TOP_BAR_LIGHT_SIZE_PX = 8`, `FRAME_TOP_BAR_LIGHT_GAP_PX = 4`, `FRAME_TOP_BAR_LIGHT_EXCLUSION_PX = 40` constants. Add `frame_is_top_bar()` helper. Update `frame_light_at()` to dispatch on top bar mode. Update `frame_tab_at()` to use correct y-range and exclusion. Update `hit_test_surface_chrome()` to replace top edge rim with top bar zone when flag set. Set `FRAMES[0].flags = FRAME_FLAG_TOP_BAR` at boot init. Add `[shell.frame.top_bar.model]` boot proof marker. |

### Files NOT Modified

- `kernel/` — no kernel ABI changes
- `crates/sex-pdx/src/lib.rs` — no protocol changes in model phase
- `servers/sexdisplay/src/main.rs` — no renderer changes in model phase
- `servers/silkbar/` — no forwarding changes
- `crates/silkbar-model/` — no model changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

---

## Proof Markers

### New marker

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.top_bar.model] frame=N height=N` | 1 | Boot: proves top bar constants, flag, and helper exist |

### Pre-existing markers that must still fire

| Marker | Status |
|--------|--------|
| `[shell.frame.light.model]` | Lights model proof (boot) |
| `[shell.frame.tab.model]` | Tab strip model proof (boot) |
| `[shell.frame.light.hover]` | Light hover detection |
| `[shell.frame.light.close/minimize/zoom]` | Light actions still work |
| `[shell.frame.tab.switch]` | Tab switching still works |
| `[shell.frame.tab.info.send]` | Tab metadata sent to sexdisplay |
| `[shell.drag.start/move/end]` | Rim drag still works on non-top edges |
| `[shell.focus.set]` | Focus changes |
| `[shell.hit_target.chrome]` | Chrome hit targets produced |

---

## STOP Conditions

1. **Sexdisplay rendering mismatch** — Model phase defines hit targets for 16px top bar, but sexdisplay still renders 4px rim. This creates a temporary hit-test/visual inconsistency (clicks on y=4..16 register as chrome hits, but visually appear as content area). This is ACCEPTABLE for a model-only phase — the rendering is updated in FRAME_TOP_BAR_RENDER_V1. Document this in the commit message.

2. **Top bar overlaps with surface content** — In default mode, the surface's visible content area shrinks from being below y=4 to below y=16 (12px less). This is a visual change but requires no geometry changes — the compositor controls what's chrome vs content within the surface bounds.

3. **FRAME_TAB_STRIP_PX = 4 conflicts with top bar height** — The tab strip uses `FRAME_TAB_STRIP_PX = 4` for y-range checks in minimal mode, but needs the full top bar height in default mode. Solution: `frame_tab_at()` and `hit_test_surface_chrome()` branch on `frame_is_top_bar()` — using `FRAME_TOP_BAR_HEIGHT_PX` when flag set, `FRAME_TAB_STRIP_PX` when clear.

4. **Lights exclusion zone too small in default mode** — Current `FRAME_TAB_LIGHT_EXCLUSION_PX = 20` covers 4px lights + 2px gaps = 18px + 2px margin = 20px. In default mode with 8px lights + 4px gaps, 3 lights need: 4 + 8 + 4 + 8 + 4 + 8 + 4 = 40px. Solution: `FRAME_TOP_BAR_LIGHT_EXCLUSION_PX = 40` for default mode.

5. **Rim drag on top edge in default mode** — In default mode, the top edge (y=0..16) is the top bar zone, not rim. But `hit_test_surface_chrome()` still returns `FRAME_CHROME_RIM` for these positions if no light or tab is hit. This means clicks on the top bar background (empty area) start a rim drag. This is acceptable for V1 — the top bar zone behaves like rim for residual clicks, which is similar to the current behavior.

6. **Focus/zoom/minimize unaffected by top bar mode** — These operations use `active_surface_for_frame()` and `ShellFrame.flags`. The top bar flag does not interact with these.

---

## Next Phase

### FRAME_TOP_BAR_MODEL_V1

```
MISSION: FRAME_TOP_BAR_MODEL_V1.

Implement the shell-side top bar model: constants, flag, helper, hit target updates,
and boot init. No renderer or protocol changes.

Design complete in FRAME_TOP_BAR_MODEL_PLAN_V1.md.

Changes:

1. servers/silk-shell/src/main.rs:
   a. Add constants:
      - FRAME_FLAG_TOP_BAR = 1 << 2
      - FRAME_TOP_BAR_HEIGHT_PX = 16
      - FRAME_TOP_BAR_LIGHT_SIZE_PX = 8
      - FRAME_TOP_BAR_LIGHT_GAP_PX = 4
      - FRAME_TOP_BAR_LIGHT_EXCLUSION_PX = 40
   b. Add frame_is_top_bar() helper
   c. Update frame_light_at():
      - If top bar: use FRAME_TOP_BAR_HEIGHT_PX for y-range
      - If top bar: use FRAME_TOP_BAR_LIGHT_SIZE/FRAME_TOP_BAR_LIGHT_GAP for geometry
      - If minimal: current behavior (unchanged)
   d. Update frame_tab_at():
      - If top bar: use FRAME_TOP_BAR_HEIGHT_PX for y-range
      - If top bar: use FRAME_TOP_BAR_LIGHT_EXCLUSION_PX for exclusion
      - If minimal: current behavior (unchanged)
   e. Update hit_test_surface_chrome():
      - If top bar: replace top edge rim check with top bar zone height
      - If top bar: tab strip y-range uses FRAME_TOP_BAR_HEIGHT_PX
      - If minimal: current behavior (unchanged)
   f. Set FRAMES[0].flags = FRAME_FLAG_TOP_BAR in boot init
   g. Add [shell.frame.top_bar.model] boot proof marker

Forbidden:
- Sexdisplay changes
- Opcode/protocol changes
- Kernel edits
- Text rendering
- Top bar rendering (deferred to FRAME_TOP_BAR_RENDER_V1)
- Settings integration (deferred)
- Toggle mechanism (deferred)

PASS:
- Default build passes
- [shell.frame.top_bar.model] fires at boot with frame=1 height=16
- [shell.frame.light.model] still fires (unchanged)
- [shell.frame.tab.model] still fires (unchanged)
- frame_is_top_bar(1) returns true for default boot
- frame_light_at() returns correct light kinds within 16px top bar zone
- frame_tab_at() returns correct tab indices within 16px top bar zone
- Lights still fire CLOSE/MINIMIZE/ZOOM actions via click
- Tab strip still triggers tab switch via click
- Rim drag works on left/right/bottom edges
- No panic/#PF/#GP
- No kernel edits confirmed

KNOWN ISSUE (documented):
- Visual mismatch: sexdisplay still renders 4px rim while hit targets expect 16px top bar.
  Clicks on y=4..16 are handled by chrome hit targets (lights/tabs/rim) but visually
  appear as content background. This is resolved in FRAME_TOP_BAR_RENDER_V1.
```
