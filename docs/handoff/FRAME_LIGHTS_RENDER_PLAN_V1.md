# FRAME_LIGHTS_RENDER_PLAN_V1

## Status

Design (2026-05-04). No code changed. Audit of sexdisplay render pipeline shows Frame Lights are renderable **without protocol changes** as part of `composite_pixel()` Pass 2, alongside the neon rim (which also needs implementation).

**Important finding:** FRAME_CHROME_RENDER_V1 (neon rim) was designed but **never implemented** — `RIM_PX` and `RIM_COLOR` do not exist in sexdisplay. Frame Lights rendering must be implemented **together with** the neon rim in a combined phase (`FRAME_CHROME_RENDER_V1`), or the rim must be added first.

---

## Current Render Pipeline (simplified)

```
render(fb, w, h, bar):
  for y in 0..h:
    for x in 0..w:
      if y < 50:  bar_color(x, y, bar)           ← SilkBar (top strip)
      else:       composite_pixel(x, y, ...)      ← surfaces + cursor
  draw_cursor_z_top(fb, ...)
  draw_launcher_panel(fb, ...)
```

### composite_pixel (sexdisplay:80-107)

```
composite_pixel(x, y, w, h, bg, focused_id) → u32:
  Pass 1 (non-focused surfaces):
    for each surface != focused:
      clamp → (sx, sy, sw, sh), check bounds → fill_rect_color
  Pass 2 (focused surface):
    if focused_id != 0:
      find focused surface, clamp → (sx, sy, sw, sh)
      if (x, y) within bounds:
        return fill_rect_color(surf, x, y, FOCUS_SURFACE_COLOR)
```

### Key state in sexdisplay

| State | Source | Description |
|-------|--------|-------------|
| `SURFACES[slot].{x,y,w,h}` | 0xEC create / 0xEB update | Surface position and size |
| `FOCUSED_SURFACE_ID` | 0xED set_focus | Currently focused surface |
| `clamp_surface()` | Computed | Bounds-safe (x,y,w,h) within FB |

---

## V1 Render Target: Frame Lights in Top Rim Band of Focused Surface

### Feasibility: ✅ Implementable without ABI change

| Needed | Available in sexdisplay | Source |
|--------|------------------------|--------|
| Surface bounds | ✅ `SURFACES[slot].{x,y,w,h}` | 0xEC create / 0xEB update |
| Focused surface | ✅ `FOCUSED_SURFACE_ID` | 0xED set_focus |
| Per-pixel position | ✅ `(x, y)` passed to `composite_pixel()` | Framebuffer loop |
| Light geometry | ✅ Matches shell `FRAME_LIGHT_SIZE_PX`/`GAP_PX` | Add constants |
| Light colors | ✅ Hardcoded in sexdisplay | Add constants |

---

## Design: Combined Rim + Lights Rendering

The neon rim and frame lights are **always drawn on the focused surface** in `composite_pixel()` Pass 2. No hover IPC needed for V1 — hover highlighting is a future phase.

### Priority ordering (per-pixel, Pass 2):

```
1. If pixel is in a Frame Light region (top-left rim band) → LIGHT_COLOR
2. Else if pixel is in rim band (any edge)             → RIM_COLOR
3. Else                                                 → fill_rect_color (surface content)
```

### Constants to add (near FOCUS_SURFACE_COLOR, line 53)

```rust
// ── Frame Chrome Rim (focused surface) ──
/// Thickness of the neon rim edge band in pixels (matches shell FRAME_RIM_PX).
const RIM_PX: usize = 4;
/// Color of the neon rim around the focused frame surface.
const RIM_COLOR: u32 = 0x00C0F0FF;  // bright neon cyan-white

// ── Frame Light Colors (top-left rim band) ──
/// Red close light.
const LIGHT_CLOSE_COLOR: u32 = 0x00FF4444;
/// Yellow minimize/collapse light.
const LIGHT_MINIMIZE_COLOR: u32 = 0x00FFCC44;
/// Green zoom/maximize light.
const LIGHT_ZOOM_COLOR: u32 = 0x0044FF44;

// ── Frame Light Geometry (matches shell FRAME_LIGHT_SIZE_PX/GAP_PX) ──
/// Width and height of each frame light square in pixels.
const LIGHT_SIZE_PX: usize = 4;
/// Gap between adjacent frame lights in pixels.
const LIGHT_GAP_PX: usize = 2;
```

### Modified composite_pixel Pass 2 (pseudocode)

```rust
// Pass 2: focused surface (always on top)
if focused_id != 0 {
    for surf in SURFACES.iter() {
        if !surf.active || surf.surface_id != focused_id { continue; }
        let (sx, sy, sw, sh) = clamp_surface(surf, w, h);
        if sw == 0 || sh == 0 { continue; }
        if x >= sx && x < sx + sw && y >= sy && y < sy + sh {
            // Local coordinates within the surface
            let lx = x - sx;
            let ly = y - sy;

            // Rim check: pixel is within 4px of any edge
            if ly < RIM_PX || lx < RIM_PX
                || lx >= sw - RIM_PX || ly >= sh - RIM_PX
            {
                // Frame Light check: top rim band, light horizontal ranges
                if ly < RIM_PX {
                    let light_x = lx;
                    // CLOSE
                    if light_x >= LIGHT_GAP_PX
                        && light_x < LIGHT_GAP_PX + LIGHT_SIZE_PX
                    {
                        c = LIGHT_CLOSE_COLOR;
                    }
                    // MINIMIZE
                    else if light_x >= LIGHT_GAP_PX + LIGHT_SIZE_PX + LIGHT_GAP_PX
                        && light_x < LIGHT_GAP_PX + LIGHT_SIZE_PX + LIGHT_GAP_PX + LIGHT_SIZE_PX
                    {
                        c = LIGHT_MINIMIZE_COLOR;
                    }
                    // ZOOM
                    else if light_x >= LIGHT_GAP_PX + 2 * (LIGHT_SIZE_PX + LIGHT_GAP_PX)
                        && light_x < LIGHT_GAP_PX + 2 * (LIGHT_SIZE_PX + LIGHT_GAP_PX) + LIGHT_SIZE_PX
                    {
                        c = LIGHT_ZOOM_COLOR;
                    } else {
                        c = RIM_COLOR; // top rim, not on a light
                    }
                } else {
                    c = RIM_COLOR; // side or bottom rim
                }
            } else {
                c = fill_rect_color(surf, x, y, FOCUS_SURFACE_COLOR);
            }
            break;
        }
    }
}
```

### Light region coordinates (in surface-local pixels)

```
ly=0  ┌──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬──┬...
      │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │
ly=1  │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │  ← RIM_PX=4
ly=2  │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │
ly=3  │CL│CL│CL│CL│  │MI│MI│MI│MI│  │ZO│ZO│ZO│ZO│  │  │
ly=4  └──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──┴──... ← beyond rim → RIM_COLOR
      ↑  ←4px→  ←2→  ←4px→  ←2→  ←4px→
      sx+0    CLOSE  gap  MINIMIZE gap ZOOM
```

### Edge cases

| Case | Behavior |
|------|----------|
| Surface width < 18px | Lights may be clipped by right edge. `lx < sw` check in bounds means partial lights render naturally. Lights near right edge silently omitted. |
| Surface height < 4px | Top rim band may be empty. `ly < RIM_PX` with `sh < 4` means no rim/lights. |
| Surface at screen left edge | Lights at x=0..18 are at screen edge, rendered normally. |
| Surface moved below BAR_H | `clamp_surface()` enforces `y >= 50`, rim/lights render correctly. |
| No focused surface | Pass 2 skipped entirely — no rim, no lights. |
| Fullscreen surface | Rim/lights still drawn at edges. |

---

## Bounds Safety

All pixel writes go through the existing bounds-safe path:
1. `clamp_surface()` truncates surface to framebuffer dimensions (respecting BAR_H)
2. The rim/light check is relative to clamped `(sx, sy, sw, sh)`
3. No new framebuffer writes — the existing `write_volatile` path is unchanged
4. `lx = x - sx` is safe because `x >= sx` is confirmed by the bounds check immediately above
5. `ly = y - sy` is safe for the same reason
6. `sw - RIM_PX` uses `usize` subtraction — if `sw < RIM_PX`, underflow wraps to large usize, causing the rim check to span the entire surface (correct behavior for tiny surfaces)

Actually, wait — `sw` is `usize` and `RIM_PX` is `usize`. If `sw < RIM_PX`, `sw - RIM_PX` will panic in debug or wrap in release. I need to note this as a safety concern for the implementation phase. The fix is to use `saturating_sub` as designed in FRAME_CHROME_RENDER_PLAN_V1.

---

## Decision: Combine with Neon Rim or Separate?

**Recommendation: Implement Frame Lights and neon rim together in one phase.**

Rationale:
1. Both modify the same 8 lines of Pass 2 in `composite_pixel()`
2. Splitting would require implementing the rim first, then immediately touching the same code again
3. Both are purely visual, no behavioral changes
4. Both use the same bounds/safety analysis

If implemented separately:
- FRAME_CHROME_RENDER_V1 adds rim → composite_pixel Pass 2 grows rim logic
- FRAME_LIGHTS_RENDER_V1 adds lights → composite_pixel Pass 2 grows light logic inside rim check
- Two phases touching the exact same function, same lines

---

## Implementation Plan

### Allowed files
- `servers/sexdisplay/src/main.rs` only (composite_pixel + constants)

### Forbidden
- `kernel/`, `crates/sex-pdx/` — no kernel/ABI edits
- `servers/silk-shell/` — no shell changes
- `servers/sexusb/`, `servers/sexinput/` — no input changes
- `crates/silkbar-model/`, `servers/silkbar/` — no bar/protocol changes
- Any ABI/opcode changes
- Framebuffer path rewrite
- Surface storage changes
- Adding new fields to Surface struct
- Action behavior (close/minimize/zoom)
- Hover IPC

### Exact changes needed

#### 1. Add constants (after `FOCUS_SURFACE_COLOR`, line 53)

```rust
const RIM_PX: usize = 4;
const RIM_COLOR: u32 = 0x00C0F0FF;
const LIGHT_CLOSE_COLOR: u32 = 0x00FF4444;
const LIGHT_MINIMIZE_COLOR: u32 = 0x00FFCC44;
const LIGHT_ZOOM_COLOR: u32 = 0x0044FF44;
const LIGHT_SIZE_PX: usize = 4;
const LIGHT_GAP_PX: usize = 2;
```

#### 2. Replace Pass 2 in `composite_pixel()` (lines 93-104)

Insert rim + light check after bounds confirmation, before `fill_rect_color` call. As shown in the pseudocode above.

### No diagnostic markers needed

Rim and lights are always-on per-pixel rendering. No new markers required. Existing composite markers suffice.

---

## Verification Plan

```bash
# Build both configurations
./scripts/entrypoint_build.sh
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both must pass with no new warnings.

```bash
# Visual verification (no automated pixel test in V1)
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null
```

Visual checks:
- Focused surface shows 4px neon rim (cyan-white) on all edges
- Three colored squares at top-left corner: red (close), yellow (minimize), green (zoom)
- Non-focused surfaces have no rim, no lights
- Rim and lights move with surface during drag
- Cursor renders on top of rim/lights (Pass 2 vs cursor z-order)
- All existing functional markers pass

Existing marker verification (no regression):
```bash
for m in \
  shell.frame.light.model \
  shell.frame.hover.set \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.selected.options.send
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-lights-render-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-lights-render-v1.log
```

---

## Decision Summary

| Question | Answer |
|----------|--------|
| Can sexdisplay render Frame Lights without ABI change? | ✅ **Yes** — surface bounds + focus state already known |
| Can lights render independently of neon rim? | ✅ **Technically yes**, but better to combine |
| Is hover IPC required for V1? | **No** — lights are always visible on focused surface |
| Is a protocol/ABI change needed? | **No** — zero ABI changes |
| What changes are needed? | Only `composite_pixel()` + constants in sexdisplay |
| Combine with rim or separate? | **Combine** — same function, same lines, same safety analysis |
| Implementation phase name | **FRAME_CHROME_RENDER_V1** (combined rim + lights) |
| Safety concern | Use `saturating_sub` for `sw - RIM_PX` to prevent usize underflow |

---

## Next Implementation Prompt

The next phase is **FRAME_CHROME_RENDER_V1** — implement neon rim AND Frame Lights in sexdisplay's `composite_pixel()`. Combined because both touch the same 8 lines.

```
MISSION: FRAME_CHROME_RENDER_V1

IMPLEMENTATION ONLY. Design complete in FRAME_CHROME_RENDER_PLAN_V1.md
and FRAME_LIGHTS_RENDER_PLAN_V1.md.

Files to modify:
- servers/sexdisplay/src/main.rs

Changes:
1. Add rim constants: RIM_PX=4, RIM_COLOR=0x00C0F0FF
2. Add light constants: LIGHT_CLOSE_COLOR=0x00FF4444,
   LIGHT_MINIMIZE_COLOR=0x00FFCC44, LIGHT_ZOOM_COLOR=0x0044FF44,
   LIGHT_SIZE_PX=4, LIGHT_GAP_PX=2
3. In composite_pixel() Pass 2, add rim edge detection with
   saturating_sub for sw/sh - RIM_PX
4. Within top rim band (ly < RIM_PX), check three light x-ranges
5. Use RIM_COLOR for non-light rim pixels
6. Verify both builds pass
7. Visual verification: rim + lights visible on focused surface

Forbidden:
- Any ABI/opcode change
- Any silk-shell change
- Any framebuffer bounds removal
- Any Surface struct field addition
- Any renderer refactor
- Hover IPC
- Action behavior

Pass criteria:
- Default build passes
- Synthetic build passes
- Neon rim visible around focused surface
- Three colored lights at top-left corner of focused surface
- No rim/lights on non-focused surfaces
- Rim and lights move with surface during drag
- Cursor renders on top
- No new warnings
```
