# SPINDLE_APPS_REGISTRY_VIEW_V1

## Scope
- `apps/spindle` only.
- Read-only marker surface for `apps` command proof mode.

## Files
- apps/spindle/src/main.rs

## Change
- Under `SEXOS_SPINDLE_APPS_REGISTRY_PROOF`, emit compatibility marker rows in the expected contract:
  - `[spindle.apps.registry.row] idx=N app=NAME key=NAME status=NAME`
- Preserve existing detailed rows:
  - `[spindle.apps.registry.row] app_id=... state=... kind=... name=... ok=1`
- Keep summary marker:
  - `[spindle.apps.registry.done] rows=6 ok=1`

## Proof
- Build gate: PASS
- Daily-driver gate: PASS target remains 18/18 with faults=0

## Notes
- No command semantics changed.
