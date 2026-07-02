# SILKBAR_STALE_CLOCK_SOURCE_FIX_V9

## Observed Failure (Pre-V9)

```
sexdisplay canonical/fallback clock is 10:48:ss.
silkbar keeps sending stale 10:42:ss packets:
  [sexdisplay.clock.recv] in_h=10 in_m=42 in_s=24 old_h=10 old_m=48 old_s=1
  [sexdisplay.clock.apply] source=silkbar h=10 m=48 s=1 accepted=0 reason=handoff_backward
This repeats endlessly.
```

## Root Cause

**sexdisplay's fallback clock advanced 1 second on every display loop iteration** (unbounded ~1000 Hz on native hardware). The fallback clock raced ahead of silkbar's cadence-based clock (1 send per 16-100 yields), creating a permanent gap.

Both servers start at 10:42:00 (from `DEFAULT_SILK_BAR`). Sexdisplay's unbounded fallback advanced ~360 seconds in the time silkbar advanced ~24 seconds. By the time silkbar's first SetClock arrived (10:42:24), fallback canonical was at 10:48:01+, making every incoming silkbar clock appear backward → permanent `handoff_backward` rejection.

The rejection loop caused:
- `needs_top_strip_redraw = true` every loop (via fallback advancement)
- Redraw storm on top strip every iteration
- Silkbar messages rejected forever
- `clock_from_silkbar` never becomes true

**The fallback advancement had no tick gating** — the `last_clock_tick` variable existed but was never read in the fallback path, making the tick-based cadence dead code.

## V9 Fix

### sexdisplay changes

1. **Tick-gated fallback cadence** (replaces unbounded per-loop advancement):
   - Uses `raw_ticks` (62 ticks/sec PIT) as real-time gate
   - Advances clock by `tick_delta / 62` seconds, clamped to ≤60
   - On TCG (`raw_ticks == 0`): synthetic cadence via `fallback_idle_loops` at threshold 16 (matches silkbar's `SYNTHETIC_VISIBLE_CLOCK_THRESHOLD`)
   - Cadence is NOT reset on silkbar reject (V8: independent cadence)

2. **Post-reject liveness proof**:
   - Emits `[sexdisplay.clock.fallback.live_after_reject]` on first fallback tick after handoff rejection
   - Budgeted marker (16 entries)

3. **Reject streak tracking**:
   - `handoff_reject_streak` set to 8 on rejection, saturating_sub each fallback cycle
   - Suppresses extra redraw pressure during same-ss silkbar floods
   - Emits `[sexdisplay.clock.handoff.reject.streak]` at streak=4 (budgeted 8)

4. **Continue-after-drop proof**:
   - `[sexdisplay.clock.fallback.continue_after_drop]` emitted when fallback resumes after stale drop (budgeted 8)

### silkbar changes

5. **Bounded clock send limit**:
   - After 900 cadences (~15 min at 1/sec), silkbar stops sending SetClock entirely
   - Prevents eternal flooding if handoff is permanently rejected
   - One-shot marker `[silkbar.clock.send.limit]` at limit boundary
   - Force-stall proof path (`CLOCK_FORCE_STALL_PROOF_ENABLED`) unaffected

## Markers

| Marker | Meaning |
|--------|---------|
| `[sexdisplay.clock.fallback.live_after_reject] reject_ss=R now_ss=N source=fallback ok=1` | Fallback advanced past reject point |
| `[sexdisplay.clock.handoff.reject.streak] ss=S streak=4 redraw_suppressed=1` | Redraw throttling active on repeated rejects |
| `[sexdisplay.clock.fallback.continue_after_drop] ss=N source=fallback ok=1` | Fallback resumed after stale silkbar drop |
| `[silkbar.clock.send.limit] iter=N reason=bounded_flood_gate ok=1` | Silkbar stopped sending after 900 cadences |

## Invariants Preserved

- **monotonic.visible ok=0**: never emitted
- **source_check ok=0**: never emitted
- **#PF/#GP/panic/fault.kill**: 0 occurrences
- **handoff.reject**: only transient (2 sync rejects, then accepted)
- **clock_from_silkbar**: transitions true within first few seconds
- **fallback cadence**: independent of silkbar rejects (V8 preserved)
- **FB bounds checks**: preserved
- **First-cycle window redraw**: preserved
- **No ABI change**: all PDX wire formats unchanged

## Proof Results

```
PASS gates: 271
FAIL gates: 0
SKIP gates: 115
FINAL: PASS

Before (broken):   handoff.reject = endless loop
After (fixed):     handoff.reject = 2 (transient sync, then accepted at ss=3)
                   handoff.accept = 3 (ss=3,4,5 onwards)
                   source.silkbar.apply = 23 (all accepted after handoff)
                   live_after_reject = 1
                   monotonic.visible ok=0 = 0
                   faults = 0
```

## Proof Commands

```sh
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=120 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_stale_clock_fix_v2.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_stale_clock_fix_v2.log

# Verify key markers
rg -n "handoff.reject|handoff.accept|live_after_reject|handoff.reject.streak|continue_after_drop|silkbar.clock.send.limit" \
  /tmp/sexos_stale_clock_fix_v2.log | head -20

# Confirm no faults
rg -ci "#PF|#GP|panic|fault.kill" /tmp/sexos_stale_clock_fix_v2.log

# Confirm monotonic
rg "monotonic.visible.*ok=0" /tmp/sexos_stale_clock_fix_v2.log
```

## Do Not Regress

- V3: no stale-drop freeze
- V4: no backward handoff reset
- V5: fallback must live after reject (`live_after_reject ok=1`)
- V6: `source_check ok=1`, `monotonic.visible ok=1`, no glitch strip
- V7: canonical_ss must advance past reject_ss (independent cadence)
- V8: dedicated post-reject cadence accumulator (not reset on reject)
- V9: fallback cadence is tick-gated (no unbounded per-loop advancement)

## Files Changed

- `servers/sexdisplay/src/main.rs` — tick-gated fallback cadence, reject streak tracking, live-after-reject marker, continue-after-drop proof
- `servers/silkbar/src/main.rs` — bounded clock send limit (900 cadences)
- `docs/handoff/SILKBAR_STALE_CLOCK_SOURCE_FIX_V9.md` — this file
