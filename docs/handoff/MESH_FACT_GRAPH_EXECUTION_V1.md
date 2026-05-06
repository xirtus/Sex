# MESH_FACT_GRAPH_EXECUTION_V1

## Purpose
Capture and lock bounded Mesh fact-graph execution evidence in current shell implementation (fact write, render, selection, object-link scan).

## Files Changed
- `servers/silk-shell/src/main.rs` (existing Mesh markers/logic in current tree)
- `docs/handoff/MESH_FACT_GRAPH_EXECUTION_V1.md` (this handoff)

## Proof Gate / Env
- No dedicated new gate implemented in this closure.
- Audit invocation used: `SEXOS_MESH_FACT_GRAPH_PROOF=1` (runtime gate pass only).

## Exact Proof Markers
- Fact write path:
  - `[mesh.fact.write]`
  - `[mesh.fact.done]`
- Link scan path:
  - `[mesh.object_link.start]`
  - `[mesh.object_link.row]`
  - `[mesh.object_link.reject.missing_object]`
  - `[mesh.object_link.done]`
- Render/selection path:
  - `[mesh.fact_list.render]`
  - `[mesh.fact_list.row]`
  - `[mesh.selection.current]`
  - `[mesh.selection.next]`
  - `[mesh.selection.prev]`

## Build / Runtime Result
- `./scripts/entrypoint_build.sh`: PASS
- `./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)
- `SEXOS_MESH_FACT_GRAPH_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: runtime PASS (`GREEN_MASTER`)

## Non-Goals
- No graph engine redesign
- No unbounded query language
- No kernel/ABI edits

## Remaining Risks
- `SEXOS_MESH_FACT_GRAPH_PROOF` currently lacks a dedicated strict marker assertion contract in gate script.
- Mesh remains shell-local in-memory graph evidence, not persisted.

## Persistence / Hardware Claim Status
- Mesh graph execution markers are real runtime behavior.
- No persistence claim.
