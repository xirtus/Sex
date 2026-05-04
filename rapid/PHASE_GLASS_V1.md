# PHASE GLASS_V1: Glass Chrome — Alpha Transparency on Window Chrome

## Goal
Enable semi-transparent alpha blending on window chrome elements (top bar, rim, tabs) so SexOS windows visually appear to have glass/glow effects. No blur, no animation — just alpha transparency letting surface content show through chrome.

## Revolutionary Angle
Every pixel in the SexOS compositor is currently fully opaque — `clamp_color_token()` forces `0xFF` alpha on every color. The framebuffer is written with raw overwrite semantics: last pixel written wins. This produces crisp, correct rendering — but flat.

**Glass changes ONE thing:** when a chrome pixel has alpha < 0xFF, it blends with the surface content behind it instead of overwriting it. This lets the top bar, rim, and tabs become semi-transparent layers that reveal the window content beneath them.

The effect is dramatic for the code change:
- Top bar at 0x60 alpha (~38% opacity): surface content shows through faintly, like frosted glass
- Rim at 0x80 alpha (~50% opacity): neon rim glows with surface content bleeding through
- Tabs at 0x60-0x80 alpha: subtle layering between active and inactive tabs

No blur kernel needed. No framebuffer refactor. No new surfaces. Just **src-over alpha blending on 5 assignment sites** in `composite_pixel()`.

## Ownership
- **sexdisplay** (exclusive): alpha blending in composite_pixel(), color constant updates
- **silk-shell** (token sender): appearance token alpha value updates

## What Already Exists
- `composite_pixel()` with Pass 1 (non-focused) and Pass 2 (focused + chrome) — complete, working
- `clamp_color_token()` — currently forces `| 0xFF000000`, will be modified
- Color constants: `FRAME_TOP_BAR_COLOR = 0x0088C2B7`, `FRAME_RIM_COLOR = 0x00B8F2E8`, etc. — all use 0x00 alpha (clamped to 0xFF at render time)
- Appearance token presets: 4 presets (BottleGlass, VioletGlass, GraphiteGlass, HighContrast) — all use 0x00 alpha in the high byte
- Alpha blending primitives are common knowledge — no library dependency
- No alpha blending exists anywhere in the rendering pipeline

## Code Changes

### 1. Add alpha_blend() helper
```rust
/// Standard src-over alpha blending.
/// Channels: ARGB (8 bits each). Alpha is in the high byte.
/// Result is fully opaque (alpha = 0xFF) regardless of input alphas.
#[inline]
fn alpha_blend(bg: u32, fg: u32) -> u32 {
    let bg_a = (bg >> 24) & 0xFF;
    let bg_r = (bg >> 16) & 0xFF;
    let bg_g = (bg >>  8) & 0xFF;
    let bg_b =  bg        & 0xFF;
    let fg_a = (fg >> 24) & 0xFF;
    let fg_r = (fg >> 16) & 0xFF;
    let fg_g = (fg >>  8) & 0xFF;
    let fg_b =  fg        & 0xFF;
    // src-over: out = fg * fg_a/255 + bg * (1 - fg_a/255)
    let out_a = 0xFFu32;
    let out_r = ((fg_r as u32 * fg_a) + (bg_r as u32 * (255 - fg_a))) / 255;
    let out_g = ((fg_g as u32 * fg_a) + (bg_g as u32 * (255 - fg_a))) / 255;
    let out_b = ((fg_b as u32 * fg_a) + (bg_b as u32 * (255 - fg_a))) / 255;
    (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b
}
```

### 2. Add blend_chrome() dispatch helper
```rust
/// If chrome is fully opaque, return chrome (overwrite).
/// If chrome is semi-transparent, blend chrome over existing c.
/// If chrome is fully transparent (alpha=0), return c unchanged.
#[inline]
fn blend_chrome(existing: u32, chrome: u32) -> u32 {
    let a = (chrome >> 24) & 0xFF;
    if a == 0xFF { chrome }
    else if a == 0x00 { existing }
    else { alpha_blend(existing, chrome) }
}
```

### 3. Remove alpha forcing from clamp_color_token()
```rust
// Before:
fn clamp_color_token(c: u32) -> u32 { c | 0xFF000000 }
// After:
fn clamp_color_token(c: u32) -> u32 { c }
```

### 4. Replace chrome assignments in composite_pixel() Pass 2
Every `c = DISPLAY_TOKENS.*` in the chrome section becomes `c = blend_chrome(c, DISPLAY_TOKENS.*)`:
- Top bar background: `c = blend_chrome(c, DISPLAY_TOKENS.frame_top_bar_color)`
- Tab colors: `c = blend_chrome(c, DISPLAY_TOKENS.active_tab_color)` etc.
- Rim band: `c = blend_chrome(c, DISPLAY_TOKENS.frame_rim_color)`
- Lights remain fully opaque (fast-path in blend_chrome is just `chrome`)

### 5. Update color constants
```rust
// Surface colors (fully opaque)
const FOCUS_SURFACE_COLOR: u32 = 0xFF7AAFA4;       // was 0x00
const FRAME_LIGHT_CLOSE_COLOR: u32 = 0xFFFF4444;    // was 0x00
const FRAME_LIGHT_MINIMIZE_COLOR: u32 = 0xFFFFCC44;  // was 0x00
const FRAME_LIGHT_ZOOM_COLOR: u32 = 0xFF44FF44;     // was 0x00
// Glass chrome (semi-transparent)
const FRAME_RIM_COLOR: u32 = 0x80B8F2E8;            // was 0x00 — 50% opacity
const FRAME_TOP_BAR_COLOR: u32 = 0x6088C2B7;        // was 0x00 — 38% opacity
const TAB_ACTIVE_COLOR: u32 = 0x807AAFA4;           // was 0x00 — 50% opacity
const TAB_INACTIVE_COLOR: u32 = 0x406080B0;         // was 0x00 — 25% opacity
```

### 6. Update silk-shell token presets
Match alpha bytes in TOKEN_PRESETS (all 4 presets), keep HighContrast fully opaque.

## Bundle

| Task | Detail | Effort | Priority |
|------|--------|--------|----------|
| alpha_blend() + blend_chrome() | Add helper functions to sexdisplay | 1h | HIGH |
| Remove alpha clamp | Modify clamp_color_token() to preserve alpha | 0.5h | HIGH |
| Update color constants | Set proper alpha on all chrome/surface colors | 0.5h | HIGH |
| Replace chrome assignments | ~6 sites in composite_pixel() Pass 2 → blend_chrome | 1h | HIGH |
| Update silk-shell token presets | Match alpha values in all 4 presets | 0.5h | HIGH |
| Test + tune | Boot QEMU, verify glass effect, tune alpha values | 2h | Medium |
| HighContrast override | Keep fully opaque for accessibility preset | 0.5h | Medium |

Total: ~6h

## Smallest First Step
Add `alpha_blend()` and `blend_chrome()`. Replace ONE chrome assignment (top bar background) in composite_pixel(). Set `FRAME_TOP_BAR_COLOR` to `0x6088C2B7`. Boot QEMU — top bar becomes semi-transparent with surface content visible through it. If it works, apply to remaining chrome elements. If it breaks, revert 3 lines.

## Dependencies
- **Zero dependencies on any phase.** sexdisplay-only change + silk-shell cosmetic.
- **Can be done RIGHT NOW**, before any other phase.

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Alpha blending causes visual artifacts | Medium | Medium | src-over is mathematically correct. Test with known colors first. |
| Performance regression | Low | Low | alpha_blend is ~8 integer ops per pixel. Negligible vs VRAM write cost. |
| Fully transparent chrome invisible | Low | Low | blend_chrome returns existing when alpha=0x00. Self-correcting. |
| HighContrast loses contrast | Low | High | HighContrast stays fully opaque (0xFF on all chrome). |
| SilkShell token alpha desyncs from sexdisplay | Medium | Low | Immediately visible on boot. Fix by aligning values. |

## Testing Strategy
- Boot QEMU, verify top bar is semi-transparent with surface content behind it visible
- Toggle minimal mode (F4), verify rim band is semi-transparent
- Verify lights are fully opaque (no surface bleed-through)
- Cycle all 4 presets (F5), verify each has appropriate glass appearance
- Verify cursor renders correctly over glass chrome
- All existing markers fire

## Exit Criteria
- [ ] alpha_blend() and blend_chrome() added to sexdisplay
- [ ] clamp_color_token() no longer forces 0xFF alpha
- [ ] All chrome assignments in composite_pixel() Pass 2 use blend_chrome()
- [ ] Color constants updated with meaningful alpha values
- [ ] Silk-shell token presets updated with matching alpha values
- [ ] HighContrast preset remains fully opaque
- [ ] Boot QEMU: top bar visibly semi-transparent
- [ ] Boot QEMU: lights are fully opaque
- [ ] F4 minimal mode: rim band is semi-transparent
- [ ] F5 preset cycle: all presets have correct glass appearance
- [ ] Build passes. Boot passes. No panic.

## Efficiency Opportunity
The entire phase is changing ~15 lines of color constants and ~6 chrome assignments. Highest visual-impact-per-line-of-code change in the entire OS.

## Files Changed
- `servers/sexdisplay/src/main.rs` (+alpha_blend, +blend_chrome, modify clamp_color_token, update color constants, 6 chrome assignment changes)
- `servers/silk-shell/src/main.rs` (update alpha bytes in TOKEN_PRESETS)

## Forbidden
- Blur kernel (defer to V2 — too expensive for software rendering)
- Animation (see ANIMATION_V1 phase)
- New surfaces or framebuffer layers
- Alpha on surface content (only chrome gets alpha)
- Broad refactor of composite_pixel()

## Next Phase
PHASE_ANIMATION_V1.md
