# SEXFILES_NAMESPACE_CAPS_BIND_V2

## Scope
Shell-side capability binding only, while `servers/sexfiles/` remains owned by another agent.

Write scope used:
- `docs/handoff/SEXFILES_NAMESPACE_CAPS_BIND_V2.md`

No edits made to:
- `kernel/`
- `crates/sex-pdx/`
- `servers/sexfiles/`

## Route Bound (Existing Path, Confirmed)
Bound route: Linen object open into Quil path in shell.

- Function: `open_linen_object_in_quil(object_id)`
- File: `servers/silk-shell/src/main.rs`
- Gate order (current):
  1. object existence check
  2. `AccessSexFiles` capability check:
     - `collar_check_operation(CollarOperation::AccessSexFiles, FOCUSED_SURFACE_ID, 0)`
  3. object-link grant check:
     - `collar_check_operation(CollarOperation::LinkObjectToBuffer, object_id, 0)`

On deny of step 2, route rejects with:
- `[linen.quil.open.reject.cap] op=AccessSexFiles decision={...}`

## Namespace/Capability Contract Status
- Namespace capability enforcement in storage backend is provided by SexFiles work (`SEXFILES_NAMESPACE_CAPS_V1`) and remains in effect.
- Shell side now consistently binds object-open path to `AccessSexFiles` capability before link/open side effects.
- No POSIX path semantics were added.

## Proof / Markers Used
Existing markers used as proof surface:
- `[collar.enforce.allow]`
- `[collar.enforce.deny]`
- `[collar.audit]`
- `[collar.enforce.proof.sexfiles.allow]`
- `[collar.enforce.proof.sexfiles.deny]`
- `[linen.quil.open.reject.cap]` (live route marker when denied)

Proof gate:
- `SEXOS_COLLAR_ENFORCE_PROOF=1`

## Build / Runtime Validation
- `cargo check` target build for `silk-shell`: pass
- `./scripts/entrypoint_build.sh`: pass
- `SEXOS_COLLAR_ENFORCE_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: pass (`GREEN_MASTER`)

## Remaining Risks
1. Subject identity is shell-surface based (`FOCUSED_SURFACE_ID`), not a dedicated per-request subject token.
2. Route-level proof for `[linen.quil.open.reject.cap]` requires explicit deny stimulus during runtime interaction; synthetic Collar proof already covers allow/deny semantics.
3. Delegation/share semantics across PDs are still future work in SexFiles capability model.
