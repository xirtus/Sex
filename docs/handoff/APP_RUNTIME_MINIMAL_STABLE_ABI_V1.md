# APP_RUNTIME_MINIMAL_STABLE_ABI_V1

## Scope
Lock the smallest stable app runtime ABI around existing `OP_APP_SURFACE_REQ` + `AppManifest` behavior, without kernel/sex-pdx ABI changes.

## Contract (Locked)
- Opcode: `OP_APP_SURFACE_REQ = 0xFA`
- ABI version tag: `APP_RUNTIME_ABI_VERSION = 1` (contract tag)
- Manifest packing (`arg2`):
  - bits `0..7` capability bits (known-only)
  - bits `8..23` app_id
  - bits `24..55` reserved (must be zero)
  - bits `56..63` version (must equal V1 wire value `0`)
- Deterministic reject classes:
  - manifest invalid (version/reserved/unknown cap)
  - zero surface id
  - zero title id
  - reserved range (`surface_id < 200`)
  - duplicate surface id
  - no frame slot

## Proof Gate
- `SEXOS_APP_RUNTIME_ABI_PROOF=1`

## Proof Markers
- `[app.abi.proof]`
- `[app.abi.proof.accept.v1]`
- `[app.abi.proof.roundtrip]`
- `[app.abi.proof.reject.reserved]`
- `[app.abi.proof.reject.unknown_cap]`
- `[app.abi.proof.reject.version]`
- `[app.abi.proof.reject.sid_range]`

## Non-Goals
- No kernel ABI changes
- No sex-pdx ABI changes
- No capability system redesign
- No broad app runtime refactor
