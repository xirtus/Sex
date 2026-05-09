# SILK_RENDERER_R6_INTEGRATED_GLASS_OVERLAY_V1

## Scope
- Patched only `servers/sexdisplay/src/main.rs` and this handoff doc.
- No kernel/ABI/PDX/model/layout changes.

## Visual Constants Tuned
- `R6_BAR_ALPHA = 196`
- `R6_CHIP_ALPHA = 212`
- `R6_EDGE_ALPHA = 72`
- `BAR_TOP_HIGHLIGHT = 0x00A8D8FF` (1px top highlight)
- `BAR_BOTTOM_DARK_EDGE = 0x000B1220` (1px bottom dark edge)
- `BAR_CRYSTAL_TINT = 0x001C2A54`

## Callsites Changed
- `chip_color`: Net chip glass alpha moved from 208 to `R6_CHIP_ALPHA`.
- `bar_color`:
  - launcher edge uses `R6_EDGE_ALPHA`
  - launcher body changed from flat cyan dot to subtle blue/cyan glass blend
  - bar row `y == 0`: top 1px crystalline highlight over glass body
  - bar row `y == 49`: bottom 1px dark/glow edge over glass body
  - bottom glow spread uses `R6_EDGE_ALPHA`
  - bar base/body path uses `R6_BAR_ALPHA` and subtle `BAR_CRYSTAL_TINT`

## Preserved Behavior
- Active workspace pulse from R5 unchanged.
- Clock/text/cursor/layout/compositor behavior unchanged.
- Blur source remains bounded `BAR_BLUR_BUF` path from R4.

## R6 Proof Marker
- Added one-shot marker:
  - `[sexdisplay.render.glass.r6] bar_alpha=196 chip_alpha=212 blur=1 pulse=1`

## Build Proof
- Command: `./scripts/entrypoint_build.sh`
- Result: PASS (`[SEXOS ENTRYPOINT] success`)

## Runtime Proof
- GTK path failed in this host (`gtk initialization failed`).
- Headless fallback (`-display none`) used.
- Observed markers:
  - `[sexdisplay.render.row_buffer] cap_w=2560 fb_w=1280 filled=1`
  - `[sexdisplay.render.blur] radius=1 fb_w=1280 filled=1`
  - `[sexdisplay.render.anim] period=96 base=48 amp=32`
  - `[sexdisplay.render.glass.r6] bar_alpha=196 chip_alpha=212 blur=1 pulse=1`
- Clock progression markers continue (`sexdisplay.clock.apply`, `sexdisplay.clock.redraw`).
- No `#PF`, `#GP`, `panic`, or `fault.kill` in sampled grep output.
