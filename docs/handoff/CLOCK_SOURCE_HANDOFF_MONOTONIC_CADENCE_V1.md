# CLOCK_SOURCE_HANDOFF_MONOTONIC_CADENCE_V1

## Root Cause
Near-final clock reset/cadence drift came from two coupled behaviors in `sexdisplay`:

1. `SetClock` from SilkBar was applied before any monotonic guard, so an older incoming second could overwrite the already-advanced fallback-visible second (source handoff rebase).
2. Synthetic fallback cadence was split (`startup=1 loop`, `steady=64 loops`), which creates an early fast phase and later slower phase, amplifying visible discontinuity around handoff time.

## Source Handoff Invariant
Visible canonical clock must not move backward on source transition.

- On incoming `SetClock`, if incoming is older than canonical in short-range handoff window, drop it for visible clock.
- First fallback->silkbar handoff only accepted when update is non-stale.
- Redraw source check remains canonical-backed (`redraw_ss == canonical_ss`).

## Files Changed
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

## Minimal Diff Summary
### `servers/sexdisplay/src/main.rs`
- Added helper `clock_to_day_seconds(hh, mm, ss)` for monotonic comparison.
- Unified synthetic fallback loop cadence to one bounded value:
  - `FALLBACK_SYNTH_TICK_LOOPS=64` for both startup and steady fallback synth path.
- Added monotonic guard before applying `SetClock`:
  - Drop stale short-range backward update instead of mutating visible bar clock.
- Added bounded markers:
  - `[sexdisplay.clock.handoff] from=fallback to=silkbar canonical_ss=C incoming_ss=S accepted=A`
  - `[sexdisplay.clock.source.silkbar.drop_stale] incoming_ss=S canonical_ss=C reason=monotonic_guard`
  - `[sexdisplay.clock.monotonic.guard] prev_ss=P next_ss=N accepted=A source=...`
  - `[sexdisplay.clock.cadence.phase] phase=startup/steady source=... loops=...`

### `scripts/daily_driver_master_gate.sh`
- Added new gate: `clock_source_handoff_monotonic`.
- Gate now reports:
  - `first_redraw_ss`
  - `max_ss_before_first_silkbar`
  - `first_silkbar_apply_ss`
  - `first_silkbar_visible_ss`
  - `backward_count`
  - `source_switch_line`
  - `early_delta`
  - `late_delta`
- Gate fails on:
  - backward visible second transitions (except 59->0 rollover)
  - accepted monotonic guard backward transition
  - source switch reset (`first_silkbar_visible_ss < max_ss_before_first_silkbar`)
- Wired into dependency/final summary flow:
  - added `gate_clock_source_handoff_monotonic`
  - included in dep fail checks and gate summary lines.

## Proof Command
- Build:
  - `./scripts/entrypoint_build.sh`
- Proof:
  - `SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_handoff_monotonic_v1.log`

## Marker Evidence (This Run)
- Build: PASS (`entrypoint_build.sh` completed successfully).
- Proof run status: inconclusive in this execution environment.
  - QEMU process ended with signal 15 before serial marker stream was captured into `/tmp/sexos_clock_handoff_monotonic_v1.log`.
  - Log currently contains Limine boot menu lines only; no `sexdisplay.clock.*` markers available yet.

## Remaining Risk
Clock remains synthetic cadence-driven and not a real hardware timebase.
This change enforces monotonic visible handoff and bounded cadence behavior, but does not implement RTC/PIT/HPET design.
