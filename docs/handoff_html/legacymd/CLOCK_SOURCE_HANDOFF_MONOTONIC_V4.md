# CLOCK_SOURCE_HANDOFF_MONOTONIC_V4

## Root cause
Fallback clock advanced canonical time, but first SilkBar `SetClock` could be accepted even when lower than canonical fallback time. This allowed a backward canonical rewrite and visible reset.

Bad sequence seen in logs:
- `[sexdisplay.clock.redraw.source_check] redraw_ss=26 canonical_ss=26 source=fallback ok=1`
- later: `[sexdisplay.clock.redraw.source_check] redraw_ss=1 canonical_ss=1 source=silkbar ok=1`

## Fix rule
When handoff candidate is SilkBar and current canonical source is fallback:
- compute incoming and canonical total seconds (`HH:MM:SS`)
- if `incoming < canonical`: reject handoff, keep fallback canonical clock/source
- if `incoming >= canonical`: accept handoff

## Markers
Reject/defer:
- `[sexdisplay.clock.handoff.reject] from=fallback to=silkbar incoming_ss=S canonical_ss=S reason=backward accepted=0`
- `[sexdisplay.clock.fallback.continue_after_handoff_reject] ss=S source=fallback`

Accept:
- `[sexdisplay.clock.handoff.accept] from=fallback to=silkbar incoming_ss=S canonical_ss=S accepted=1`

Visible monotonic proof:
- `[sexdisplay.clock.monotonic.visible] prev_ss=P redraw_ss=S ok=1`
- `ok=0` means visible backward jump without allowed rollover handling.

## Gate expectations
`clock_visible_seconds` must fail if any of:
- `source_check` has `ok=0`
- `monotonic.visible` has `ok=0`
- fallback `canonical_ss` exceeds a later silkbar `canonical_ss` in one boot sequence
- fallback reached `>=10`, a later backward silkbar handoff is seen, but no `handoff.reject`
- `handoff.reject` exists but no `fallback.continue_after_handoff_reject`

V3 continuity checks stay required:
- stale probe markers
- stale drop rejection markers
- fallback continue-after-drop markers
- redraw source check `ok=1`

## Proof commands
```bash
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_source_handoff_monotonic_v4.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_source_handoff_monotonic_v4.log
```

GTK visual:
```bash
LOG=/tmp/sexos_clock_source_handoff_monotonic_v4_gtk.log
qemu-system-x86_64 \
  -M q35 \
  -m 512M \
  -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-mouse,bus=xhci.0 \
  -serial file:"$LOG" \
  -display gtk \
  -boot d

rg -n "sexdisplay.clock.redraw.source_check|sexdisplay.clock.handoff|sexdisplay.clock.monotonic.visible|sexdisplay.clock.fallback.continue_after|#PF|#GP|panic|fault" \
  /tmp/sexos_clock_source_handoff_monotonic_v4_gtk.log | tail -180
```

## Do-not-regress
- Never let fallback->silkbar handoff move canonical time backward.
- Keep stale-drop continuity behavior and markers from V3.
- Keep redraw source-check and monotonic markers green (`ok=1`, no `ok=0`).
