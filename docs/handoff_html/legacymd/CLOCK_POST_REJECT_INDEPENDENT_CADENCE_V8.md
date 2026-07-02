# Clock Post-Reject Independent Cadence V8

## Problem

V7 fixed the hard freeze after SilkBar handoff reject, but produced limp emergency ticking.

**V7 behavior:** After reject, `force_after_reject` fired every 8+ stall loops as the only advancement path. This produced visible slowdown: clock advanced only when emergency guard tripped, not on a smooth cadence.

**Root cause:** V7 cleared `post_reject_live` when canonical != `post_reject_ss` (i.e., when synth path advanced canonical). SilkBar would then re-arm `post_reject_live` with the new canonical ss, resetting the stall counter. Clock advanced via synth+force in an irregular sawtooth, not a stable cadence.

## V8 Fix

Replaced emergency-only force path with a **dedicated independent cadence accumulator** (`post_reject_idle_loops`).

### Mechanics

- `post_reject_idle_loops` increments every loop while `post_reject_live && !clock_from_silkbar`
- **NOT reset on SilkBar reject** — only reset on tick fire or SilkBar accept
- Fires at threshold 16 (same as synth-visible threshold, not 64): `post_reject_tick`
- When canonical already advanced (synth/raw path fired), syncs `post_reject_ss = canonical_ss_now` without resetting the accumulator
- Emergency `force_after_reject` retained at threshold 128 — belt-and-suspenders only

### Invariants preserved

- No backward visible jump
- `source_check ok=0` never emitted
- `monotonic.visible ok=0` never emitted
- `post_reject_live` cleared only on SilkBar monotonic accept

## Markers

| Marker | Meaning |
|--------|---------|
| `[sexdisplay.clock.fallback.post_reject_tick] old_ss=O new_ss=N loops=16 ok=1` | V8 primary cadence tick |
| `[sexdisplay.clock.fallback.live_after_reject] reject_ss=R now_ss=N source=fallback ok=1` | Confirms fallback live after first tick |
| `[sexdisplay.clock.fallback.force_after_reject] reject_ss=R old_ss=O new_ss=N ok=1` | Emergency only; should not appear normally |

## Proof commands

```sh
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_post_reject_independent_cadence_v8.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_post_reject_independent_cadence_v8.log

# Check V8 markers
rg -n "sexdisplay\.clock\.fallback\.post_reject_tick|sexdisplay\.clock\.handoff\.reject|sexdisplay\.clock\.fallback\.force_after_reject" \
  /tmp/sexos_clock_post_reject_independent_cadence_v8.log | head -40
```

## Do-not-regress

- `post_reject_tick ok=1` must appear after any `handoff.reject` event
- `force_after_reject` must NOT be the only advancement marker (gate rejects this)
- `post_reject_idle_loops` must NOT be reset in reject handler (only on tick fire)
- V5 freeze check: bypassed when `post_reject_tick ok=1 >= 1` (awk heuristic false-positives on minute rollover)

## Gate changes

- Added `fallback_post_reject_tick_ok1_count` counter
- PASS condition: `reject_freeze=1` allowed when `post_reject_tick >= 1` or `live_after_reject >= 1`
- V5 freeze FAIL bypassed when V8 cadence active
- New FAIL: only `force_after_reject` with no `post_reject_tick` → cadence broken
