# CLOCK_FALLBACK_CONTINUITY_V2

## Scope
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

## Root cause
When a `SetClock` update is rejected by the existing monotonic/stale guard path (`apply_update=false` for `SetClock`), the display loop could remain effectively keyed to SilkBar source state until a later stale timeout path. During this window, canonical fallback continuity and redraw source checks were not explicitly enforced at the stale-drop boundary, so visual clock progress could appear frozen.

## Minimal fix
1. Preserve stale/backward rejection behavior (no acceptance of stale producer values).
2. On rejected `SetClock`:
- emit `[sexdisplay.clock.stale_drop] ... accepted=0`
- emit `[sexdisplay.clock.handoff] from=fallback to=silkbar accepted=0`
- force immediate fallback authority (`clock_from_silkbar=false`), keep cadence continuity, arm one-shot continuity marker.
3. On next fallback tick (real/synth):
- emit `[sexdisplay.clock.fallback.continue_after_drop] ss=S source=fallback`.
4. Before every `redraw_top_strip` call:
- copy canonical `CLOCK_CANON_HH/MM/SS` into `bar.clock_*`.
5. Redraw source marker hardens to include explicit check bit:
- `[sexdisplay.clock.redraw.source_check] ... ok=1|0`.

## Gate updates
`clock_visible_seconds` now additionally enforces:
- `redraw.source_check ... ok=1` present
- no `redraw.source_check ... ok=0`
- if stale-drop is observed, fallback-continue-after-drop must be observed

This keeps daily-driver honest when stale-drop does not occur in a given boot, while enforcing continuity once stale-drop exists.

## Commands run
```bash
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_fallback_continuity_v2.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_fallback_continuity_v2.log
```

## Results
- Build: PASS
- Proof log generated: `/tmp/sexos_clock_fallback_continuity_v2.log`
- Gate (current script on generated log): `FINAL: PASS (269 gates proved, 117 skipped, 0 faults)`
- Clock gates:
  - `clock_visible_seconds: PASS`
  - `clock_cadence_bound: PASS`

## Marker excerpts (from `/tmp/sexos_clock_fallback_continuity_v2.log`)
- `[sexdisplay.clock.redraw.source_check] redraw_ss=1 canonical_ss=1 source=fallback ok=1`
- `[sexdisplay.clock.redraw.source_check] redraw_ss=2 canonical_ss=2 source=fallback ok=1`
- `[sexdisplay.clock.redraw.source_check] redraw_ss=3 canonical_ss=3 source=fallback ok=1`
- `clock_visible_seconds PASS ... source_check_ok_count=64 source_check_bad_count=0 stale_drop_count=0 continue_after_drop_count=0`

## Notes
In this specific proof boot, stale-drop markers were not triggered (`stale_drop_count=0`), but the continuity enforcement is now wired and gated conditionally for boots where stale-drop occurs.
