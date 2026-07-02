# ATLAS_OVERVIEW_FINAL_CALLPATH_DIAG_V1

## Scope
- Mission: Diagnose Atlas final closeout sentinel non-emission.
- Files touched:
  - `servers/silk-shell/src/main.rs`
  - `docs/handoff/ATLAS_OVERVIEW_FINAL_CALLPATH_DIAG_V1.md`
- Backup created before edits:
  - `/tmp/microkernel-backup-atlas-callpath-20260522-000734.patch`

## Source Findings

1. Function exists and includes required sentinels:
- `servers/silk-shell/src/main.rs:16931` defines `maybe_run_atlas_overview_final_closeout_proof()`.
- Emits:
  - `[silk.atlas.overview.final.begin]`
  - `[silk.atlas.overview.final.done] ok=1`

2. Compile-time gate:
- `ATLAS_OVERVIEW_FINAL_CLOSEOUT_PROOF_ENABLED` is computed from:
  - `option_env!("SEXOS_ATLAS_OVERVIEW_FINAL_CLOSEOUT_PROOF").is_some()`
  - at `servers/silk-shell/src/main.rs:479-480`.

3. Call site exists at the correct sequencing point:
- Main proof loop calls:
  - `maybe_run_atlas_phase_e4d_real_pointer_drop_proof();`
  - then `maybe_run_atlas_overview_final_closeout_proof();`
- Location: `servers/silk-shell/src/main.rs:21680-21681`.

4. Closeout internal runtime condition:
- Requires all subphase DONE flags A through E4d.
- If any flag is false, closeout returns without sentinel emission.

## Diagnostics Added
Added one-time markers in `maybe_run_atlas_overview_final_closeout_proof()`:
- `[silk.atlas.overview.final.callpath.enter] ok=1`
- `[silk.atlas.overview.final.callpath.enabled] enabled={0|1} ok={0|1}`

These markers are emitted once per boot before early returns, to distinguish:
- not-called callpath vs
- called-but-disabled/incomplete.

## Proof Run
Command:
- `./scripts/run_daily_driver_proof.sh /tmp/atlas_overview_final_callpath_diag_v1.log`

Observed gate summary:
- `FINAL: PASS (276 gates proved, 68 skipped, 0 faults)`
- `atlas_overview_final_closeout: SKIP`

Observed raw log progression reaches:
- `[silk.atlas.phase_c.begin]`

No `silk.atlas.overview.final*` markers were present in this specific 30s captured boot log, consistent with closeout not firing in-window.

## Interpretation
- Not a missing call-site: call is present and correctly ordered after E4d in source.
- Closeout can SKIP when A..E4d aggregate completion is not reached during the captured proof window.
- Diagnostic markers are now in place to prove callpath entry + compile-time enabled state on subsequent run logs.

## Status Update (ATLAS_OVERVIEW_FINAL_LONG_WINDOW_PROOF_V1)
- Long-window proof command:
  - `DAILY_DRIVER_PROBE_SECONDS=90 ./scripts/run_daily_driver_proof.sh /tmp/atlas_overview_final_long_window_proof_v1.log`
- Result:
  - `atlas_overview_final_closeout PASS`
  - `FINAL: PASS (284 gates proved, 60 skipped, 0 faults)`
- Confirmed markers now present:
  - `[silk.atlas.overview.final.callpath.enter] ok=1`
  - `[silk.atlas.overview.final.callpath.enabled] enabled=1 ok=1`
  - `[silk.atlas.overview.final.begin]`
  - `[silk.atlas.overview.final.done] ok=1`
- Conclusion:
  - Earlier SKIP was window/truncation-related, not a missing callpath.
