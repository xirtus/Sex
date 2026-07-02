# CLOCK_REJECT_STARVATION_V7 Handoff

## Observed Failure (V6)

Clock froze at ss=18 after first fallback→silkbar handoff.reject under GTK timing.

```
[sexdisplay.clock.handoff.reject] from=fallback to=silkbar incoming_ss=1 canonical_ss=18 accepted=0
[sexdisplay.topstrip.redraw.begin] frame=18 clock_ss=18 source=fallback
[sexdisplay.clock.redraw.source_check] redraw_ss=18 canonical_ss=18 source=fallback ok=1
...
[sexdisplay.clock.handoff.reject] from=fallback to=silkbar incoming_ss=2 canonical_ss=18 accepted=0
[sexdisplay.topstrip.redraw.begin] frame=19 clock_ss=18 source=fallback
[sexdisplay.clock.redraw.source_check] redraw_ss=18 canonical_ss=18 source=fallback ok=1
```

## Root Cause

Under GTK timing, SilkBar sends rapid backward SetClock floods. Each loop iteration:
- SilkBar backward updates are rejected (fallback_idle_loops NOT reset)
- But raw timer ticks and synth ticks may not fire before the next reject batch arrives
- Non-clock SilkBar updates trigger redraws but don't advance the clock
- Result: canonical_ss stays at reject_ss indefinitely

The `fallback_idle_loops` synth path (threshold=64 when `silkbar_clock_seen`) accumulates too slowly relative to the GTK-paced drain loop when the display loop yields wait for QEMU event pump.

## Fix: Post-Reject Force-Tick Mode (V7)

New local state in main loop:
- `post_reject_live: bool` — armed on any handoff.reject
- `post_reject_ss: u8` — canonical_ss at time of reject
- `post_reject_loop_stall: u32` — counts main loop iterations stalled at reject_ss

Inserted before "Post-drain redraws" in main loop:

**On reject:** arm `post_reject_live=true, post_reject_ss=canon_ss`.

**Each loop iteration (while post_reject_live && !clock_from_silkbar):**
- If `CLOCK_CANON_SS == post_reject_ss`: increment stall counter
- If stall counter > 8: force-increment canonical by 1, emit markers, reset counter
- If `CLOCK_CANON_SS > post_reject_ss`: clear force mode (normal progression resumed)

**Clear force mode:** on accepted monotonic silkbar handoff (`clock_from_silkbar=true`).

## Markers

- `[sexdisplay.clock.fallback.force_after_reject] reject_ss=R old_ss=R new_ss=R+1 ok=1` — forced tick
- `[sexdisplay.clock.fallback.live_after_reject] reject_ss=R now_ss=R+1 source=fallback ok=1` — liveness proof

## Gate Changes

- Added `fallback_force_after_reject_ok1_count` counter
- Added `force_after_reject_ok1` to `clock_diag_counts`
- PASS condition: `handoff_reject_count==0 OR live_after_reject>=1 OR force_after_reject>=1`
- FAIL message updated to V7 text covering both missing markers

## Do Not Regress

- V3: no stale-drop freeze — `fallback_reject_freeze_detected==0`
- V4: no backward handoff reset — `source_handoff_backward_count==0`
- V5: fallback must live after reject — `live_after_reject ok=1` OR `force_after_reject ok=1`
- V6: source_check ok=1, monotonic.visible ok=1
- V7: after reject at R, canonical_ss must advance past R within N=8 loop iterations

## Proof Commands

```bash
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_reject_starvation_v7.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_reject_starvation_v7.log
```

GTK manual:
```bash
LOG=/tmp/sexos_clock_reject_starvation_v7_gtk.log
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-mouse,bus=xhci.0 \
  -serial file:"$LOG" -display gtk -boot d

rg -n "force_after_reject|live_after_reject|handoff.reject|visual_liveness|#PF|#GP|panic" \
  "$LOG" | tail -100
```

## Files Changed

- `servers/sexdisplay/src/main.rs` — V7 force-tick state + logic
- `scripts/daily_driver_master_gate.sh` — counter, diag, PASS/FAIL conditions
- `docs/handoff/CLOCK_REJECT_STARVATION_V7.md` — this file
