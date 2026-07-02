# CLOCK_STALE_SOURCE_RECOVERY_V1

## Root Cause
`servers/sexdisplay/src/main.rs` pinned clock ownership to SilkBar after first `SetClock` (`clock_from_silkbar = true`), and fallback ticking only executed when `!clock_from_silkbar`.

The old stale detector depended on `raw_ticks` progress:
- stale check used `raw_ticks - last_silkbar_clock_ticks`
- in affected runs, `sec_now` remained `0` for long periods and tick progress could stall/repeat
- result: stale detector could fail to trip, so fallback never re-armed and the clock appeared frozen on last applied SilkBar second.

## Files Changed
- `servers/sexdisplay/src/main.rs`
- `servers/silkbar/src/main.rs`

## Why No Kernel/ABI Change
Fix is fully local to sexdisplay/silkbar userland clock-source arbitration and proof gating.
No kernel, syscall, scheduler, sex-pdx, or ABI surface changes were required.

## Stale-Source Invariant
Sexdisplay trusts SilkBar only while updates are fresh by display-loop observation.

Sexdisplay state:
- `display_loop_counter`
- `last_silkbar_msg_loop`
- `last_silkbar_second`
- `repeated_silkbar_second_msgs`
- `fallback_idle_loops`

Behavior:
1. Main loop increments `display_loop_counter`.
2. On `SetClock` apply:
- keep `clock_from_silkbar = true`
- set `last_silkbar_msg_loop = display_loop_counter`
- detect repeated `ss`; increment/reset repeat counter
3. If SilkBar is stale (`no_msg` or repeated `ss` too long), re-arm fallback (`clock_from_silkbar = false`).
4. Fallback continues from current visible `bar.clock_hh/mm/ss` (no jump backward).
5. Fallback ticks from either:
- raw tick progress, or
- synthetic loop-progress cadence when raw ticks are stalled.
6. Fresh later `SetClock` updates can re-establish SilkBar ownership.

## Forced-Stall Proof Method
A disabled-by-default proof flag was added in SilkBar using existing `option_env!` pattern:
- `SEXOS_SILKBAR_CLOCK_FORCE_STALL_PROOF=1`

When enabled:
- sends one seed `SetClock`
- sends repeated same-second `SetClock` bursts to trigger repeat-stale logic
- suppresses further `SetClock` sends to let fallback run

Normal mode (flag unset) keeps original behavior.

## Proof Commands
Forced-stall proof:
- `DAILY_DRIVER_PROBE_SECONDS=120 SEXOS_SILKBAR_CLOCK_FORCE_STALL_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_stale_source_forced_stall_long.log`

Normal restoration proof:
- `./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_stale_source_normal_restore.log`

## Marker Excerpts
From `/tmp/sexos_clock_stale_source_forced_stall_long.log`:
- `[silkbar.clock.force_stall.seed] hh=10 mm=42 ss=1`
- `[sexdisplay.clock.source.silkbar.apply] hh=10 mm=42 ss=1 loop=34`
- `[sexdisplay.clock.source.fallback.rearm] reason=stale_silkbar loop=58 last_ss=1`
- `[sexdisplay.clock.source.fallback.tick] hh=10 mm=42 ss=2`
- `[sexdisplay.clock.source.fallback.tick] hh=10 mm=42 ss=3`
- `[sexdisplay.clock.source.fallback.tick] hh=10 mm=42 ss=4`

## Proof Results
Forced-stall run:
- `FINAL: PASS (124 gates proved, 0 skipped, 0 faults)`
- required rearm and post-rearm fallback tick markers observed
- fault scan clean (`#PF/#GP/panic/fault.kill` absent)

Normal mode restoration run:
- `FINAL: PASS (124 gates proved, 0 skipped, 0 faults)`
- forced-stall markers absent when flag unset
- no persistent normal-path regression

## Remaining Risks
- Synthetic fallback cadence is loop-progress based in no-tick environments; it is monotonic and bounded but not wall-clock accurate.
