# SILKBAR UPDATE STORM FIX V1

**Date**: 2026-05-25
**Root cause**: Two synergistic bugs causing chrome/clock flicker and clock speed-up

## Primary Bug: No dedup in apply_update (silkbar-model)

`apply_update()` in `crates/silkbar-model/src/lib.rs` always returned `true` for
successfully parsed updates, even when the new value was identical to the current
bar state.  Sexdisplay then unconditionally armed `needs_top_strip_redraw = true`
for every applied update, causing a full top-strip redraw on every silkbar
message — including duplicates and the steady-state clock send.

**Positive feedback loop**: Redraw → sys_yield → silkbar gets another turn →
silkbar's synthetic-yield cadence advances faster → more SetClock sends → more
redraws → faster clock (clock appeared to speed up exactly when chrome flickered).

**Fix**: Added value-identity checks in every `apply_update` match arm.
Returns `false` (no-change) when the new value equals the current bar field,
preventing unnecessary redraws.

## Secondary Bug: Aggressive synthetic clock cadence (silkbar)

`SYNTHETIC_VISIBLE_CLOCK_THRESHOLD` was 2, meaning the clock advanced one second
every 2 yield cycles.  Under QEMU TCG (raw_ticks == 0) this produced ~15 clock
advances per second, each triggering a SetClock PDX send → sexdisplay redraw.

**Fix**: Increased `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD` from 2 to 16.
At ~30 silkbar loops/sec this yields ~2 clock advances per second — fast enough
for visible proof without triggering an update storm.

## Files Changed

1. `crates/silkbar-model/src/lib.rs` — apply_update: value-identity dedup (all 11 match arms)
2. `servers/silkbar/src/main.rs` — SYNTHETIC_VISIBLE_CLOCK_THRESHOLD: 2 → 16

## Verification

- `./scripts/entrypoint_build.sh` — PASS
- `SEXOS_QEMU_DISPLAY=gtk SEXUSB_QEMU_DEVICE=kbd SEXOS_QEMU_I8042=off ./dev.sh run` — PASS
- 0 faults (no KERNEL PANIC, no PKU SECURITY VIOLATION, no GP FAULT, no EXCEPTION)
- `[sexusb.ready]` — present
- `daily_driver_master_gate.sh` — PASS (139 pass, 0 fail, 0 faults)
- silkbar clock sends reduced from storm-frequency to ~0.5/sec

## Preserved Invariants

- No PDX ABI changes
- No framebuffer write relocation (sexdisplay sole FB writer)
- All bounds checks preserved
- No USB/HID code touched
- No Silk visual/chrome code touched
- No scheduler/kernel changes
