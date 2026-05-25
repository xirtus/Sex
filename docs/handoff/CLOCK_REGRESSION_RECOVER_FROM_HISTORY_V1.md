# CLOCK_REGRESSION_RECOVER_FROM_HISTORY_V1

## Old Known-Good Invariants Recovered
Recovered from:
- `docs/handoff/CLOCK_STALE_SOURCE_RECOVERY_V1.md`
- `docs/handoff/VISIBLE_CLOCK_REDRAW_SOURCE_FIX_V1.md`
- `docs/handoff/CLOCK_STARTUP_LIVENESS_GATE_V1.md`

Required clock protections:
1. Stale source fallback
- `clock_from_silkbar` must re-arm fallback when SilkBar is stale (`no_msg` or repeated same-second SetClock).
- Markers: `sexdisplay.clock.source.fallback.rearm`, `sexdisplay.clock.source.fallback.tick`, `sexdisplay.clock.source.silkbar.repeat`.

2. Canonical redraw latch
- Visible redraw must be canonical-state driven.
- Marker: `sexdisplay.clock.redraw.source_check` with `redraw_ss == canonical_ss`.

3. Startup liveness
- Redraw source defaults to fallback until first valid SetClock.
- Gate must report bounded first nonzero redraw and source_check equality.

## Regression Found (History vs Current)
Primary cause classification: **2) source ownership drift**.

Exact drift found in current tree before fix:
- stale-source ownership recovery logic removed from `servers/sexdisplay/src/main.rs`
- fallback rearm/tick markers absent
- redraw source policy drifted to silkbar-first
- SetClock coercion path forced updates toward synthetic `next` values in display, bypassing original producer-source truth path

This matches the user symptom class (rapid tick then freeze after unrelated work), without requiring scheduler/kernel/timebase edits.

## Files Changed
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

## Minimal Diff Summary
### `servers/sexdisplay/src/main.rs`
- Restored startup source default: `CLOCK_REDRAW_SOURCE = 1` (fallback-first).
- Restored stale-source arbitration state:
  - `clock_from_silkbar`
  - `last_silkbar_msg_loop`
  - `last_silkbar_second`
  - `repeated_silkbar_second_msgs`
  - `fallback_idle_loops`
- Restored stale rearm behavior and marker:
  - `[sexdisplay.clock.source.fallback.rearm]`
- Restored fallback ticking (raw-tick and synthetic loop cadence) and marker:
  - `[sexdisplay.clock.source.fallback.tick]`
- Restored repeated SetClock detection marker:
  - `[sexdisplay.clock.source.silkbar.repeat]`
- Removed SetClock coercion path in display (`apply_arg1/apply_arg2` override to synthetic `next`).
- Preserved canonical latch and redraw source-check path.

### `scripts/daily_driver_master_gate.sh`
- Hardened `clock_visible_seconds` with rapid-tick guard:
  - computes second-advance density from first 64 redraw markers
  - fails on runaway advance pattern (`rapid_tick`)
  - reports `rapid_tick_advances`, `line_span`, `min_delta`
- Added explicit `clock_cadence_bound` gate:
  - compares redraw-line delta vs visible second delta
  - fails on excessive second advance relative to redraw window
- Wired cadence gate into integrated interaction dependency checks.

## Proof Commands Run
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_regression_recover_v1.log`
- `./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_regression_recover_v1_fixed.log`

## Runtime Evidence / Blocker
Environment blocker prevented runtime marker capture in both runs:
- `qemu-system-x86_64: -qmp unix:/tmp/sexos_qmp.sock,server,nowait: Failed to bind socket to /tmp/sexos_qmp.sock: Operation not permitted`

Observed result:
- build: PASS
- runtime log: truncated (1-2 lines)
- marker scans for clock lines: empty

Therefore full runtime proof rows requested (first redraw, first nonzero, first silkbar apply, first fallback tick, rearm, last redraw before freeze, source at freeze) are **not observable in this environment**.

## Remaining Risk
- Synthetic clock remains synthetic (not real wall-clock/timebase).
- Final visual cadence/freeze behavior still requires a run where QEMU/QMP logging is permitted so clock markers can be observed end-to-end.

## CLOCK_REGRESSION_PROOF_NO_QMP_V1 (2026-05-26)
### QMP blocker root cause
- Host/sandbox policy forbids AF_UNIX bind in `/tmp` for this session.
- Confirmed with direct bind probe:
  - `bind_fail PermissionError(1, 'Operation not permitted')`
- Not a stale socket, ownership, or collision issue:
  - `/tmp/sexos_qmp.sock` absent before run
  - unique socket path still fails (`/tmp/sexos_qmp_clock_regression_*.sock`)

### Harness-only proof change
- File: `scripts/run_daily_driver_proof.sh`
- Added host-only controls (no clock logic change):
  - `SEXOS_QMP_SOCK` override support (default `/tmp/sexos_qmp.sock`)
  - stale-socket cleanup before launch when QMP enabled
  - `SEXOS_PROOF_QMP=0` serial-only fallback lane (disables `-qmp` and injection)

### Proof method used
1. Normal QMP lane + stale cleanup -> failed bind `Operation not permitted`
2. Unique QMP sock override -> failed bind `Operation not permitted`
3. Serial-only fallback:
   - `SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=90 ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_regression_serial_no_qmp_v1.log`

### Exact proof result
- `/tmp/sexos_clock_regression_recover_v1_fixed.log`: 1 line (truncated; unusable for clock proof)
- `/tmp/sexos_clock_regression_serial_no_qmp_v1.log`: 95,900 lines (real runtime log)
- Gate result: `FINAL: FAIL (1 gate(s) failed)`
- Unrelated failure: `linen_sexobject_native_persist FAIL` (outside this clock mission)
- Clock gates:
  - `clock_visible_seconds PASS`
  - `clock_cadence_bound PASS`
  - `faults_zero PASS`

### Marker evidence summary
- `sexdisplay.clock.redraw.source_check` repeatedly shows `redraw_ss == canonical_ss`
- Fallback lane present (`sexdisplay.clock.source.fallback.tick`) and silkbar apply lane present (`sexdisplay.clock.source.silkbar.apply`)
- Clean fallback -> silkbar ownership handoff observed
- No rapid runaway by gate:
  - `rapid_tick_advances=40 line_span=21175 min_delta=65`
  - `clock_cadence_bound PASS redraw_delta=21354 second_delta=14 limit=10678`
- No `#PF`, `#GP`, `panic`, `KERNEL PANIC`, or `fault.kill` in required scan output
- No runaway/freeze evidence in the scanned marker window

### Patch acceptance decision
- Clock-lane status: **PASS** in this environment, based on serial-only runtime proof with passing clock gates and marker consistency.
- Full daily suite status: **FAIL** due unrelated `linen_sexobject_native_persist` gate.
- Do not claim whole OS PASS from this run.

### Commit recommendation
- Commit harness + clock recovery changes only if accepted in a **clock-lane pass / suite-fail** state:
  - clock lane proven
  - unrelated Linen gate still failing
- Recommended commit scope:
  - `scripts/run_daily_driver_proof.sh`
  - clock recovery files already changed in this mission chain
  - this handoff doc

### Next action
1. Fix or restore `linen_sexobject_native_persist` in a separate mission.
2. Alternatively, run an accepted lane mode with Linen proof disabled if that mode is explicitly approved.
