# CLOCK_SYNTHETIC_VISUAL_MODE_V11

## Root Cause
Under GTK, `raw_ticks` is often zero or effectively stalled in `sexdisplay`.
Existing fallback synthetic cadence advanced one visible second every 8 display loops.
On slow GTK redraw cadence, 8 loops can take many real seconds, so visual clock moved at ~10s per second.

## V11 Change
Add explicit synthetic visual clock mode in `servers/sexdisplay/src/main.rs` for proof-time rendering honesty:

- Enter when real tick is unavailable/stalled:
  - `raw_ticks == 0`, or
  - `raw_ticks` unchanged past stale threshold.
- Mode identity:
  - `clock_source=fallback_synthetic_visual` (emitted via cadence sample source field).
- Visual cadence:
  - bounded threshold `2` loops per visible second (visual proof time only, not real time).
- Exit when real ticks resume advancing.

This preserves V3-V10 monotonic and handoff protections while making GTK visual proof clock usable.

## New Markers
- Enter:
  - `[sexdisplay.clock.synthetic_visual.enter] raw_ticks=R reason=zero_or_stalled ok=1`
- Tick:
  - `[sexdisplay.clock.synthetic_visual.tick] old_ss=O new_ss=N threshold=2 ok=1`
- Exit:
  - `[sexdisplay.clock.synthetic_visual.exit] raw_delta=D ok=1`

Updated existing cadence sample source field:
- synthetic visual ticks emit `source=fallback_synthetic_visual`
- real tick-gated fallback emits `source=fallback`

## Gate Updates
`scripts/daily_driver_master_gate.sh` now enforces:
- If `raw_delta=0 synthetic=1` cadence samples dominate over non-zero raw deltas,
  - require `synthetic_visual.enter` marker,
  - require `synthetic_visual.tick ok=1` marker.
- `handoff.reject` freeze detection now evaluates post-reject progress per reject point `canonical_ss=R`:
  - progress accepted from any later marker with `ss > R`:
    - `fallback.live_after_reject` (`now_ss`)
    - `synthetic_visual.tick` (`new_ss`)
    - `redraw.source_check` (`canonical_ss`)
    - `cadence.sample` with `source=fallback_synthetic_visual` (`ss`)
  - freeze only fails when reject exists, no later progress is found, and enough later clock evidence exists to prove stall.
- `clock_visible_seconds` diagnostics include:
  - `reject_freeze`
  - `reject_progress_count`
  - `synthetic_visual_tick_count`
  - `live_after_reject_progress_count`
- Existing failures preserved:
  - fail on `monotonic.visible ok=0`
  - fail on `source_check ok=0`
  - preserve no-freeze/no-backward-reset checks.

## Do-Not-Regress
- Do not treat synthetic visual cadence as real time.
- Keep fallback->silkbar monotonic handoff rules from V3-V10.
- Lower SilkBar `SetClock` must stay rejected.
- Rejected SilkBar packets must not reset synthetic visual cadence.
- Do not reintroduce `CLOCK_SEND_LIMIT`.
- Keep same-second redraw suppression except when second advances.
