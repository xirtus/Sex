# SILK_RENDERER_R5_ANIMATION_CADENCE_V1

## Scope
- Patched only `servers/sexdisplay/src/main.rs` and this handoff doc.
- No kernel/ABI/PDX/scheduler edits.

## Counter Added
- `static RENDER_FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);`
- Incremented once per `render()` call:
  - `let frame = RENDER_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);`

## Wave Helpers Added
- `fn triangle_wave_u8(frame: u64, period: u64) -> u8`
- `fn pulse_alpha(base: u8, amp: u8, frame: u64, period: u64) -> u8`
- Integer math only, bounded output, period-0 safe handling.

## Animated Target (Single Safe Target)
- Only active workspace glow alpha was animated.
- Parameters:
  - `base=48`
  - `amp=32`
  - `period=96` frames
- Applied in `workspace_color` before glass blend.

## One-Shot Proof Marker
- Added one-shot marker:
  - `[sexdisplay.render.anim] period=96 base=48 amp=32`

## Build Proof
- Command: `./scripts/entrypoint_build.sh`
- Result: PASS (`[SEXOS ENTRYPOINT] success`)

## Runtime Proof
- GTK path failed in this host (`gtk initialization failed`).
- Headless fallback (`-display none`) used for proof lane.
- Observed markers:
  - `[sexdisplay.render.row_buffer] cap_w=2560 fb_w=1280 filled=1`
  - `[sexdisplay.render.blur] radius=1 fb_w=1280 filled=1`
  - `[sexdisplay.render.anim] period=96 base=48 amp=32`
- Clock advancement markers continue (`sexdisplay.clock.redraw`, `sexdisplay.clock.apply`).
- No `#PF`, `#GP`, `panic`, or `fault.kill` in sampled grep output.
