# SILK_RENDERER_R3_ROW_BUFFER_V1

## Scope
- Patched only `servers/sexdisplay/src/main.rs` for row-buffer capture.
- Added this handoff doc.
- No kernel/ABI/PDX edits.
- No blur/alpha/glow behavior changes in this phase.

## Constants and Buffer Added
- `BAR_BG_W_CAP: usize = 2560`
- `BAR_BG_H: usize = 50`
- `static mut BAR_BG_BUF: [u32; BAR_BG_H * BAR_BG_W_CAP]`

## Helper Added
- `fn fill_bar_bg_buffer(fb_w: u32, fb_h: u32) -> bool`
- Behavior:
  - returns `false` and skips fill when `fb_w > BAR_BG_W_CAP`
  - fills width=`fb_w`, height=`min(BAR_BG_H, fb_h)`
  - writes `bg(y, fb_h)` into `BAR_BG_BUF[y * BAR_BG_W_CAP + x]`
  - returns `true` on fill
- No heap, no framebuffer reads, no BAR_BG_BUF reads.

## Render Callsite
- `render()` now calls `fill_bar_bg_buffer(w as u32, h as u32)` before top-strip compositing loop.

## One-Shot Proof Marker
- Added one-shot marker guarded by `ROW_BUFFER_PROOF_LOGGED`:
  - `[sexdisplay.render.row_buffer] cap_w=2560 fb_w=<N> filled=<0|1>`

## Build Proof
- Command: `./scripts/entrypoint_build.sh`
- Result: PASS (`[SEXOS ENTRYPOINT] success`)

## Runtime Proof
- Requested GUI boot command with `-display gtk` could not run in this host:
  - `gtk initialization failed`
- Headless fallback used to verify serial markers:
  - `timeout 45s qemu-system-x86_64 ... -display none ...`
- Marker observed:
  - `[sexdisplay.render.row_buffer] cap_w=2560 fb_w=1280 filled=1`
- No fault markers in sampled log segment:
  - no `#PF`
  - no `#GP`
  - no `panic`
  - no `fault.kill`

## Notes
- This phase only captures background gradient rows into a capped display-owned strip buffer for future blur work.
- BAR_BG_BUF remains write-only in this revision.
