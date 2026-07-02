# SILKBAR_LIVENESS_FALLBACK_V1

**Date:** 2026-05-03
**Status:** MERGED

## Symptom

If SilkBar stalls or dies after sexdisplay has accepted its clock (`clock_from_silkbar = true`), the clock display freezes permanently at the last received time. The fallback clock is a one-way latch — once disabled by the first non-stale SetClock, it never re-enables.

This is the same symptom as CLOCK_FREEZE_FALLBACK_GATE_V1, but triggered by a different failure mode: SilkBar stall (crash, livelock, or silent SPSC ring loss) rather than stale boot-time message + lost messages.

## Root Cause

Sexdisplay's `clock_from_silkbar` is a one-way latch (`bool`). Once set to `true` by a non-stale SetClock, the fallback clock path (`if !clock_from_silkbar`) is dead code forever. If SilkBar stops sending SetClock messages:

1. `last_clock_second` freezes at the value when `clock_from_silkbar` transitioned to `true`
2. The fallback block (lines 606–614) is skipped
3. `bar.clock_hh/mm/ss` never changes
4. The clock display freezes silently — no marker, no diagnostic

## Fix

Add a liveness timeout in sexdisplay's main loop. If `clock_from_silkbar == true` and no fresh SetClock has been received for >5 seconds (measured in `sec_now = get_ticks() / 62` ticks), the fallback is re-enabled.

### Changes to `servers/sexdisplay/src/main.rs`

1. **New variable** (line 579):
   ```rust
   let mut last_silkbar_clock_sec: u64 = 0;
   ```

2. **Liveness timeout** (after `sec_now` computation, before fallback block):
   ```rust
   if clock_from_silkbar && sec_now.saturating_sub(last_silkbar_clock_sec) > 5 {
       clock_from_silkbar = false;
       // Budgeted: first 4 fallback-resume events.
       unsafe {
           static mut CLOCK_FALLBACK_RESUME_BUDGET: u32 = 4;
           let remaining = &mut CLOCK_FALLBACK_RESUME_BUDGET;
           if *remaining > 0 {
               *remaining -= 1;
               serial_println!("[sexdisplay.clock.fallback.resume] reason=silkbar_stale");
           }
       }
   }
   ```

3. **Freshness tracking** in SetClock handler:
   - When `clock_from_silkbar == false` and a non-stale SetClock arrives: set `clock_from_silkbar = true` AND `last_silkbar_clock_sec = sec_now`
   - When `clock_from_silkbar == true` (already trusting SilkBar): update `last_silkbar_clock_sec = sec_now` on every SetClock (resets the timeout)
   - Stale/rejected SetClock does NOT update `last_silkbar_clock_sec` (does not reset timeout)

### Timeout reset round-trip

When the timeout fires and fallback resumes:
1. `clock_from_silkbar = false`
2. Fallback block computes time from `get_ticks()/62` — catches up instantly
3. When SilkBar recovers and sends SetClock, the stale-time gate (CLOCK_FREEZE_FALLBACK_GATE_V1) checks freshness
4. Since both silkbar and sexdisplay use the same `get_ticks()/62` timebase, the times match → gate accepts → `clock_from_silkbar = true` again

## Changed Invariants

1. `clock_from_silkbar` is no longer a permanent one-way latch. It can revert to `false` after a 5-second liveness timeout.
2. Every SetClock resets the liveness timer while `clock_from_silkbar == true`. Only fresh (non-stale) SetClock resets it during the fallback→silkbar transition.
3. The stale-time gate still prevents boot-time stale messages from prematurely disabling the fallback.
4. The liveness timeout and stale-time gate compose: the gate protects against stale takes, the timeout protects against stalled senders.

## Marker List

| Marker | Type | Budget | When |
|--------|------|--------|------|
| `[sexdisplay.clock.fallback.resume]` | accept | 4 | SilkBar clock stalls >5s, fallback re-enabled |

## Verification

```bash
# Build
./scripts/entrypoint_build.sh

# Boot (nographic)
timeout 25 ./dev.sh run-nographic 2>/dev/null | tee serial_liveness.log

# Check fallback resume (should be 0 in healthy boot)
grep -c 'sexdisplay.clock.fallback.resume' serial_liveness.log

# Check clock sends (should show ss=1..12)
grep 'silkbar.clock.send' serial_liveness.log

# Check for errors (should be 0)
grep -cE 'fault|panic|silkde.m2.assert.bad' serial_liveness.log

# Check for drops (should be 0)
grep -c 'silkbar.send_update.drop' serial_liveness.log
```

## Verified Results (2026-05-03)

```
sexdisplay.clock.fallback.resume:    0 (no fallback resume in healthy boot)
silkbar.clock.send ss=1..12:         12 (clock advances, no freeze)
silkde.m2.assert.bad:                0 (no update errors)
fault/panic:                         0 (no exceptions)
silkbar.send_update.drop:            0 (no dropped messages)
interaction transitions:             16 (proof markers preserved)
silkbar clicks:                       7 (proof markers preserved)
drag proof stages:                    5 (proof markers preserved)
```

## Deferred Items

- **SilkBar crash detection**: This patch handles the *symptom* (frozen clock) but does not detect or report SilkBar crashes. A future watchdog or health-check PD could monitor silkbar liveness and restart it.
- **SPSC→MPSC ring buffer fix**: The kernel's `RingBuffer` (SPSC) loses messages under multi-producer load. If the ring loss causes the >5s gap, the timeout fires and fallback resumes, but the root cause (ring loss) remains.
- **Calibrate LAPIC_TICKS_PER_SECOND_APPROX**: Currently 62, not wall-clock accurate. Both silkbar and sexdisplay use the same value, so they agree on monotonic time.

## STOP FIRST Conditions

1. Changes to kernel IPC (SPSC ring, syscall dispatch, capability routing)
2. Changes to sex-pdx library (PDX call model, slot definitions)
3. Changes to silkbar-model crate (UpdateKind, apply_update, ABI constants)
4. Adding new IPC opcodes or ABI fields
5. Changing the LAPIC tick calibration value
6. Removing the fallback clock from sexdisplay
7. Changing the timeout value (5 seconds) without prior analysis
8. Adding a watchdog subsystem or new PD for liveness monitoring
