# QUIL_BUFFER_PROTOCOL_LOCK_V1

## Purpose
Lock the existing bounded Quil buffer object protocol used by shell/Linen integration, without adding new ABI surface.

## Files Changed
- `servers/silk-shell/src/main.rs` (existing markers and flow in current tree)
- `servers/quil/src/main.rs` (runtime proof-related path already present)
- `docs/handoff/QUIL_BUFFER_PROTOCOL_LOCK_V1.md` (this handoff)

## Proof Gate / Env
- No dedicated new gate introduced in this closure.
- Audit invocation used: `SEXOS_QUIL_BUFFER_PROTOCOL_PROOF=1` (runtime gate pass only).

## Exact Proof Markers
- `[quil.buffer_table.init]`
- `[quil.buffer.seed]`
- `[quil.buffer_table.ready]`
- `[quil.buffer_list.row]`
- `[linen.quil.open.request]`
- `[linen.quil.buffer.linked]`
- `[linen.quil.done]`
- Reject paths:
  - `[linen.quil.open.reject.missing]`
  - `[linen.quil.open.reject.full]`
  - `[linen.quil.open.reject.buffer_id_collision]`

## Build / Runtime Result
- `./scripts/entrypoint_build.sh`: PASS
- `./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)
- `SEXOS_QUIL_BUFFER_PROTOCOL_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: runtime PASS (`GREEN_MASTER`)

## Non-Goals
- No new Quil PDX ABI
- No editor/storage redesign
- No POSIX file semantics

## Remaining Risks
- The `SEXOS_QUIL_BUFFER_PROTOCOL_PROOF` env currently acts as audit invocation context; dedicated marker contract enforcement is still an evidence gap.
- Multi-client contention semantics for buffer ownership remain limited.

## Persistence / Hardware Claim Status
- Protocol behavior is runtime-real in memory.
- No persistence durability claim.
