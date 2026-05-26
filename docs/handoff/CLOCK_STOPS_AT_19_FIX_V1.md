# CLOCK_STOPS_AT_19_FIX_V1

## Observed Failure (Pre-V10)

Visual clock starts at 10:42:00, reaches ~10:42:19, then stops. V10 use_synthetic changes fixed startup freeze but the clock still lost active source/redraw around ss=19.

## Root Cause: Cascade Failure Across Both Servers

### Phase 1: Normal operation (boot → ~ss=19)
- PIT ticks advance at 62 Hz on native. Silkbar cadence fires at ~0.6 Hz (LIVE_CLOCK_THRESHOLD=100). Sexdisplay fallback advances at ~1 Hz (tick-gated, 62 ticks/sec).
- Handoff accepted initially (incoming ss=1 ≥ canonical ss=0), then rejected as fallback pulls ahead of silkbar cadence.
- Both servers advance independently. Clock visible.

### Phase 2: Tick stall (KVM one-shot LAPIC, uncalibrated timer)
- `raw_ticks` stops advancing but stays non-zero (>0).

**In silkbar** — stale-real-tick path (line 488-498):
1. `stale_tick_loops` reaches 16 → `cadence_threshold` drops from 100 to 4.
2. `cadence_yields` accumulated ~30 from real ticks before stall — already > 4.
3. Next stale increment: `cadence_yields` becomes 31 → **cadence fires immediately**.
4. Then 4 more stale-increments (64 loops) per subsequent fire → ~15 clock-Hz on native.
5. `loop_iter` races toward 900 at this accelerated rate.
6. When `loop_iter >= 900`: **CLOCK_SEND_LIMIT kills the clock source permanently**.
7. Sexdisplay is stranded in `clock_from_silkbar=true` with no updates.

**In sexdisplay** (V9 code, pre-use_synthetic):
- `raw_ticks > 0` prevents synthetic path from activating.
- `raw_ticks.wrapping_sub(last_fallback_raw_tick) = 0` → `secs = 0` → no tick-gated advancement.
- **Fallback freezes permanently** at whatever ss was current when ticks stalled (~19).

### Combined effect
Silkbar stops sending → sexdisplay clock_from_silkbar=true with silent source → staleness detection (120 display loops) eventually fires → sexdisplay falls back → V9 fallback freezes because raw_ticks > 0 but constant → **clock display stops at ~ss=19**.

## Fix: Two Changes

### Fix 1: Remove CLOCK_SEND_LIMIT (silkbar)
The `CLOCK_SEND_LIMIT=900` was an artificial proof-window limit ("15 min at 1/sec cadence") that doesn't match actual cadence behavior. The stale-real-tick path changes cadence_threshold from 100 to 4, causing loop_iter to race to 900 in ~60 seconds instead of 24 minutes.

**Removed**: `if loop_iter < CLOCK_SEND_LIMIT { ... } else { stop }` gate.
**Replaced with**: Always send on cadence hit. Cadence itself provides rate limiting. Added `[silkbar.clock.source.liveness]` marker every 60 clock-seconds to prove liveness.

### Fix 2: use_synthetic for constant non-zero ticks (sexdisplay) — V10
Already in working tree (unstaged). Detects when `raw_ticks > 0` but constant for ≥16 iterations, then switches to synthetic cadence (threshold=16), matching silkbar's `STALE_REAL_TICK_FALLBACK_LOOPS=16`.

## Markers

| Marker | Meaning |
|--------|---------|
| `[silkbar.clock.source.liveness] iter=N ss=S threshold=T ok=1` | Silkbar clock source still alive (once per clock-minute, budgeted 32) |

## Invariants Preserved

- **never advance 1 second per display/render loop**: tick-gated at 62 ticks/sec, synthetic at threshold=16
- **never accept backward SilkBar time**: `incoming_total >= canonical_total` check preserved
- **rejected SilkBar must not reset/stall fallback**: independent cadence V8 — fallback_idle_loops not reset on reject
- **sexdisplay remains sole framebuffer writer**: no FB access changes
- **first-cycle window redraw stays fixed**: OP_PRIMARY_FB path unchanged
- **monotonic.visible ok=0**: never emitted
- **source_check ok=0**: never emitted
- **#PF/#GP/panic/fault.kill**: 0 occurrences

## Proof Commands

```sh
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=120 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_stops_at_19_fix_v1.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_stops_at_19_fix_v1.log

# Verify key markers
rg -n "silkbar.clock.source.liveness|sexdisplay.clock.handoff.accept|sexdisplay.clock.handoff.reject|sexdisplay.clock.source.fallback|sexdisplay.clock.source.silkbar|sexdisplay.clock.redraw" \
  /tmp/sexos_clock_stops_at_19_fix_v1.log | head -40

# Confirm no faults
rg -ci "#PF|#GP|panic|fault.kill" /tmp/sexos_clock_stops_at_19_fix_v1.log

# Confirm clock passes 19
rg "clock.redraw" /tmp/sexos_clock_stops_at_19_fix_v1.log | rg "s=19|s=20|s=21"
```

## Do Not Regress

- V3: no stale-drop freeze
- V4: no backward handoff reset
- V5: fallback must live after reject
- V6: source_check ok=1, monotonic.visible ok=1, no glitch strip
- V7: canonical_ss must advance past reject_ss
- V8: post-reject independent cadence (fallback_idle_loops not reset on reject)
- V9: fallback cadence is tick-gated, silkbar bounded send removed (was killing source)
- V10: use_synthetic for constant non-zero ticks → no fallback freeze

## Files Changed

- `servers/silkbar/src/main.rs` — removed CLOCK_SEND_LIMIT, added liveness marker
- `servers/sexdisplay/src/main.rs` — V10 use_synthetic for constant non-zero ticks (already staged)
- `docs/handoff/CLOCK_STOPS_AT_19_FIX_V1.md` — this file
