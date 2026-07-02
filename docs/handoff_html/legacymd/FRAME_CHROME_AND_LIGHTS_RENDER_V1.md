# FRAME_CHROME_AND_LIGHTS_RENDER_V1

## Status

Implemented (2026-05-04). Neon rim and Frame Lights (red close, yellow minimize, green zoom) rendered in sexdisplay on the focused surface. Combined implementation because both modify the same 8 lines in `composite_pixel()` Pass 2.

---

## Contracts Proven

| Contract | Mechanism | Location |
|----------|-----------|----------|
| Neon rim on focused surface | 4px edge band via `FRAME_RIM_PX`/`FRAME_RIM_COLOR` | `composite_pixel()` Pass 2 |
| Frame Lights at top-left | 4×4px colored squares within top rim band | `composite_pixel()` Pass 2 |
| Lights drawn before rim | Light check within `ly < FRAME_RIM_PX` branch returns light color first | Lights override rim at overlap |
| No rim/lights on non-focused | Only triggered in Pass 2 (focused surface check) | Pass 1 untouched |
| Bounds safety preserved | All rim/light checks use clamped `(sx, sy, sw, sh)` | `clamp_surface()` still called |
| No underflow on tiny surfaces | `sw.saturating_sub(FRAME_RIM_PX)` and `sh.saturating_sub(...)` | Edge detection safe for any size |
| Cursor renders on top | `draw_cursor_z_top()` called after `composite_pixel()` | Unchanged |
| SilkBar unaffected | Top strip rendering path unchanged | `y < 50` path separate |
| No ABI/protocol changes | All data already in sexdisplay (bounds + focus state) | No new opcodes |

---

## Changes

### File: `servers/sexdisplay/src/main.rs`

#### 1. Constants added (after `FOCUS_SURFACE_COLOR`, ~line 53)

```rust
const FRAME_RIM_PX: usize = 4;
const FRAME_RIM_COLOR: u32 = 0x00C0F0FF;
const FRAME_LIGHT_SIZE_PX: usize = 4;
const FRAME_LIGHT_GAP_PX: usize = 2;
const FRAME_LIGHT_CLOSE_COLOR: u32 = 0x00FF4444;
const FRAME_LIGHT_MINIMIZE_COLOR: u32 = 0x00FFCC44;
const FRAME_LIGHT_ZOOM_COLOR: u32 = 0x0044FF44;
```

#### 2. `composite_pixel()` Pass 2 modified (~line 112)

Replaced single `fill_rect_color()` call with rim + light priority logic:

1. Compute local `(lx, ly)` relative to clamped surface bounds
2. Compute rim edges with `saturating_sub` (safe for tiny surfaces)
3. If pixel is within rim band:
   - If top rim band (`ly < FRAME_RIM_PX`): check three light x-ranges
   - Return light color if in a light, `FRAME_RIM_COLOR` otherwise
4. If pixel is in content area: return `fill_rect_color()` as before

### File Changes

| File | Changes |
|------|---------|
| `servers/sexdisplay/src/main.rs` | +8 constants, ~35 lines in composite_pixel Pass 2 |

### Files NOT Modified

Everything else — kernel, PDX ABI, silk-shell, silkbar, silkbar-model, sexusb, sexinput all untouched.

---

### Light Geometry (surface-local coordinates)

```
lx=0  lx=2    lx=6  lx=8     lx=12 lx=14    lx=18
 │     ┌──────┐ │    ┌──────┐ │    ┌──────┐ │
 │     │ RED  │ │    │YELLOW│ │    │GREEN │ │   ly=0..3 (top rim)
 │     │CLOSE │ │    │MINIMZ│ │    │ ZOOM │ │
 │     └──────┘ │    └──────┘ │    └──────┘ │
 │    gap=2     │   gap=2     │   gap=2     │
 ├── rim band (4px) ────────────────────────┤
 │                                           │
 │              content area                  │  ly >= 4
 │           (FOCUS_SURFACE_COLOR             │
 │            or fill_rect_color)             │
```

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
# Run with real mouse
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-chrome-and-lights-render-v1.log

# Check for faults
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-chrome-and-lights-render-v1.log

# Check key markers still fire
grep -ac "\[sexdisplay.selected.options.update\]" /tmp/frame-chrome-and-lights-render-v1.log
grep -ac "\[shell.drag.move\]" /tmp/frame-chrome-and-lights-render-v1.log
```

### Visual verification (manual)

- Focused surface shows a 4px neon cyan rim (`0x00C0F0FF`) on all four edges
- Three colored 4×4 squares at top-left corner of focused surface:
  - Red (`0x00FF4444`) = CLOSE
  - Yellow (`0x00FFCC44`) = MINIMIZE
  - Green (`0x0044FF44`) = ZOOM
- Non-focused surfaces have no rim and no lights
- Rim and lights move with the surface during drag
- Cursor renders on top of rim and lights
- SilkBar top strip is unaffected

### Pass criteria

- Default build passes
- Synthetic build passes
- No panic/#PF/#GP
- Rim + lights visible on focused surface (visual)
- No rim/lights on non-focused surfaces
- Drag lifecycle intact
- Selected-window options display still works
- No ABI/protocol changes
- No action behavior implemented

---

## Bounds Safety Analysis

`clamp_surface()` is called before the rim/light check, guaranteeing:
- `(sx, sy, sw, sh)` are within framebuffer bounds
- `y >= 50` (below SilkBar)
- `x` and `y` are valid framebuffer coordinates

The rim edge detection uses `saturating_sub`:
- `sw.saturating_sub(FRAME_RIM_PX)` = for `sw >= 4`: `sw - 4`; for `sw < 4`: `0`
- When `rim_right = 0`, the condition `lx >= 0` is always true, so the entire width is "rim" — correct for tiny surfaces
- No underflow possible, no panic

The light geometry check:
- `lx = x - sx` is safe because `x >= sx` is confirmed by the bounds check
- `ly = y - sy` is safe for the same reason
- All light range expressions involve only addition of constants (no subtraction)
- Lights beyond the right edge of the surface are simply not rendered (the `lx < ...` checks fail naturally)

---

## Remaining Risks

- **No hover highlighting**: Lights are always visible on the focused surface. The shell tracks `HOVERED_FRAME_LIGHT` (which light is hovered) but this state is never forwarded to sexdisplay. Hover highlight (e.g., brighter light) is deferred.
- **No action behavior**: Clicking a light currently falls through to FrameChrome rim drag or capture. No close/minimize/zoom behavior.
- **Tiny light targets**: 4×4px squares are visually small but visible. Future phases may increase to 8×8 when a tab strip is added.
- **Rim always visible**: The neon rim is drawn on the focused surface regardless of hover state. A future phase may gate rim visibility on hover proximity.
- **No tab strip**: Lights live in the top rim band because `FRAME_TAB_STRIP_PX = 0`. When tab strip is added, lights should relocate.

---

## Next Recommended Phase

### FRAME_LIGHTS_ACTION_PLAN_V1

Design how clicking a Frame Light triggers its action (close/minimize/zoom). Requires:
- Hit-target classification for light click (extend `hit_test_surface_chrome()` or add light-specific HitTarget)
- Click handling in `click_hit_test_and_focus()` that routes to action instead of rim drag
- Safety guards for each action (confirmation for close, state management for minimize/zoom)
- No surface destruction in V1 unless safe

Alternatively, **FRAME_HOVER_IPC_V1** could forward `HOVERED_FRAME_LIGHT` from silk-shell to sexdisplay, enabling light hover highlighting (brighten the hovered light, dim others).
