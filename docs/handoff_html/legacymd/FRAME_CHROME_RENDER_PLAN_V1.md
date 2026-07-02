# FRAME_CHROME_RENDER_PLAN_V1

## Status

Design (2026-05-04). No code changed. Audit of sexdisplay render pipeline and silk-shell→display
ABI shows neon rim is implementable **without protocol changes** in `composite_pixel()`.

---

## Current Render Pipeline

```
silk-shell                     sexdisplay
    │                              │
    │ 0xEC surface_create(id,x,y)  │  → SURFACES[slot] = {id,x,y,w,h,color,active}
    │ 0xEB surface_update(id,x,y)  │  → SURFACES[slot].{x,y} = new position
    │ 0xED set_focus(id)           │  → FOCUSED_SURFACE_ID = id + redraw_surface_area()
    │ 0xEE surface_destroy(id)     │  → slot.active = false
    │ 0xEF surface_fill_rect(id)   │  → slot.fill_{sx,sy,sw,sh,color} = rect
    │                              │
    │              render(fb,w,h,bar) every event loop:
    │                1. composite_pixel(x,y,w,h,bg,focused_id)
    │                   Pass 1: non-focused surfaces (slot order, break on first hit)
    │                   Pass 2: focused surface (always on top, FOCUS_SURFACE_COLOR)
    │                2. draw_cursor_z_top()
    │                3. draw_launcher_panel()
```

### Surface struct (sexdisplay line 28-44)

```rust
struct Surface {
    surface_id: u64, owner_pd: u32,
    x: i32, y: i32, w: u32, h: u32,
    color: u32, active: bool,
    fill_sx: i32, fill_sy: i32, fill_sw: u32, fill_sh: u32,
    fill_color: u32, fill_active: bool,
}
```

### composite_pixel (sexdisplay line 80-107)

Per-pixel compositing:
- Pass 1: iterate SURFACES, skip focused, `clamp_surface()`, check bounds → `fill_rect_color()`
- Pass 2: if focused_id != 0, find focused surface, `clamp_surface()`, check bounds → `FOCUS_SURFACE_COLOR`

### Clamp (sexdisplay line 67-76)

```rust
fn clamp_surface(surf, fb_w, fb_h) -> (x, y, w, h):
    x = surf.x.max(0).min(fb_w-1)
    y = surf.y.max(BAR_H).min(fb_h-1)
    w = surf.w.min(fb_w - x)
    h = surf.h.min(fb_h - y)
```

Ensures all surface writes stay within framebuffer bounds and below the top strip (BAR_H=50).

---

## V1 Render Target: Neon Rim Around Focused Surface

### Feasibility: ✅ Implementable without ABI change

Sexdisplay already has all information needed:
| Needed | Available in sexdisplay | Source |
|--------|------------------------|--------|
| Surface bounds (x,y,w,h) | ✅ SURFACES[slot] | 0xEC create, 0xEB update |
| Which surface is focused | ✅ FOCUSED_SURFACE_ID | 0xED set_focus |
| Clamped pixel position | ✅ clamp_surface() result | Computed per-pixel |

### Implementation approach (sexdisplay only)

In `composite_pixel()`, Pass 2 (focused surface), after confirming pixel is within bounds:

```rust
// existing pass 2
if x >= sx && x < sx + sw && y >= sy && y < sy + sh {
    // NEW: check 4px rim band
    let lx = x.saturating_sub(sx);  // local x within clamped surface
    let ly = y.saturating_sub(sy);  // local y within clamped surface
    if lx < RIM_PX || lx >= sw - RIM_PX || ly < RIM_PX || ly >= sh - RIM_PX {
        c = RIM_COLOR;         // neon rim edge band
    } else {
        c = fill_rect_color(...); // existing surface content color
    }
    break;
}
```

Key details:
- `RIM_PX = 4` (matches `FRAME_RIM_PX` in silk-shell)
- `RIM_COLOR = 0x00C0F0FF` (bright neon cyan-white, visually distinct from `FOCUS_SURFACE_COLOR=0x00A8E0FF`)
- Rim is drawn on ALL sides of the focused surface, including top (below BAR_H) and bottom
- Draws on top of the surface's fill rect (rim always visible at edges)
- `saturating_sub` prevents underflow for tiny surfaces

### Bounds safety

`clamp_surface()` is already called before the rim check. All pixel writes go through the existing
bounds-checked path (`sx, sy, sw, sh` are clamped to framebuffer + BAR_H). The rim check is relative
to the clamped dimensions, so it is inherently bounds-safe.

### Edge cases

| Case | Behavior |
|------|----------|
| Surface < 8px in one dimension | Rim band may overlap or cover surface. `saturating_sub` prevents underflow. Surface would be all rim. |
| Surface at screen edge | Rim clipped by clamp_surface, same as surface content. |
| Multiple frames (V2+) | Only focused surface gets rim. Non-focused frame surfaces render with normal color. |
| Fullscreen surface | Rim still drawn at edges. Visible 4px border. |

---

## Hover-Reveal Tab Label: ❌ NOT Feasible in V1

| Requirement | Available? |
|-------------|-----------|
| General text rendering | ❌ Only 5×7 clock digit font (0-9, 7 rows, 5 wide each) |
| Label string storage | ❌ No string/buffer for tab title text |
| Hover state protocol | ❌ Hover is tracked only in silk-shell, never sent to sexdisplay |
| Label positioning | ❌ No overlay/annotation model for surface area |

### Blockers

1. **No text rendering pipeline** — `clock_fg_at()` renders 5×7 digit bitmaps using hardcoded
   `FONT[0..9]` array. Would need full ASCII font, glyph dimensions, and a text layout engine.
2. **No hover IPC** — `HOVERED_FRAME_ID` / `HOVER_KIND` are static mut in silk-shell, logged
   via serial_println only. No opcode exists to forward hover to sexdisplay.
3. **No label storage in sexdisplay** — Surface struct has no title/name field. ShellFrame::tabs[].title_id
   exists in silk-shell but is never transmitted.

### Recommendation

Defer hover-reveal labels to a dedicated text rendering phase. Three prerequisites:
1. Add a general glyph bitmap array (ASCII printable, at minimum)
2. Add a text render helper to sexdisplay
3. Add a hover-report opcode or include label in surface metadata

---

## Implementation Plan (FRAME_CHROME_RENDER_V1)

### Allowed files
- `servers/sexdisplay/src/main.rs` — only `composite_pixel()` + constants

### Forbidden
- `kernel/`, `crates/sex-pdx/` — no kernel/ABI edits
- `servers/silk-shell/` — no shell changes in this phase
- `servers/sexusb/`, `servers/sexinput/` — no input changes
- Any ABI/opcode changes
- Framebuffer path rewrite
- Surface storage changes
- Adding new fields to Surface struct

### Exact changes needed

#### 1. Add rim constants (near FOCUS_SURFACE_COLOR, line 53)

```rust
/// Thickness of the neon rim edge band in pixels (matches FRAME_RIM_PX in silk-shell).
const RIM_PX: usize = 4;
/// Color of the neon rim around the focused frame surface.
const RIM_COLOR: u32 = 0x00C0F0FF;
```

#### 2. Modify composite_pixel Pass 2 (lines 93-104)

Insert rim check before surface color assignment. Only touched lines: the focused surface
color assignment (line 100). No structural changes.

#### 3. Update redraw_surface_area (no change needed)

Already calls `composite_pixel()` for every pixel. Rim will automatically render in both
full renders (`render()`) and focus-change redraws (`redraw_surface_area()`).

### No new markers needed

The existing render pipeline already produces:
- `[sexdisplay.cursor.surface.update]` — cursor tracking
- No diagnostic needed for rim rendering (it's always-on per-pixel)

For proof, a budgeted marker in composite_pixel Pass 2 could be added to confirm rim
production, max 4 emissions.

---

## Verification Plan

```bash
./scripts/entrypoint_build.sh
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both must pass with no new warnings.

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-chrome-render-v1.log
```

Visual verification (no automated pixel test in V1):
- Focused surface should show a 4px neon rim around all edges
- Rim color should be distinct from surface fill color
- Rim should stay with the surface when it moves (drag)
- Non-focused surfaces should have no rim
- All existing functional markers should pass

---

## Next Implementation Prompt

The next phase is **FRAME_CHROME_RENDER_V1** — implement the neon rim in sexdisplay's
`composite_pixel()` only. Exact task:

```
MISSION: FRAME_CHROME_RENDER_V1

IMPLEMENTATION ONLY. Design complete.

Files to modify:
- servers/sexdisplay/src/main.rs

Changes:
1. Add RIM_PX = 4 and RIM_COLOR = 0x00C0F0FF constants near FOCUS_SURFACE_COLOR
2. In composite_pixel() Pass 2, add rim band check before assigning surface color
3. Verify both builds pass
4. Test with real mouse: focused surface should show neon rim at edges

Forbidden:
- Any ABI/opcode change
- Any silk-shell change
- Any framebuffer bounds removal
- Any Surface struct field addition
- Any renderer refactor

Pass criteria:
- Default build passes
- Synthetic build passes
- Neon rim visible around focused surface
- No rim on non-focused surfaces
- Rim moves with surface during drag
- No new warnings
```

---

## Decision Summary

| Question | Answer |
|----------|--------|
| Can sexdisplay draw rim without ABI change? | ✅ **Yes** — surface bounds + focus state already known |
| Can hover-reveal labels be drawn? | ❌ **No** — no text pipeline, no hover IPC, no label storage |
| Is a protocol/ABI change needed for V1? | **No** — zero ABI changes |
| What is the safest V1 render target? | **Display-only neon rim** around focused surface via modified composite_pixel |
| Next phase name | **FRAME_CHROME_RENDER_V1** (implementation) |
