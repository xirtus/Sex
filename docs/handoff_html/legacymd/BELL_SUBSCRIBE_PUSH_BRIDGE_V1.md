# BELL_SUBSCRIBE_PUSH_BRIDGE_V1

## Purpose
Document the current Bell subscribe/poll bridge used to deliver Bell presence into SilkBar state using existing opcodes.

## Files Changed
- `servers/sexbell/src/main.rs`
- `servers/silkbar/src/main.rs`
- `docs/handoff/BELL_DELIVERY_CHAIN_V1.md` (existing)
- `docs/handoff/BELL_SUBSCRIBE_PUSH_BRIDGE_V1.md` (this handoff)

## Proof Gate / Env
- Primary Bell delivery gate: `SEXOS_BELL_DELIVERY_PROOF=1`
- Audit invocation alias used: `SEXOS_BELL_PUSH_BRIDGE_PROOF=1`

## Exact Proof Markers
- `[bell.event.accept]`
- `[bell.event.reject]`
- `[bell.poll.ok]`
- `[silkbar.bell.state]`

## Build / Runtime Result
- `./scripts/entrypoint_build.sh`: PASS
- `./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)
- `SEXOS_BELL_PUSH_BRIDGE_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: runtime PASS (`GREEN_MASTER`)

## Non-Goals
- No popup policy
- No renderer policy changes
- No kernel/`sex-pdx` ABI changes
- No persistent Bell store

## Remaining Risks
- Delivery remains generation-poll based, not interrupt push.
- Alias gate name (`SEXOS_BELL_PUSH_BRIDGE_PROOF`) is audit-context only unless strict marker assertions are added.

## Persistence / Hardware Claim Status
- Bridge behavior is runtime-real in-memory.
- No persistence claim.
