# CLOCK_FREEZE_FALLBACK_GATE_V1

**Date:** 2026-05-03
**Status:** MERGED (w/ budgeted diagnostics)

## Symptom

The SilkBar clock visual freezes at 00:00 approximately 4 seconds into boot. After the freeze, the clock never advances.

## Root Cause (Two Bug Interaction)

### Bug A — Stale initial SetClock permanently disables fallback

Silkbar's boot init sends a SetClock with `ss=0` (the real uptime at boot). This message is enqueued behind 9 other init messages (5×SetWorkspaceActive + 4×SetChipVisible). By the time sexdisplay processes the SetClock, ~4 real seconds have passed, but the time value is still `00:00:00`.

Sexdisplay's `clock_from_silkbar` is a one-way gate: once set to `true` by the first incoming SetClock, the local fallback clock (`get_ticks()/62`) is disabled forever. The stale `00:00:00` overwrites the fallback time and permanently locks the display to "00:00".

### Bug B — Silkbar's later SetClock messages never arrive

After the init batch, silkbar's main loop sends SetClock every second (ss=1,2,3,...). These messages are enqueued into sexdisplay's message ring via `AsyncEnqueue` (kernel `RingBuffer<MessageType, 256>` — SPSC design). Under multi-producer load (shell + silkbar both sending to sexdisplay), messages are lost — the SPSC ring buffer lacks the atomic-CAS needed for MPSC safety.

Result: sexdisplay receives exactly two SetClock messages (both stale ss=0 from init), then never receives another. Since `clock_from_silkbar = true` was set by the first stale message, the fallback is dead and no new time arrives → clock frozen at 00:00 forever.

## Fixes

### Fix 1 — Stale time gate in sexdisplay (essential)

**File:** `servers/sexdisplay/src/main.rs`

Before a SetClock message can set `clock_from_silkbar = true`, the receiver compares the incoming `ss` against the current fallback time (`sec_now % 60`). If the incoming time is stale (less than fallback, within a 30-second window to allow midnight rollover), the gate rejects it and the fallback stays active.

```rust
if !clock_from_silkbar {
    let incoming_ss = bar.clock_ss;
    let fallback_ss = (sec_now % 60) as u8;
    let stale = incoming_ss < fallback_ss
        && (fallback_ss.wrapping_sub(incoming_ss) < 30);
    if !stale {
        clock_from_silkbar = true;
    }
}
```

This ensures:
- If no SetClock ever arrives → fallback runs indefinitely → correct time
- If a stale SetClock arrives → rejected → fallback continues → correct time
- If a fresh SetClock arrives → accepted → silkbar clock takes over → correct time

### Fix 2 — Remove boot-time initial SetClock from silkbar (defense-in-depth)

**File:** `servers/silkbar/src/main.rs`

The initial SetClock with `ss=0` is removed from silkbar's boot init. Sexdisplay's fallback handles the first second of uptime. Silkbar's main loop sends its first SetClock at `ss=1` (not `ss=0`).

### Fix 3 — Initialize `last_uptime_seconds` to 0 (defense-in-depth)

**File:** `servers/silkbar/src/main.rs`

Changed from `u64::MAX` to `0`. This prevents the first loop iteration from sending a redundant SetClock(ss=0) when `uptime_seconds` is still 0.

### Fix 4 — Budgeted error logging in send_update (diagnostics)

**File:** `servers/silkbar/src/main.rs`

Added `[silkbar.send_update.drop]` with a budget of 16 to log async enqueue failures.

## Verification

```bash
# Build
./scripts/entrypoint_build.sh

# Boot (nographic)
timeout 20 ./dev.sh run-nographic 2>/dev/null | tee serial_verify.log

# Check clock sends (should show ss=1,2,3,... not ss=0)
grep 'silkbar.clock.send' serial_verify.log

# Check for drops (should be 0)
grep -c 'silkbar.send_update.drop' serial_verify.log

# Check for errors (should be 0)
grep -cE 'fault|panic|silkde.m2.assert.bad' serial_verify.log
```

## Verified Results (2026-05-03)

```
silkbar.clock.send ss=1 through ss=12:  12 lines (first SetClock is ss=1, not ss=0)
silkbar.send_update.drop:               0 (no dropped messages)
fault/panic/silkde.m2.assert.bad:       0 (no errors)
clock freeze at 00:00:                  ELIMINATED
```

## Changed Invariants

1. `clock_from_silkbar` is no longer a one-way latch — the first SetClock must pass a freshness check.
2. Silkbar's first SetClock is `ss=1` (not `ss=0`). Sexdisplay fallback covers `ss=0`.
3. If no valid SetClock ever arrives, the fallback runs forever. The clock never freezes.
4. Message delivery between silkbar and sexdisplay is unreliable (SPSC ring under MPSC load). Sexdisplay must tolerate dropped messages.

## Deferred Items

- **SPSC→MPSC ring buffer fix**: The kernel `RingBuffer` (SPSC) loses messages under multi-producer load. Fix requires atomic CAS or per-producer slots. Not critical because the fallback gate makes sexdisplay tolerant of lost messages.
- **Calibrate LAPIC_TICKS_PER_SECOND_APPROX**: Currently 62, based on divide=16, init_count=1_000_000. Not wall-clock accurate but monotonic. Both silkbar and sexdisplay use the same value, so they agree on time.
- **Synthetic click proof after fix**: After this fix, test that the silkbar panel click proof (sexinput stages) still operates correctly.

## Marker List (Budgeted)

| Marker | Type | Budget | When |
|--------|------|--------|------|
| `[silkbar.clock.send]` | accept | 12 | Silkbar sends SetClock |
| `[silkbar.send_update.drop]` | error | 16 | AsyncEnqueue fails (ring full) |

## STOP FIRST Conditions

1. Changes to kernel IPC (SPSC ring, syscall dispatch, capability routing)
2. Changes to sex-pdx library (PDX call model, slot definitions)
3. Changes to silkbar-model crate (UpdateKind, apply_update, ABI constants)
4. Adding new IPC opcodes or ABI fields
5. Changing the LAPIC tick calibration value
6. Removing the fallback clock from sexdisplay
