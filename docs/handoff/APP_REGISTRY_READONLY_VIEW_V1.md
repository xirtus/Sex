# APP_REGISTRY_READONLY_VIEW_V1

Status: implemented (read-only proof markers)

## Goal
Implement Phase A read-only app registry surface in Linen using local object model primitives, with no kernel/ABI/pdx changes.

## Change Summary
- Added compile-time gate:
  - `SEXOS_APP_REGISTRY_READONLY_PROOF=1`
- Added one-shot proof pass in silk-shell main loop that emits registry rows from local `LINEN_OBJECTS`.

## Markers
- `[app.registry.row] app_id=N state=NAME kind=NAME name=NAME ok=1`
- `[app.registry.readonly.proof.done] rows=N ok=N`

## Safety/Scope
- Read-only marker emission only.
- No launch behavior rewiring.
- No kernel/ABI/sex-pdx edits.

## Proof Path
1. `SEXOS_APP_REGISTRY_READONLY_PROOF=1 ./scripts/entrypoint_build.sh`
2. `./scripts/run_daily_driver_proof.sh /tmp/sexos_app_registry_readonly.log`
3. env-boot marker grep for `app.registry.row` and `.proof.done`

## Result
- Baseline gate preserved (`18/18 PASS`, `faults=0`).
