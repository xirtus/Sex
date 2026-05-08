# SILKBAR_CLOCK_CADENCE_FIX_V1

**Date:** 2026-05-08
**Status:** MERGED

## Symptom

SilkBar clock froze at `ss=2` (iter=2). After iter=2, cadence never completed — `[silkbar.loop.cadence.done]` stopped appearing. Clock stuck; no further `[silkbar.clock.send]` or `[silkbar.loop.iter_advance]` emitted.

## Root Causes (Three Separate Bugs)

### Bug 1 — `sys_yield()` in ERR_CAP_INVALID path created cadence-skip window

`send_update_status` originally called `sys_yield()` when `pdx_call_checked` returned `ERR_CAP_INVALID`. This yield gave the scheduler a window to deliver workspace/focus messages from silk-shell. Those messages' early `continue` statements bypassed `cadence_yields++`, so the cadence counter stalled.

**Fix:** Removed `sys_yield()` from the ERR_CAP_INVALID path in `send_update_status`. Also added `[bootgraph.edge.defer]` one-shot proof marker on first cap-miss.

### Bug 2 — Reject handlers called `sys_yield()` + `continue`, bypassing cadence accumulator

When an out-of-bounds workspace index or focus state arrived, the reject path called `sys_yield()` then `continue`. This skipped `cadence_yields = cadence_yields.wrapping_add(1)`, so any loop iteration that received a reject message produced zero cadence progress.

**Fix:** Replaced `sys_yield(); continue;` in both reject handlers with `[silkbar.cadence.no_skip]` proof markers. Execution falls through to the cadence section normally.

### Bug 3 — STEADY_CLOCK_THRESHOLD=100 activated after `loop_iter >= 10 && boot_clock_sends >= 10`

Original code switched threshold from 8 to 100 after 10 clock sends. At synthetic yield speed (no LAPIC timer in QEMU TCG), 100 yields takes a very long time with no wall-clock correlation. Clock appeared frozen because `ss` advanced only once per 100 synthetic yields.

**Fix:** Replaced conditional threshold policy with flat `LIVE_CLOCK_THRESHOLD = BOOT_CLOCK_THRESHOLD = 8`. `STEADY_CLOCK_THRESHOLD = 100` is preserved as a named constant with a tombstone reference (`let _ = STEADY_CLOCK_THRESHOLD`) for future real-timer integration. The comment explains why the switch is intentionally deferred.

## STOP FIRST: Real Clock Source

Audited `sex_pdx::get_ticks()` (syscall 34 → `TICKS.load(Relaxed)`) as a real clock source.

**Finding:** Under QEMU TCG, LAPIC timer does not fire → `TICKS` stays 0 → `get_ticks()` always returns 0. `[silkbar.clock.synthetic.threshold]` proof marker confirms this at iter=10.

**STOP FIRST decision:** Making `TICKS` nonzero requires either:
- KVM acceleration (`-enable-kvm` QEMU flag) — not a code change
- LAPIC timer delivery investigation in `kernel/src/apic.rs` / `kernel/src/interrupts.rs` — HIGH RISK, touches scheduler-critical ISR path

Neither is silkbar-only. Not implemented. Synthetic `kind=synthetic` fallback is accepted state for QEMU TCG.

## Files Changed

| File | Change |
|------|--------|
| `servers/silkbar/src/main.rs` | All three fixes above |

## Commits

| Hash | Description |
|------|-------------|
| `e00af39` | bootgraph: add v2 soft barriers for display and input edges |
| `3de9b39` | bootgraph: prove v2 soft barrier for input shell edge |
| `d51c7bd` | bootgraph: prove v2 soft barrier for usb input edge |
| `bd0c9c4` | bootgraph: prove v2 soft barrier for shell display edge |
| `7b68228` | silkbar: prove early clock cadence past iter2 |
| `ec2f590` | silkbar: keep synthetic clock live past boot thresholds |

## Cadence Constants (Post-Fix)

```rust
const BOOT_CLOCK_THRESHOLD: u16 = 8;   // yields per cadence during boot
const STEADY_CLOCK_THRESHOLD: u16 = 100; // reserved for real timer (DO NOT USE until LAPIC fires)
const LIVE_CLOCK_THRESHOLD: u16 = BOOT_CLOCK_THRESHOLD; // flat — no conditional switch
```

## Proof Markers Added

| Marker | Purpose | Budget |
|--------|---------|--------|
| `[bootgraph.edge.defer from=silkbar to=sexdisplay slot=5 reason=missing_cap]` | V2 soft barrier, first cap-miss | 1 (once) |
| `[silkbar.cadence.no_skip] reason=workspace iter=N` | Reject handler did NOT skip cadence | unbudgeted |
| `[silkbar.cadence.no_skip] reason=options iter=N` | Reject handler did NOT skip cadence | unbudgeted |
| `[silkbar.loop.cadence.count] iter=2 yield_count=Y threshold=T` | Milestone proof at iter=2 | unbudgeted (milestone only) |
| `[silkbar.clock.synthetic.threshold] iter=10 threshold=8 reason=until_real_timer` | Confirms TICKS=0 fallback | 1 (once) |
| `[silkbar.stall.after_bell_all] iter=N` | Post-Bell processing alive proof | unbudgeted |

## Deferred

- Real kernel tick source: requires LAPIC to fire under QEMU (KVM or LAPIC fix) — STOP FIRST
- STEADY_CLOCK_THRESHOLD: activate only after real clock source confirmed nonzero
