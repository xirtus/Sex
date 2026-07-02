# APP_INSTALL_MODEL_PHASEB_PLAN_V1

## Goal
Docs-only architecture plan for a real app install/registry/launch model beyond hardcoded surfaces.

## Constraints
- No POSIX path assumptions inside SexOS internals.
- No kernel/ABI changes in this plan phase.
- No implementation in this mission.

## Model Overview
- **SexObject**: canonical object identity and typed metadata.
- **Linen**: user-facing catalog/session view over local objects.
- **Registry record**: normalized app metadata row derived from SexObject fields.

## Proposed Registry States
- `Discovered`: object recognized as app candidate.
- `Indexed`: metadata normalized and searchable.
- `Runnable`: launch intent can be formed safely.
- `Blocked`: missing capability or policy constraint.

## Launch Intent Flow (No New ABI)
1. Resolve selected Linen object to registry row.
2. Validate app kind + required capability profile.
3. Emit launch intent marker/event to existing shell dispatcher path.
4. Record success/fail marker with reason code.

## Security / Capability Checks
- Never auto-grant capabilities.
- Preserve existing Collar policy decisions.
- Require explicit deny/reject markers for blocked launch intent.

## Proof Marker Families (Future)
- `app.registry.resolve.*`
- `app.registry.intent.*`
- `app.registry.intent.reject.*`
- `app.registry.intent.done.*`

## STOP FIRST Boundaries
- Any kernel spawn contract change.
- Any new ABI verb for launch/install.
- Any persistence contract changes in `sexfiles`/storage transport.

## Suggested Implementation Phases
1. Read-only registry parity against seeded objects (done in current track).
2. Filter/sort/search parity + stable markers (in progress).
3. Launch-intent marker layer via existing dispatcher (no new ABI).
4. Optional persistence/index durability after proof lane is stable.
