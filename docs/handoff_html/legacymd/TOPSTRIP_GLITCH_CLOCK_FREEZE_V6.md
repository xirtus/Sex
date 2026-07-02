# TOPSTRIP_GLITCH_CLOCK_FREEZE_V6

## Visual Symptom

Manual GTK boot: clock freezes visually at ~18s. A glitch/corruption strip
appears across the top chrome (SilkBar area, rows 0–50). Daily-driver serial
proof passes; GTK-specific timing exposes the defect.

## Observed Log Sequence

```
redraw_ss=17 canonical_ss=17 source=fallback ok=1
handoff.reject incoming_ss=1 canonical_ss=18 accepted=0
redraw_ss=18 canonical_ss=18 source=fallback ok=1
repeated: redraw_ss=18 canonical_ss=18 source=fallback ok=1
```

## Suspected Corruption Boundary

Two separate issues:

### 1. Clock Freeze (logical or visual)

After `handoff.reject`, `clock_from_silkbar=false` and `CLOCK_POST_REJECT_FLAG`
is set. The fallback synth tick (64-loop threshold in steady state) or the
real-tick path (`raw_ticks > last_clock_tick`) should advance the clock.

In GTK mode, QEMU timing may differ from daily-driver proof timing:
- Kernel LAPIC ticks may not advance across yields
- SilkBar may flood backward SetClock(ss=1) faster than synth threshold

Root cause not fully confirmed. The `visual_liveness` marker will distinguish:
- `ok=0` → clock truly frozen (fallback not advancing)
- `ok=1` always → clock advances logically but display not refreshing (render path)

### 2. Glitch Strip (visual artifact)

`draw_cursor_z_top` draws cursor pixels at raw `oy = surf.y.max(0)` with no
top-strip clamping. If cursor is near y=0–50 (e.g. at boot or during
interaction), cursor pixels land in the SilkBar zone and persist until the
next `redraw_top_strip` call (which only fires on clock ticks or SilkBar
updates, not on cursor moves).

## V6 Fixes Applied

1. **Pre-clear extended to row 50**: The defensive clear in `redraw_top_strip`
   now covers rows 0–50 (inclusive) instead of 0–49. Row 50 (glow edge) was
   previously omitted; any stale pixels from cursor or other writes are now
   cleared before bar render overwrites them.

2. **`CLOCK_POST_REJECT_FLAG`**: Set when `handoff.reject` fires; consumed by
   `visual_liveness` in `redraw_top_strip` to gate the `ok=0` alarm.

## V6 Markers Added

| Marker | When emitted | Meaning |
|---|---|---|
| `[sexdisplay.topstrip.redraw.begin] frame=N clock_ss=S source=...` | Entry to `redraw_top_strip` | Proves draw started |
| `[sexdisplay.topstrip.redraw.end] frame=N clock_ss=S ok=1` | Exit from `redraw_top_strip` | Proves draw completed |
| `[sexdisplay.topstrip.bounds.ok] w=W h=H pitch=0 ok=1` | First 8 per boot | FB bounds are valid |
| `[sexdisplay.topstrip.damage] reason=w_exceeds_blur_cap w=N cap=2560 ok=0` | If w > BAR_BG_W_CAP | Blur buffer incomplete |
| `[sexdisplay.clock.visual_liveness] prev_ss=P now_ss=S redraw_frame=N ok=1/0` | Every 4 redraws or on change | `ok=0` = clock frozen post-reject |

## Do Not Regress

- `sexdisplay` is the sole framebuffer writer — no other server touches FB pixels
- Framebuffer bounds checks in `redraw_top_strip` and `render` must be preserved
- `BAR_H = 50` and `BAR_BG_H = 50` constants must stay consistent
- V3/V4/V5 clock logic intact: no backward reset, no handoff starvation,
  `fallback.live_after_reject` still emitted, SilkBar accepted only monotonic

## Proof Commands

```sh
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_topstrip_glitch_v6.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_topstrip_glitch_v6.log
```

GTK manual:
```sh
LOG=/tmp/sexos_topstrip_glitch_v6_gtk.log
qemu-system-x86_64 \
  -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-mouse,bus=xhci.0 \
  -serial file:"$LOG" \
  -display gtk -boot d

rg -n "sexdisplay.clock.visual_liveness|sexdisplay.topstrip|sexdisplay.clock.handoff|fallback.live_after_reject|#PF|#GP|panic|fault" \
  "$LOG" | tail -200
```

## Gate Pass Criteria

- `topstrip_glitch_v6_redraw` = PASS (begin/end balanced, no damage ok=0)
- `topstrip_glitch_v6_bounds` = PASS (bounds.ok present)
- `topstrip_glitch_v6_liveness` = PASS (visual_liveness ok=1, no ok=0)
- `faults_zero` = PASS
- GTK visual: clock advances past 90s, no chrome glitch strip

## Files Changed

- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/TOPSTRIP_GLITCH_CLOCK_FREEZE_V6.md` (this file)
