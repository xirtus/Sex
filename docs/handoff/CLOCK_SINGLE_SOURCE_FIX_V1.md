# CLOCK_SINGLE_SOURCE_FIX_V1

## Duplicate Sources Found
Two visible clock sources were active in `sexdisplay` top-strip rendering:
1. V2 top-left debug/visibility chip at `x=16 y=8 w=132 h=24`
2. Legacy hardfix clock path around `x=820` (chip background + `clock_fg_at` digits)

This caused duplicate boxes (empty + frozen perception).

## Source Removed / Kept
Removed/disabled for visible rendering:
- legacy hardfix source at `x=820` by removing:
  - hardcoded `chip_color()` override box (`in_rect(..., 820, 18, 80, 22)`)
  - `clock_fg_at(...)` usage in top-strip render paths

Kept as single visible source:
- `clock_visible_chip_v2_at(...)` at top-left, driven by live `bar.clock_ss`

## Final Single Source
- Exactly one visible clock box is drawn.
- Live tick parity is sourced from the same `bar.clock_ss` used by runtime clock updates.
- Marker proofs:
  - `[clock.single.source] boxes=1 debug_chip=0 original_chip=1 live_ss=N ok=1`
  - `[clock.single.source.draw] x=16 y=8 w=132 h=24 ss=N live=1 ok=1`
  - `[clock.single.source.done] ok=1 boxes=1 duplicates=0 frozen=0`

## Final Clock Position
- `x=16 y=8 w=132 h=24`
- tick block: `x=124 y=12 w=16 h=16`

## Red Disabled State (Preserved)
- `close_allowed=0`
- `close_impl=0`
- `red_enabled=0`
- explicit color remains `0x00902020`

## Golden Hash
- old: `0xE61ADAACFC8334DD`
- new: `0x362B52A1FBE428C1`

Hash changed because the legacy clock source at `x=820` was removed from top-strip composition.

## Proof
Commands run:
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_single_source_fix_v1.log`

Result:
- `DAILY-DRIVER PROOF PROFILE: PASS`
- `FINAL: PASS (122 gates proved, 0 skipped, 0 faults)`

## Fault Count
- `0`

## Files Changed
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/CLOCK_SINGLE_SOURCE_FIX_V1.md`

## Manual Visual Expectation
- Exactly one visible clock box in top-left.
- Visible ticking behavior (parity-based block) with live seconds.
