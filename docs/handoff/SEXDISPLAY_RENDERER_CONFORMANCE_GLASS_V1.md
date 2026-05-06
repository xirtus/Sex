# SEXDISPLAY_RENDERER_CONFORMANCE_GLASS_V1

- date: 2026-05-06
- git commit: (current HEAD)
- ISO: sexos-v1.0.0.iso (full build, GREEN_MASTER)

## Summary

Audited all sexdisplay framebuffer write paths for bounds safety and
correct ownership. Confirmed the renderer writes only from model/shell
state (SilkBar model + Surface structs + Appearance Tokens), does not
own policy, and uses flat ARGB colors (no alpha blending, no blur, no
shadow, no full-frame effects). Applied two minimal changes: (1) prefer
DEFAULT_THEME from silkbar-model over hardcoded bg gradient endpoints,
and (2) added a live render proof marker.

## Changes Made

### 1. Prefer DEFAULT_THEME for bg() gradient endpoints

File: `servers/sexdisplay/src/main.rs` (line 328)

Before:
```rust
fn bg(y: usize) -> u32 {
    if      y < 200 { 0x00081424 }  // deep navy
    else if y < 350 { 0x00102038 }
    else if y < 500 { 0x00182850 }
    else if y < 650 { 0x00281848 }  // warm purple
    else            { 0x00281848 }
}
```

After:
```rust
fn bg(y: usize) -> u32 {
    if      y < 200 { DEFAULT_THEME.bg_top }    // deep navy
    else if y < 350 { 0x00102038 }
    else if y < 500 { 0x00182850 }
    else if y < 650 { DEFAULT_THEME.bg_bottom }
    else            { DEFAULT_THEME.bg_bottom }
}
```

`0x00081424 == DEFAULT_THEME.bg_top` and `0x00281848 == DEFAULT_THEME.bg_bottom`
(value-identical). Mid-gradient stops (0x00102038, 0x00182850) are not in
DEFAULT_THEME and remain hardcoded.

### 2. Add live render proof marker

File: `servers/sexdisplay/src/main.rs` (line 984)

Added `[sexdisplay.render.live.ok]` after the first live-FB render
(OP_PRIMARY_FB handler). Fires once with fb_w/fb_h dimensions:

```
[sexdisplay.render.live.ok] fb_w=1280 fb_h=800
```

Proof chain for render liveness:
- `[sexdisplay.ready]` — initial fallback-FB render done
- `[sexdisplay.render.live.ok]` — first live-FB render done
- `[silk.render_proof.top_strip.start/hash/ok]` — top-strip pixel hash verified non-zero

## Audit Results

### All framebuffer write paths are bounds-checked

| Function | Bounds guards | Safe? |
|----------|--------------|-------|
| `render()` | addr >= HIGH_HALF_BASE, w/h non-zero, w <= MAX_FB_W, h <= MAX_FB_H, checked_mul for total_pixels, idx < total_pixels per pixel | ✓ |
| `redraw_top_strip()` | Same guards + h >= 51, y iterates 0..51 only, idx < total_pixels per pixel | ✓ |
| `redraw_surface_area()` | Same guards + h >= 51, y iterates 50..h only, idx < total_pixels per pixel | ✓ |
| `draw_cursor_z_top()` | py >= h break, px >= w continue, idx < total_pixels check | ✓ |
| `draw_launcher_panel()` | py >= h break, px >= w continue, idx >= total_pixels continue | ✓ |
| `clamp_surface()` | x/y/w/h clamped to [0, fb_dim), y >= BAR_H (50px) | ✓ |
| `composite_pixel()` | Uses clamp_surface() for all surface lookups, rim arithmetic uses saturating_sub | ✓ |
| `fill_rect_color()` | Iterates over fill_count only, bounds-checked pixel coords relative to surface | ✓ |

### State ownership is correct

- **SilkBar model**: Only mutated via `handle_silkbar_update()` → `silkbar_model::apply_update()`
- **Surface registry**: Only mutated via IPC ops (0xEC create, 0xEB move, 0xEE destroy, 0xEF fill rect, 0xFD tab info). Owner PD is bound on create and enforced on all mutations via `caller_pd` check.
- **Focus**: Set only via OP_SET_FOCUS (0xED) from shell
- **Appearance tokens**: Set only via OP_APPEARANCE_TOKENS (0xFC), two-call commit. All token colors pass through `clamp_color_token()` which forces alpha=0xFF.
- **Background gradient**: `DEFAULT_THEME.bg_top` / `DEFAULT_THEME.bg_bottom` from silkbar-model

### Flat ARGB glass constants (no alpha, no blur, no shadow)

- All colors are 0x00RRGGBB format (alpha byte unused by raw framebuffer)
- `clamp_color_token()` forces alpha=0xFF for token colors as future-proofing
- No alpha blending, no blur, no shadow, no full-frame effects
- The `bg()` gradient is 5 hardcoded color bands — a flat approximation, not a rendering primitive
- `effect_levels` field in RenderTokensV1 has blur bits cleared (`& !0x0F`) to enforce zero blur in V1

### Duplicate DEFAULT_THEME colors replaced

The only exact duplicates of DEFAULT_THEME values in sexdisplay's render
constants were the bg gradient endpoints (bg_top and bg_bottom), now replaced
with direct references. All other renderer constants are chrome/frame-specific
colors not present in the Theme struct.

## Proof Markers Added

```
[sexdisplay.render.live.ok] fb_w=1280 fb_h=800
```

## Verified at Runtime

- Full ISO build: PASS (BUILD_GATE)
- 6 PDs spawn: PASS (SPAWN_GATE)
- Clock ticks: PASS (CLOCK_GATE)
- Scheduler liveness: PASS (SCHED_GATE)
- Zero faults/panics: PASS (FAULT_GATE)
- Master gate score: GREEN_MASTER

## Future Considerations

- If new rendering primitives are added (e.g., rounded rects, gradients,
  textures), they must be bounds-checked via `clamp_surface()` or equivalent.
- The `bg()` mid-gradient stops (0x00102038, 0x00182850) could become Theme
  tokens if the model expands its palette.
- Alpha-aware rendering is not supported; all writes are opaque.
- `DISPLAY_TOKENS.effect_levels` blur bits are hard-forced to zero in V1;
  any future blur path must remain bounds-safe and framebuffer-local.
