# APP_REGISTRY_LAUNCH_INTENT_MARKERS_V1

## Scope
- Shell marker-only proof for app-registry launch intent readiness.
- No kernel/ABI/dispatch behavior changes.

## Files
- servers/silk-shell/src/main.rs

## Added Proof Gate
- `SEXOS_APP_REGISTRY_LAUNCH_INTENT_PROOF=1`

## Marker Contract
- `[app.registry.intent] app_id=N kind=NAME status=NAME ok=N`
- `[app.registry.intent.reject] app_id=N reason=NAME ok=N` (only for blocked rows)
- `[app.registry.intent.done] rows=N runnable=N ok=N`

## Behavior Notes
- Uses existing local seeded `LINEN_OBJECTS` only.
- Classifies known seeded object kinds as `runnable` in this marker phase.
- No launch side effects are performed.

## Proof
- Build gate: PASS
- Daily-driver gate: PASS target remains 18/18 with faults=0
