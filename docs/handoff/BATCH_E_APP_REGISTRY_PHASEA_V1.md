# BATCH_E_APP_REGISTRY_PHASEA_V1

Status: implemented

## Scope
- Expand read-only app registry marker surface
- Add registry filter/sort proof markers in shell
- Align spindle `apps` with registry markers

## New/Used Proof Gates
- `SEXOS_APP_REGISTRY_READONLY_PROOF=1`
- `SEXOS_APP_REGISTRY_FILTER_SORT_PROOF=1`
- `SEXOS_SPINDLE_APPS_REGISTRY_PROOF=1`

## Markers
- Shell:
  - `[app.registry.row] ...`
  - `[app.registry.readonly.proof.done] ...`
  - `[app.registry.filter] ...`
  - `[app.registry.sort] ...`
- Spindle:
  - `[spindle.apps.registry.row] ...`
  - `[spindle.apps.registry.done] ...`

## Safety
- No kernel/ABI/sex-pdx edits
- No launch behavior rewiring
- Marker-only visibility improvements

## Gate
- baseline preserved at `18/18 PASS`, `faults=0`
