# CLOCK_VISUAL_CADENCE_CALIBRATION_V1

## Status: PASS (272 gates, 0 failures)

## Root Cause

The visual clock under GTK/KVM ticked extremely slowly (~0.5 Hz) because of two misaligned thresholds:

1. **silkbar `STEADY_CLOCK_THRESHOLD = 100`**: With the 62 Hz PIT, each tick change increments `cadence_yields` by 1. Threshold of 100 meant 100 PIT ticks per logical second = 100/62 ≈ 1.61 seconds real time per clock advance. This made the master clock source run at ~0.62 Hz.

2. **silkbar `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD = 16` and sexdisplay `FALLBACK_SYNTHETIC_THRESHOLD = 16`**: On TCG (raw_ticks==0), both servers use synthetic loop-counting cadence. At 16 loops/advance, with a slow GTK loop rate, the clock advanced at <1 Hz. Additionally, the sexdisplay `FALLBACK_STALE_REAL_THRESHOLD = 16` caused false-positive stall detection on fast loops (>1000 Hz), where 16 iterations is normal between 62Hz PIT ticks, causing premature synthetic fallback.

## Changes Made

### Threshold Constants (Final Calibrated Values)

| Constant | File | Old | New | Rationale |
|---|---|---|---|---|
| `STEADY_CLOCK_THRESHOLD` | silkbar/main.rs | 100 | 62 | Match PIT 62 Hz for ~1 Hz live clock |
| `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD` | silkbar/main.rs | 16 | 8 | Conservative reduction; matches sexdisplay |
| `FALLBACK_SYNTHETIC_THRESHOLD` | sexdisplay/main.rs | 16 | 8 | Must match silkbar to prevent handoff storm |
| `FALLBACK_STALE_REAL_THRESHOLD` | sexdisplay/main.rs | 16 | 256 | Prevent false stall on fast loops (>1000 Hz) |

silkbar cascading: `STEADY_CLOCK_THRESHOLD` → `REAL_TICK_VISIBLE_CLOCK_THRESHOLD` → `LIVE_CLOCK_THRESHOLD`. All now 62.

silkbar stale fallback: `STALE_REAL_TICK_FALLBACK_THRESHOLD` unchanged at 4.

### Cadence Sample Markers

All rate-limited via budgeted static mut counters (32 samples each).

1. **`[sexdisplay.clock.cadence.sample] ss=S loops=L raw_delta=D synthetic=0/1 threshold=T source=fallback ok=1`**
   — Emitted on every fallback clock advance.

2. **`[sexdisplay.clock.post_reject.cadence.sample] ss=S loops=L threshold=T ok=1`**
   — Emitted when fallback advances while `handoff_reject_streak > 0` (budgeted, 16 samples).

3. **`[silkbar.clock.cadence.sample] ss=S yields=Y threshold=T sent=1 ok=1`**
   — Emitted on every silkbar clock send.

### Parity Marker

**`[sexdisplay.clock.cadence.parity] fallback_threshold=F post_reject_threshold=P ok=1`**
— One-shot on first fallback advance. Proves both thresholds equal.

### Same-Second Redraw Suppression

**`[sexdisplay.clock.redraw.skip_same_second] ss=S reason=no_clock_delta ok=1`**
— Guard prevents clock-only top-strip redraw when canonical_ss unchanged since last draw. Uses `LAST_REDRAWN_CLOCK_CANON_SS` tracking (budgeted, 8 samples fallback path, 4 samples stale_drop path).

### Gate Script

Added `clock_cadence_parity` gate check:
- PASS: cadence.parity ok=1 exists, ok=0 count is 0
- FAIL: ok=1 exists but ok=0 also exists
- SKIP: no cadence.parity marker
- Integrated into `req_clock` and `dep_fail` dependency chains

## Proof Results

```
clock_visible_seconds        PASS
clock_cadence_bound          PASS
clock_cadence_parity         PASS (fallback=8 post_reject=8)
faults_zero                  PASS (0 fault markers)
FINAL: PASS (272 gates proved, 115 skipped, 0 faults)
```

No source_check ok=0, no monotonic.visible ok=0, no #PF/#GP/panic.

## Do-Not-Regress Rules

1. **Never increase `STEADY_CLOCK_THRESHOLD` above 62** — slows master clock below 1 Hz.
2. **Never decouple sexdisplay `FALLBACK_SYNTHETIC_THRESHOLD` from silkbar `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD`** — mismatch causes permanent handoff rejection.
3. **Never reduce `FALLBACK_STALE_REAL_THRESHOLD` below 128** — false stall detection on fast loops.
4. **Never remove cadence.parity marker** — primary diagnostic for threshold alignment.
5. **Never increase `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD` above 16** — clock becomes visually too slow.
6. **Never reduce `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD` below 4** — clock races ahead of silkbar.

## Safety Invariants Preserved

- No visible backward jump (monotonic.visible ok=1 enforced)
- No source_check ok=0 (canonical_ss matches displayed ss)
- No accepted lower SilkBar time (handoff.reject guards)
- No freeze at 18/19/27/28 (fallback continues independently)
- No fast-forward jump (one second per advance)
- Clock passed 19/28/60 in long proof

## Proof Commands

```bash
# Build
./scripts/entrypoint_build.sh

# Daily driver gate (headless)
SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=240 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_visual_cadence_calibration_v2.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_visual_cadence_calibration_v2.log

# GTK visual verification
LOG=/tmp/sexos_clock_visual_cadence_calibration_v2_gtk.log
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

# Diagnostic markers
rg -n "clock.cadence.sample|clock.post_reject.cadence.sample|clock.cadence.parity|clock.redraw.skip_same_second|sexdisplay.clock.redraw.source_check|sexdisplay.clock.handoff|monotonic.visible|#PF|#GP|panic|fault.kill" \
  "$LOG" | tail -500
```

## Files Changed

| File | Changes |
|---|---|
| `servers/sexdisplay/src/main.rs` | FALLBACK_SYNTHETIC_THRESHOLD 16→8, FALLBACK_STALE_REAL_THRESHOLD 16→256, added cadence/parity/same-second markers, LAST_REDRAWN_CLOCK_CANON_SS guard, advance tracking vars |
| `servers/silkbar/src/main.rs` | STEADY_CLOCK_THRESHOLD 100→62, SYNTHETIC_VISIBLE_CLOCK_THRESHOLD 16→8, cadence.sample marker in send path |
| `scripts/daily_driver_master_gate.sh` | clock_cadence_parity gate check, req_clock integration |
| `docs/handoff/CLOCK_VISUAL_CADENCE_CALIBRATION_V1.md` | This document |
