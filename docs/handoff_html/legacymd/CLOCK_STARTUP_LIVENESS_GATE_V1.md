# CLOCK_STARTUP_LIVENESS_GATE_V1

## Startup s=0 Reason
Startup redraws could appear frozen because:
1. redraw source marker defaulted to silkbar before first SetClock (`CLOCK_REDRAW_SOURCE=0`), and
2. fallback synthetic startup cadence was too sparse for early visible progression.

This created many early `[sexdisplay.clock.redraw] ... s=0 source=silkbar` lines before first nonzero redraw.

## Files Changed
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

## Minimal Diff Summary
### sexdisplay
- Start redraw source in fallback mode:
  - `CLOCK_REDRAW_SOURCE` default `1` (fallback) until first valid SetClock apply.
- Add startup liveness tracking:
  - `silkbar_clock_seen` flag.
- Make fallback synthetic tick cadence aggressive pre-SetClock:
  - startup threshold `1` loop until first SetClock.
  - steady threshold remains `64` loops after SilkBar clock has been seen.
- Preserve canonical clock latch and redraw source-check behavior.
- Update top-strip hash golden due intentional startup-pixel change:
  - old `0xFD6093AC9ADE7B4D`
  - new `0xD83B049A7ED0EE21`

### gate
Hardened `clock_visible_seconds` gate to require bounded liveness quality:
- fail if first 16 `sexdisplay.clock.redraw` markers are all `s=0`
- require first nonzero redraw within bounded marker distance (`<=240` lines)
- require post-update `source_check` equality (`redraw_ss == canonical_ss`)
- report first redraw line, first nonzero line, and distance
- keep fault gates unchanged

## New Gate Rule (clock_visible_seconds)
PASS only if all are true:
1. first redraw exists,
2. first nonzero redraw exists within bound,
3. startup sample window is not all zero,
4. no post-update source_check mismatches.

## Proof Excerpt
From `/tmp/sexos_clock_startup_liveness_gate.log`:
- gate report:
  - `clock_visible_seconds PASS first=6631 first_nonzero=6631 distance=0 source_check=equal`
- hash result:
  - `actual=0xD83B049A7ED0EE21 expected=0xD83B049A7ED0EE21 match=1 ok=1`

## Proof Result
- `FINAL: PASS (123 gates proved, 0 skipped, 0 faults)`
- no `#PF/#GP/panic/fault.kill`

