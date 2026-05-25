# SILKBAR_STARTUP_FLICKER_FINAL_FIX_V1

## Root Cause

1. `sexdisplay` fallback synthetic startup cadence was too aggressive:
   - `FALLBACK_SYNTH_TICK_LOOPS_STARTUP = 1` advanced fallback clock every loop.
   - Each fallback tick set `needs_top_strip_redraw = true`, producing visible startup strip flicker.
2. Drain capacity was undersized for known startup burst:
   - `DRAIN_MAX = 8` while startup deferred SilkBar burst can be 9 updates.
   - The 9th update spilled into a second drain cycle and second redraw cycle.
3. Clock no-op updates still triggered redraw:
   - `SetClock` updates with identical `hh:mm:ss` still armed top-strip redraw.
4. `OP_PRIMARY_FB` path did immediate in-drain top-strip redraw:
   - This bypassed batch coalescing and could add an extra startup redraw.

## Constants Changed

- `servers/sexdisplay/src/main.rs`
  - `DRAIN_MAX: 8 -> 16`
  - `FALLBACK_SYNTH_TICK_LOOPS_STARTUP: 1 -> 30`

## Behavioral Changes

- Redraw gate for `SetClock` now requires visible clock change:
  - unchanged `hh:mm:ss` no longer arms `needs_top_strip_redraw`.
- `OP_PRIMARY_FB` now sets `needs_top_strip_redraw = true` and relies on post-drain redraw path
  instead of immediate `redraw_top_strip()` call.
- Bounded drain remains bounded (`DRAIN_MAX=16`), no unbounded loop introduced.

## ABI/Protocol Safety

- No changes to `silkbar-model` layout or ABI constants.
- No changes to `OP_SILKBAR_UPDATE` format or decode.
- No kernel/scheduler/USB/HID edits.
- Existing framebuffer bounds checks untouched.

## Proof Markers To Check

- Startup/render/liveness:
  - `[sexdisplay.render.live.ok]`
  - `[sexdisplay.clock.redraw]`
  - `[sexusb.ready]`
- Fault scan must remain clean:
  - no `KERNEL PANIC`
  - no `EXCEPTION PAGE FAULT`
  - no `KERNEL PAGE FAULT HALT`
  - no `GP FAULT`
  - no `PKU SECURITY`
  - no `fault.kill pd=6`

## Visual Expectation

- No rapid startup chrome flicker storm.
- Clock does not visibly tick every loop during startup fallback.
- Boot strip settles after init/deferred update flush.
