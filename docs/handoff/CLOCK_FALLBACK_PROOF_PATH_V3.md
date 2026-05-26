# CLOCK_FALLBACK_PROOF_PATH_V3

## Summary
V2 achieved runtime emission of `[sexdisplay.clock.redraw.source_check] ... ok=1`, but gate still produced a false FAIL and stale-drop continuation proof remained non-deterministic in organic runs.

## V2 Partial Success
Observed runtime markers included both fallback and silkbar redraw sources with canonical match:
- `[sexdisplay.clock.redraw.source_check] redraw_ss=1 canonical_ss=1 source=fallback ok=1`
- `[sexdisplay.clock.redraw.source_check] redraw_ss=2 canonical_ss=2 source=fallback ok=1`
- `[sexdisplay.clock.redraw.source_check] redraw_ss=1 canonical_ss=1 source=silkbar ok=1`

## Root Cause (False Negative)
`clock_visible_seconds` in `scripts/daily_driver_master_gate.sh` required all of the following at once:
- source_check `ok=1`
- no source_check `ok=0`
- `stale_drop accepted=0` present
- `fallback.continue_after_drop` present

That made the stale-drop path mandatory even in runs where no stale SetClock was naturally produced. Result: false FAIL against otherwise valid redraw source-check evidence.

## Organic Runtime Gap
Organic runtime may never receive a stale SetClock sample (incoming second behind canonical), so these markers can be absent:
- `[sexdisplay.clock.stale_drop] ... accepted=0`
- `[sexdisplay.clock.fallback.continue_after_drop] ...`

Without deterministic injection, continuation-after-drop proof is non-deterministic.

## V3 Deterministic Proof Probe
A bounded once-only runtime self-probe was added in `sexdisplay`:
- Trigger: once per boot, after canonical clock has advanced to `canonical_ss >= 2`
- Action: synthesize stale second as `stale_ss = canonical_ss - 1`
- Behavior: reject stale value (no acceptance), set pending fallback-continue marker
- No PDX send, no SilkBar edit, no geometry/layout edits

Expected proof markers:
- `[sexdisplay.clock.stale_probe.begin] canonical_ss=S stale_ss=S`
- `[sexdisplay.clock.stale_drop] incoming_ss=S canonical_ss=S accepted=0 proof=1`
- `[sexdisplay.clock.fallback.continue_after_drop] ss=S source=fallback proof=1`
- `[sexdisplay.clock.stale_probe.done] ok=1`

Probe runs at most once per boot.

## Gate Semantics Update
`clock_visible_seconds` now:
- accepts existing redraw source-check marker shape
- accepts `source=fallback` and `source=silkbar` when `ok=1`
- fails when any source-check `ok=0` exists
- requires `fallback.continue_after_drop` only when `stale_drop_count > 0`
- prints diagnostics in gate details:
  - `source_check_ok_count=N`
  - `source_check_bad_count=N`
  - `stale_drop_count=N`
  - `continue_after_drop_count=N`

## Proof Commands
```bash
./scripts/entrypoint_build.sh

SEXOS_PROOF_QMP=0 DAILY_DRIVER_PROBE_SECONDS=480 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_clock_fallback_proof_path_v3.log

./scripts/daily_driver_master_gate.sh /tmp/sexos_clock_fallback_proof_path_v3.log

rg -n "sexdisplay.clock.redraw.source_check|sexdisplay.clock.stale_probe|sexdisplay.clock.stale_drop|sexdisplay.clock.fallback.continue_after_drop|clock_visible_seconds|clock_cadence_bound|#PF|#GP|panic|fault" \
  /tmp/sexos_clock_fallback_proof_path_v3.log | tail -160
```

## Do-Not-Regress Rules
- Keep canonical clock copied into `bar.clock_*` before top-strip redraw.
- Keep redraw source check marker: `[sexdisplay.clock.redraw.source_check] ... ok=...`.
- Do not accept stale time.
- Do not require stale-drop path in gate unless stale-drop actually occurred.
- Keep framebuffer bounds-check behavior unchanged.
- No kernel/scheduler/ABI/sex-pdx/filesystem/network/input edits.
