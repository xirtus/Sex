# SEXFILES_NAMESPACE_MODEL_PHASE2_V1

## Purpose
Lock the phase-2 SexFiles namespace/capability binding evidence using the existing shell route and Collar checks, without kernel or `sex-pdx` ABI changes.

## Files Changed
- `servers/silk-shell/src/main.rs` (existing route/check wiring already present in dirty tree)
- `docs/handoff/SEXFILES_NAMESPACE_CAPS_BIND_V2.md` (prior handoff)
- `docs/handoff/SEXFILES_NAMESPACE_MODEL_PHASE2_V1.md` (this canonical phase-2 handoff)

## Proof Gate / Env
- Primary gate used: `SEXOS_COLLAR_ENFORCE_PROOF=1`
- Audit invocation alias also used in master audit sweep: `SEXOS_SEXFILES_NAMESPACE_PHASE2_PROOF=1`

## Exact Proof Markers
- `[collar.enforce.allow]`
- `[collar.enforce.deny]`
- `[collar.audit]`
- `[linen.quil.open.reject.cap]`
- `[collar.enforce.proof.sexfiles.allow]`
- `[collar.enforce.proof.sexfiles.deny]`

## Build / Runtime Result
- `./scripts/entrypoint_build.sh`: PASS
- `./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)
- `SEXOS_COLLAR_ENFORCE_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)

## Non-Goals
- No SexFiles backend redesign
- No POSIX path model
- No kernel capability rewrite
- No `sex-pdx` ABI edits

## Remaining Risks
- Route identity still surface-oriented (`FOCUSED_SURFACE_ID`) rather than per-request signed subject token.
- Deny-path proof for specific UI flows depends on runtime stimulus.
- Delegation/share semantics remain future capability work.

## Persistence / Hardware Claim Status
- Namespace/capability semantics are real runtime behavior.
- No new persistence claim is made here.
