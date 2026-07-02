# COLLAR_ENFORCE_TWO_OPS_V1

## Operations Enforced
Two existing operations are now actively enforced via `collar_check_operation(...)` at route points:

1. `AccessBell`
- Route: Bell open/toggle path in shell (`toggle_bell()` and SilkBar Bell click path).
- Gate target: active subject surface ID (`FOCUSED_SURFACE_ID`).
- Default deny for unknown/non-app surface IDs.
- Grant match required for allow.

2. `AccessSexFiles`
- Route: Linen object open path (`open_linen_object_in_quil(...)`).
- Gate target: active subject surface ID (`FOCUSED_SURFACE_ID`).
- Must pass `AccessSexFiles` before existing `LinkObjectToBuffer` grant check.

## Deny-by-Default Rules Added
For `AccessBell` and `AccessSexFiles`:
- unknown app surface (`caller_sid < 300`) denied
- missing cap (no matching active grant) denied

For dangerous authority:
- `AccessDisplay` always denied
- `AccessShellPolicy` always denied

No kernel changes, no `sex-pdx` ABI changes, no app-controlled policy.

## Proof Gate
- `SEXOS_COLLAR_ENFORCE_PROOF=1`

## Proof Markers
- `[collar.enforce.allow]`
- `[collar.enforce.deny]`
- `[collar.audit]`

Additional stage markers:
- `[collar.enforce.proof.bell.allow]`
- `[collar.enforce.proof.bell.deny]`
- `[collar.enforce.proof.sexfiles.allow]`
- `[collar.enforce.proof.sexfiles.deny]`
- `[collar.enforce.proof.dangerous.deny]`
- `[collar.enforce.proof.unknown.deny]`

## Remaining Collar Risks
- Subject identity is currently tied to `FOCUSED_SURFACE_ID` in shell-local dispatch, not a dedicated per-request subject token.
- Grant scoping is simple `(subject_id, object_id, operation_mask)` and not yet context/time constrained.
- Policy remains shell-local; no separate Collar PD authority runtime yet.
