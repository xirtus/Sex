# ATLAS_OVERVIEW_FINAL_LONG_WINDOW_PROOF_V1

## Scope
- Mission: Prove Atlas final closeout with longer daily-driver runtime window.
- Constraint: No Atlas/Silk source behavior changes.
- Allowed change lane: runtime proof invocation only.

## Backup Before Changes
- Backup snapshot:
  - `/tmp/atlas_overview_final_long_window_backup_20260522-001600`
- Contains:
  - `scripts/run_daily_driver_proof.sh`
  - `docs/handoff/ATLAS_OVERVIEW_FINAL_CALLPATH_DIAG_V1.md`

## Script Inspection
- Existing runtime override already present in `scripts/run_daily_driver_proof.sh`:
  - `PROBE_SECONDS="${DAILY_DRIVER_PROBE_SECONDS:-30}"`
- Result: no script edit required.
- Default behavior unchanged.

## Long-Window Proof Run
Command:
```bash
DAILY_DRIVER_PROBE_SECONDS=90 ./scripts/run_daily_driver_proof.sh /tmp/atlas_overview_final_long_window_proof_v1.log
```

Gate summary:
- `FINAL: PASS (284 gates proved, 60 skipped, 0 faults)`
- `atlas_overview_final_closeout PASS`

## Atlas Marker Evidence (from log scan)
Scan command:
```bash
grep -E "silk.atlas.phase_|silk.atlas.overview.final|atlas_overview_final_closeout|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/atlas_overview_final_long_window_proof_v1.log | tail -520
```

Observed required markers:
- `[silk.atlas.phase_e4d.done] ok=1`
- `[silk.atlas.overview.final.callpath.enter] ok=1`
- `[silk.atlas.overview.final.callpath.enabled] enabled=1 ok=1`
- `[silk.atlas.overview.final.begin]`
- `[silk.atlas.overview.final.done] ok=1`

Observed gate marker:
- `atlas_overview_final_closeout PASS`

Observed final gate line:
- `FINAL: PASS (284 gates proved, 60 skipped, 0 faults)`

## Result
- Status: PASS
- Last Atlas phase reached: final closeout done.
- Fault status: 0 faults.
