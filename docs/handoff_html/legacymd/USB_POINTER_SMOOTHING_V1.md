# USB_POINTER_SMOOTHING_V1

## Status: PASS (build verified)

## Root Cause

The USB tablet pointer path emitted raw screen-space deltas clamped to ±64 as EV_REL events with no temporal smoothing. The silk-shell receiver then applied its own transfer function (micro 1:1, medium /2, large→18). This two-stage pipeline with independent per-packet clamping produced chunky/bursty movement:

- Producer emitted up to 64 px delta per report
- Shell saturated any delta >16 to 18 px
- Single fast tablet reports produced 18 px jumps
- No sub-pixel residual tracking — small movements alternated between 0 and large jumps

## Implemented Fix

**File changed:** `servers/sexinput/src/main.rs` (only)

Added a deterministic integer-only residual accumulator smoothing layer in the ABS→EV_REL conversion path (`normalize_pointer_report_v1`), before EV_REL emission:

### Constants
- `SMOOTH_LIMIT = 24` — maximum px emitted per tick (was 64)
- `SMOOTH_DIV = 3` — integer division dampening factor
- Accumulator cap = `LIMIT * DIV * 2 = 144` — prevents unbounded wind-up

### Algorithm
```
ACC_X += raw_dx                    // accumulate screen-space delta
ACC_X = clamp(ACC_X, -144, 144)    // cap to prevent wind-up
emit_dx = clamp(ACC_X / 3, -24, 24)  // dampened output
ACC_X -= emit_dx * 3               // remove emitted contribution, keep residual
```

Same for Y axis. Zero-motion packets (emit_dx=0 && emit_dy=0) are still dropped unless button state changes.

### Effect on the pipeline
- Small movements (1-2 px) accumulate over 2-3 ticks → emit 1 px → shell passes 1:1
- Medium movements: dampened by /3, smooth graduation
- Large/fast movements: spread over multiple ticks at max 24 px/tick
- Shell transfer_axis unchanged: receives gentler values, secondary clamping becomes lighter-touch
- Wind-up bound: max 2 ticks of residual movement after input stops (48 px emitted → 36 px cursor movement after shell's 18 cap)

## Markers Added
- `[usb.pointer.smooth] raw_dx=… raw_dy=… acc_x=… acc_y=… emit_dx=… emit_dy=… limit=24 div=3 ok=1` (budget 128)
- `[usb.pointer.smooth.pass] ok=1` (budget 64)

## Existing Markers Preserved (values updated to emit_dx/emit_dy)
- `[usb.pointer.producer.evrel]`
- `[usb.tablet.delta.clamp]`
- `[usb.tablet.abs.delta]`
- `[usb.tablet.emit.delta_only]`
- `[usb.tablet.emit.raw_blocked]`
- `[sexinput.pointer.forward.reason=…]`

## Shell Markers (unchanged)
- `[usb.pointer.shell.recv.evrel]`
- `[usb.pointer.shell.apply]`
- `[usb.pointer.cursor.bounds]`
- `[shell.pointer.filter.v2]`
- `[shell.rel.transfer]`
- `[cursor.motion.bounds] source=rel`

## Files Changed
- `servers/sexinput/src/main.rs` — replaced DELTA_CLAMP=64 with SMOOTH_LIMIT=24 + SMOOTH_DIV=3 + residual accumulators

## Files NOT Changed
- `servers/silk-shell/src/main.rs` — shell transfer_axis already correct; no redundant clamp to remove
- `servers/sexdisplay/src/main.rs` — no changes needed
- Kernel, ABI, sex-pdx — no changes

## Build Verification
```
bash scripts/entrypoint_build.sh  → SUCCESS
cargo check -p sexinput --target x86_64-sex.json -Zbuild-std=core,alloc,compiler_builtins → SUCCESS
```

## Live Proof (pending human)
```bash
QEMU_PRINT_CMD=0 SEXUSB_QEMU_DEVICE=tablet-display-sdl SEXOS_QEMU_DISPLAY=sdl ./dev.sh 2>&1 | tee /tmp/sexos_usb_pointer_smoothing_v1.log
```
Expected markers in log:
```
[usb.pointer.smooth] raw_dx=… raw_dy=… acc_x=… acc_y=… emit_dx=… emit_dy=… limit=24 div=3 ok=1
[usb.pointer.smooth.pass] ok=1
[usb.pointer.producer.evrel] dx=… dy=… buttons=… ok=1
[usb.pointer.shell.recv.evrel] dx=… dy=… ok=1
[usb.pointer.shell.apply] x=… y=… dx=… dy=… ok=1
[usb.pointer.cursor.bounds] x=… y=… ok=1
[usb.hid.pointer.click.recv] left=1 value=1 ok=1
```
No #PF/#GP/panic/fault.kill expected.

## Deferred Work / STOP FIRST Notes
- If cursor still feels too sluggish, increase SMOOTH_LIMIT to 32 or decrease SMOOTH_DIV to 2
- If wind-up is noticeable (cursor continues moving after tablet stops), reduce accumulator cap from 2x to 1x
- Tuning constants are at lines 106-107 of sexinput/src/main.rs
- Shell transfer_axis (line 8838) provides secondary dampening — could be simplified if producer smoothing is sufficient, but DON'T without separate task
- No kernel/ABI changes needed for further tuning
