# SILKBAR_REAL_TICK_CLOCK_THRESHOLD_FIX_V1

## Root Cause

SilkBar real-tick mode used a boot-fast visible threshold:

- `LIVE_CLOCK_THRESHOLD = BOOT_CLOCK_THRESHOLD`
- `BOOT_CLOCK_THRESHOLD = 8`

With fast `raw_ticks` changes under QEMU/LAPIC, `SetClock` sends occurred too often and visible time raced.

## Constants Changed

File: `servers/silkbar/src/main.rs`

- `LIVE_CLOCK_THRESHOLD`: from `BOOT_CLOCK_THRESHOLD` to `REAL_TICK_VISIBLE_CLOCK_THRESHOLD`
- Added `REAL_TICK_VISIBLE_CLOCK_THRESHOLD: u16 = STEADY_CLOCK_THRESHOLD`
- `STEADY_CLOCK_THRESHOLD` remains `100`
- `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD` remains `16`

Result: live real-tick cadence is no longer tied to boot value `8`.

## Scope Safety

- Changed only `servers/silkbar/src/main.rs` for behavior.
- Added handoff doc only.
- No sexdisplay changes.
- No kernel/scheduler/APIC changes.
- No USB/HID changes.
- No ABI/model/opcode changes.

## Proof Markers (This Run)

Build:

- `./scripts/entrypoint_build.sh` => success

Runtime log (headless run, because GUI backend unavailable in this environment):

- command: `timeout 45s ./dev.sh run-nographic > /tmp/silkbar_real_tick_threshold_fix_v1_fresh.log 2>&1`
- `[sexusb.ready]` present
- no `sexdisplay.clock.source.fallback.tick`
- no fault markers: `EXCEPTION: PAGE FAULT`, `KERNEL PAGE FAULT HALT`, `KERNEL PANIC`, `GP FAULT`, `PKU SECURITY`, `fault.kill`

Clock markers from fresh log:

- `[silkbar.clock.tick] hh=10 mm=42 ss=1 raw_ticks=0`
- `[silkbar.clock.boot_canary] send=1 threshold=16`
- `[silkbar.clock.send] hh=10 mm=42 ss=1 status=0x0`
- next sends: `ss=2,3,4` with `threshold=16` (not `8`)

Gate:

- `./scripts/daily_driver_master_gate.sh /tmp/silkbar_real_tick_threshold_fix_v1_fresh.log`
- `faults_zero PASS`
- `FINAL: PASS`

## Remaining Visual Notes

- GTK/SDL display could not be initialized in this host session (`Could not initialize SDL(No available video device)`), so visible 15s operator observation was not possible here.
- This run still demonstrates the threshold decoupling from `8` and zero-fault stability via serial markers.
