# SexObject Mesh Fact View M7

**Date:** 2026-05-06
**Status:** PASS
**Gate:** `SEXOS_SEXOBJECT_MESH_FACT_PROOF=1` (Linen side), `SEXOS_MESH_FACT_PROOF=1` (silk-shell side)

## Goal

Mesh must record/view SexObject relationships using global SexFiles object_id,
not Linen-local id. Mesh is explanatory/observable only — not authoritative.

## Implementation Summary

### LinenObject.sexfiles_object_id (silk-shell)

Added `sexfiles_object_id: u64` to silk-shell's `LinenObject` struct (default 0).
All 6 seed objects initialize with `sexfiles_object_id: 0`.

### OP_LINEN_GET_GLOBAL_ID (0x45)

New Linen opcode for silk-shell to query the global sexfiles_object_id for
a Linen-local object_id. Returns:
- global ID (≥1) on success
- 0 if object exists but is not yet persisted
- Error (negative) if object not found

### Mesh fact recording with global ID

Updated `mesh_emit_linen_quil_links()` to prefer `o.sexfiles_object_id` for
MeshFact.subject_id when available (non-zero). Falls back to local
`o.object_id` for seed objects that haven't been persisted yet.

### Linen M7 proof

`run_m7_mesh_fact_proof()` demonstrates:
1. Object creation + persist → global sexfiles_object_id assigned
2. Mesh-style fact record with global ID as subject_id/object_id
3. Linen-local ID confined to ref_id (debug context only)
4. Mesh observable-only invariant

### Silk-shell M7 proof

2-stage boot proof:
- Stage 0: Record Mesh fact with best available ID, report global vs local
- Stage 1: Confirm Mesh does not mutate authority or storage

## Files Changed

| File | Change |
|------|--------|
| `servers/linen/src/main.rs` | Added `OP_LINEN_GET_GLOBAL_ID`, handler, M7 proof gate + function |
| `servers/silk-shell/src/main.rs` | Added `sexfiles_object_id` to `LinenObject`, `OP_LINEN_GET_GLOBAL_ID`, updated `mesh_emit_linen_quil_links`, M7 proof gate + function |

## Proof Markers (Linen side — canonical)

```
[sexobject.m7.mesh.fact.write] subject_id=4 object_id=4 ref_id=3 sequence=1
[sexobject.m7.mesh.global_id] local_id=3 global_id=4 global_used=1
[sexobject.m7.mesh.local_id_reject] local_leaked=0
[sexobject.m7.mesh.observable_only] authority_enforced=0 storage_mutated=0
[sexobject.m7.pass] ok=1
```

| Marker | Meaning | Result |
|--------|---------|--------|
| mesh.fact.write | Fact recorded with global IDs | subject_id=object_id=4 (global), ref_id=3 (local) |
| mesh.global_id | Global SexFiles ID used | global_used=1 |
| mesh.local_id_reject | Local ID not in authority fields | local_leaked=0 |
| mesh.observable_only | Mesh is read-only | authority_enforced=0 |
| pass | All checks passed | ok=1 |

## ID Semantics

| Field | Meaning | M7 Value |
|-------|---------|----------|
| subject_id | Authoritative SexFiles object_id | global (≥1 when persisted) |
| object_id | Secondary object identity | global (≥1 when persisted) |
| ref_id | Debug/context reference | Linen-local (safe, non-authoritative) |

## Mesh is NOT Authority

- No Collar state mutated
- No rights_generation bumped
- No SexFiles object modified
- Mesh fact ring is append-only, read-only observation
- Collar remains authority policy layer
- SexFiles remains storage/generation authority

## Build & Runtime

```sh
cargo check -p linen --target x86_64-sex.json         # PASS
cargo check -p silk-shell --target x86_64-sex.json    # PASS
./scripts/entrypoint_build.sh                          # PASS
./scripts/master_runtime_gate.sh                       # GREEN_MASTER
```

All spawn, clock, scheduler, fault, and sexfiles gates pass.

## No sex-pdx ABI Changes

New opcodes (0x45) are server-local numeric constants. No kernel edits.
No disk format changes. No sex-pdx ABI edits.

## Remaining Risks

1. `mesh_emit_linen_quil_links` uses local ID fallback for seed objects.
   Once seed objects are also persisted, all facts will use global IDs.

2. Silk-shell's `LinenObject.sexfiles_object_id` is initialized to 0 for seeds.
   A future boot-phase synchronization (Linen pushes global IDs back to
   silk-shell after persist) would close this gap.

## Next Step

**SEXOBJECT_BELL_EVENT_BINDING_M8** — Bell events reference global SexObject ids.
