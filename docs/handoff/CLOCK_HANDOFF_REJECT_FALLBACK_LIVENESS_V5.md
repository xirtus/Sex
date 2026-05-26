# CLOCK_HANDOFF_REJECT_FALLBACK_LIVENESS_V5

## History

- **V3** (`fix(clock): prove fallback continuity after stale SetClock drop`): fixed stale-drop fallback continuity — fallback ticks continue after a stale incoming SetClock is dropped.
- **V4** (`fix(clock): enforce monotonic source handoff`): fixed fallback→silkbar backward reset — SilkBar incoming ss < canonical ss is rejected, fallback source preserved.
- **V5** (this): fixed fallback tick freeze after rejected handoff.

## GTK Failure V5 Exposed

Clock froze at 27 seconds. Serial log showed redraw alive but canonical_ss frozen:

```
[sexdisplay.clock.redraw.source_check] redraw_ss=26 canonical_ss=26 source=fallback ok=1
[sexdisplay.clock.handoff.reject] from=fallback to=silkbar incoming_ss=1 canonical_ss=27 reason=backward accepted=0
[sexdisplay.clock.fallback.continue_after_handoff_reject] ss=27 source=fallback
[sexdisplay.clock.redraw.source_check] redraw_ss=27 canonical_ss=27 source=fallback ok=1
[sexdisplay.clock.handoff.reject] from=fallback to=silkbar incoming_ss=2 canonical_ss=27 reason=backward accepted=0
[sexdisplay.clock.redraw.source_check] redraw_ss=27 canonical_ss=27 source=fallback ok=1
[sexdisplay.clock.handoff.reject] from=fallback to=silkbar incoming_ss=3 canonical_ss=27 reason=backward accepted=0
[sexdisplay.clock.redraw.source_check] redraw_ss=27 canonical_ss=27 source=fallback ok=1
```

## Root Cause

V4 added `fallback_idle_loops = 0` in the `else if handoff_rejected` branch. SilkBar sends
rapid backward updates (incoming_ss=1,2,3... counting up from boot), each triggering a reject.
Each reject reset `fallback_idle_loops = 0`, starving the synth tick path (threshold=64 after
`silkbar_clock_seen=true`). The synth path fires only when `fallback_idle_loops >= threshold`.
With resets arriving each outer loop, the accumulator never reached 64.

The primary tick path (`raw_ticks > last_clock_tick`) is also blocked during periods when the
hardware tick counter hasn't incremented between outer loop iterations (same timer period).
Combined with synth starvation → total freeze until next real timer tick, which may be delayed.

## Fix Rule

**Never reset `fallback_idle_loops` on handoff reject.** The synth accumulator must count
continuously across rejections. Only the accepted-handoff path may reset it (to sync cadence
to SilkBar's rhythm).

## Changes

### `servers/sexdisplay/src/main.rs`

- Removed `fallback_idle_loops = 0` from `else if handoff_rejected` block.
- Added `has_pending_reject: bool` and `pending_reject_ss: u8` state variables.
- Set `has_pending_reject = true; pending_reject_ss = canon_ss` on reject.
- Added `live_after_reject` emission in both primary tick path and synth tick path:
  fires once when `bar.clock_ss != pending_reject_ss` (ss has advanced past reject_ss).

### `scripts/daily_driver_master_gate.sh`

- Added `fallback_live_after_reject_ok1_count`: counts `live_after_reject ok=1` markers.
- Added `fallback_reject_freeze_detected`: awk detects if max canonical_ss in source_check
  never exceeded the reject_ss (clock genuinely frozen).
- PASS condition requires: `reject_freeze == 0` AND `(no rejects OR live_after_reject >= 1)`.
- FAIL branches: freeze detected, or reject seen without live_after_reject marker.

## Markers

| Marker | When |
|--------|------|
| `[sexdisplay.clock.fallback.live_after_reject] reject_ss=R now_ss=S source=fallback ok=1` | First tick after reject where ss > reject_ss |
| `[sexdisplay.clock.handoff.reject] ... accepted=0` | Each backward SilkBar update rejected |
| `[sexdisplay.clock.fallback.continue_after_handoff_reject] ss=R` | Logged synchronously on reject (V4 marker, preserved) |

## Proof Commands

```sh
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_handoff_reject_fallback_liveness_v5.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_handoff_reject_fallback_liveness_v5.log
```

GTK visual:
```sh
LOG=/tmp/sexos_clock_handoff_reject_fallback_liveness_v5_gtk.log
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-mouse,bus=xhci.0 \
  -serial file:"$LOG" -display gtk -boot d

rg -n "sexdisplay.clock.redraw.source_check|sexdisplay.clock.handoff|sexdisplay.clock.fallback.live_after_reject|sexdisplay.clock.fallback.continue_after|sexdisplay.clock.monotonic.visible|#PF|#GP|panic|fault" \
  "$LOG" | tail -240
```

## Pass Criteria

- After `handoff.reject` at `canonical_ss=R`:
  - `fallback.live_after_reject reject_ss=R now_ss=S ok=1` appears with S > R
  - `source_check canonical_ss=S` appears with S > R
  - No repeated `canonical_ss=R` forever
- No `26→1` reset in visible redraw
- `monotonic.visible ok=1` throughout
- `fallback_reject_freeze_detected=0` in gate

## Do Not Regress

1. **Never reset `fallback_idle_loops` on handoff reject.** Only reset on accepted handoff.
2. **Monotonic handoff preserved**: incoming < canonical is always rejected.
3. **V3 stale-drop continuity**: `fallback.continue_after_drop` still emitted after stale drop.
4. **V4 continue_after_handoff_reject**: marker still emitted synchronously on reject.
5. **No backward visible jump**: `source_handoff_backward_count = 0` in gate.
