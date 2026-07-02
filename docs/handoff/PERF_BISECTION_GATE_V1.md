# PERF_BISECTION_GATE_V1 — Measurable Slowness Gate for git bisect

Date: 2026-07-02
Result: **PASS** (gate created, git-bisect-ready as log parser; no Rust changes)

## What this is

`scripts/perf_bisection_gate.sh <serial-log>` — parses a QEMU serial log and
classifies the run for `git bisect run`. **Parser only**: it does not build
or run QEMU. No server/kernel code changed.
`scripts/input_control_quality_gate.sh` untouched (fallback parsing not needed).

## Exit codes (git-bisect semantics)

| Code | Meaning |
|------|---------|
| 0    | GOOD — thresholds met, Chapter 1 intact, no faults |
| 1    | BAD/SLOW — threshold exceeded or fault present |
| 2    | CHAPTER_1_REGRESSION — input chain marker missing (bisect treats as bad) |
| 125  | UNTESTABLE — log missing/empty, boot never reached PD spawn, or commit predates trace instrumentation (bisect skip) |

## Metrics reported

1. `[perf.gate.boot]` — pd_spawn_begin/ok counts + line span.
   **boot_to_all_pds_spawn time = unavailable**: serial log has no wall-clock
   or global-tick marker at spawn points (`elapsed_ticks=6` per PD is
   loader-local). Count + line span reported instead.
2. `[perf.gate.sched]` — scheduler.tick.enter / pick_next / yield_and_switch.saved counts.
3. `[perf.gate.usb]` — sexusb.hid.transfer.event / rearm.ok counts.
4. `[perf.gate.shell]` — applies/sends from last `[input.trace.shell.summary]`
   (fallback: raw marker counts), budget_hit.
5. `[perf.gate.display]` — recv/draws/presents from last
   `[input.trace.display.summary]` (fallback: raw counts), budget_hit.
6–8. `[perf.gate.ratio]` — send_to_recv, recv_to_draw, draw_to_present.
9. `[perf.gate.latency]` — seq-joined tick chains, max total_logical
   (apply→send + recv→draw + draw→present in logical ticks).
10. `[perf.gate.logvolume]` — total lines + top noise markers
    (kernel.mem.boot_frame.alloc, linen.session.reject,
    scheduler.yield_and_switch.saved).
11. `[perf.gate.faults]` — #PF, #GP, panic, fault.kill, reboot loop, freeze,
    storm (>5000 shell applies).

## BAD thresholds (env-overridable)

| Threshold | Default | Env var | Current log |
|-----------|---------|---------|-------------|
| send_to_recv ratio | > 2.0 | `MAX_SEND_TO_RECV` | 1.03 OK |
| recv_to_draw ratio | > 2.0 | `MAX_RECV_TO_DRAW` | **2.91 BAD** |
| draw_to_present ratio | > 2.0 | `MAX_DRAW_TO_PRESENT` | 1.00 OK |
| max total_logical (if chains>0) | > 4 | `MAX_INPUT_TO_PRESENT` | 2 OK |
| any fault | — | — | clean |

All thresholds supportable from `logs/qemu-latest.log` — no PARTIAL_STOP_FIRST.
`na` ratios (denominator 0) are not BAD by themselves; a log with zero trace
markers exits 125 instead.

## Current log verdict (2026-07-02, logs/qemu-latest.log)

```
[perf.gate.boot] pd_spawn_begin=14 pd_spawn_ok=14 spawn_line_span=66307..68185
[perf.gate.sched] tick_enter=32 pick_next=32 yield_and_switch=22965
[perf.gate.usb] transfer_events=32 rearms=32
[perf.gate.shell] applies=32 sends=33 budget_hit=1
[perf.gate.display] recv=32 draws=11 presents=11 budget_hit=1
[perf.gate.ratio] send_to_recv=1.03 recv_to_draw=2.91 draw_to_present=1.00
[perf.gate.latency] chains=3 max_total_logical=2
[perf.gate.logvolume] total_lines=158283 boot_frame_alloc=67790 linen_session_reject=61269 scheduler_yield=22965
[perf.gate.faults] pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0
PERF_BISECTION_GATE_V1: BAD ( recv_to_draw(2.91>2.0))   exit=1
```

**Current HEAD is BAD by design** — recv_to_draw 2.91 matches the known
display-redraw-cadence bottleneck (INPUT_PRESENT_TICK_TRACE_V1). This is what
makes bisect meaningful: find the commit where recv_to_draw crossed 2.0
(or confirm it was always ≥2.9 → optimization mission, not regression hunt).

## Bisect usage

```sh
git bisect start
git bisect bad HEAD
git bisect good <known-fast-commit>
git bisect run scripts/perf_bisection_gate.sh logs/qemu-latest.log
```

**Caveat**: the gate only parses; for true automation each bisect step must
rebuild and rerun QEMU to regenerate `logs/qemu-latest.log` first. Wrap in a
runner, e.g.:

```sh
#!/usr/bin/env bash
./scripts/entrypoint_build.sh || exit 125
<qemu run lane with enum.done-synced injection producing logs/qemu-latest.log> || exit 125
exec scripts/perf_bisection_gate.sh logs/qemu-latest.log
```

Commits predating INPUT_PRESENT_TICK_TRACE_V1 instrumentation auto-skip
(exit 125 — no trace markers), so bisect range effectively bounded below by
commit `d9f521ac` (trace correlation) era.

## Missing markers (for future instrumentation, NOT blockers)

- **Wall-clock/global-tick at PD spawn**: no time-based boot_to_all_pds_spawn
  possible. Would need a kernel-side global tick stamp in
  `[bootgraph.pd.spawn.ok]` (STOP FIRST — kernel edit).
- Log noise dominates volume: boot_frame.alloc (67790) + linen.session.reject
  (61269) + scheduler.yield (22965) = 96% of 158k lines. Serial spam is itself
  a perf suspect (serial writes stall PDs); logvolume metric tracks it per
  bisect step.

## Recurring issues

1. Old logs without trace markers exit 125, not 2 — intentional (pre-instrumentation ≠ regression).
2. When testing exit codes, don't pipe gate output (`gate | tail` → `$?` is tail's). Redirect instead.
3. Pre-existing end-of-run pd=7 crash flake (see INPUT_PRESENT_TICK_TRACE_V1 §Recurring) will flip a run to BAD via fault scan — rerun once before trusting a BAD verdict on a bisect step.

## Next smallest prompt

MISSION: CURSOR_DRAW_FRESHNESS_PROOF_V1 (unchanged from
INPUT_PRESENT_TICK_TRACE_V1) — prove whether coalesced cursor draws use the
freshest received position before optimizing draw cadence.
