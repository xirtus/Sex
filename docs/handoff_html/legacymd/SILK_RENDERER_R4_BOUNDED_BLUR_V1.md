# SILK_RENDERER_R4_BOUNDED_BLUR_V1

## Scope
- Patched only `servers/sexdisplay/src/main.rs` and this handoff doc.
- No kernel/ABI/PDX/model/layout edits.

## Added Statics
- `static mut BAR_BLUR_BUF: [u32; BAR_BG_H * BAR_BG_W_CAP]`
- One-shot marker gate: `BLUR_PROOF_LOGGED`

## Blur Helper
- Added `fn blur_bar_bg_buffer_radius1(fb_w: u32, fb_h: u32) -> bool`
- Behavior:
  - returns `false` when `fb_w > BAR_BG_W_CAP`
  - width=`fb_w`, height=`min(BAR_BG_H, fb_h)`
  - reads only `BAR_BG_BUF`
  - writes only `BAR_BLUR_BUF`
  - radius-1 clamped 3x3 averaging

## Sample Helper
- Added `fn sample_bar_blur_bg_xrgb(x: u32, y: u32, fb_w: u32, fb_h: u32) -> u32`
- Fallback to `bg(y, fb_h)` when:
  - `fb_w > BAR_BG_W_CAP`
  - `y >= BAR_BG_H`
  - `x >= fb_w`
- Otherwise returns `BAR_BLUR_BUF[y * BAR_BG_W_CAP + x]`.

## Render Integration
- `render()` now runs:
  1. `fill_bar_bg_buffer(...)`
  2. `blur_bar_bg_buffer_radius1(...)` only when fill succeeded
- Added one-shot marker:
  - `[sexdisplay.render.blur] radius=1 fb_w=<N> filled=<0|1>`

## Glass Path Change
- Updated `glass_over_bg` to sample from `sample_bar_blur_bg_xrgb(...)`.
- This affects only SilkBar glass background blending callsites (workspace active glow, net chip glass, launcher edge glass, panel body/glow blend path).
- No text/clock/cursor/surface compositing behavior changed.

## Build Proof
- Command: `./scripts/entrypoint_build.sh`
- Result: PASS (`[SEXOS ENTRYPOINT] success`)

## Runtime Proof
- Requested GTK run failed in this host (`gtk initialization failed`).
- Headless fallback run used (`-display none`) with serial log evidence.
- Observed markers:
  - `[sexdisplay.render.row_buffer] cap_w=2560 fb_w=1280 filled=1`
  - `[sexdisplay.render.blur] radius=1 fb_w=1280 filled=1`
- Ongoing render + clock markers observed.
- No `#PF`, `#GP`, `panic`, or `fault.kill` in sampled grep output.
