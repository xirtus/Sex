# SexObject Quil Document M10

**Date:** 2026-05-06
**Status:** PASS
**Gate:** `SEXOS_QUIL_SEXOBJECT_PROOF=1`

## Goal

Bind Quil documents to global SexObject identity. QuilDocument is SexObjectKind=4.
Quil local document id stays separate from global SexFiles object_id.

## Implementation Summary

### Quil server changes

Added document SexObject state:
- `QUIL_DOC_SEXOBJECT_ID: u64` — global SexFiles object_id (default 0)
- `QUIL_DOC_SEXOBJECT_GENERATION: u64` — rights_generation
- `QUIL_LOCAL_DOCUMENT_ID: u64 = 999` — local document id (NOT authority)

Added `OP_RAMFS_OBJECT_ID = 0x37` constant (was missing).

Added proof gate `SEXOS_QUIL_SEXOBJECT_PROOF` with `run_quil_sexobject_proof()`:
1. Saves buffer via `quil_save()` → creates RamFS file
2. Reopens file via `OP_RAMFS_OPEN`
3. Obtains global object_id via `OP_RAMFS_OBJECT_ID`
4. Binds to `QUIL_DOC_SEXOBJECT_ID`
5. Proves local `document_id=999` ≠ global `object_id=1`

### Spindle app fixes

Fixed pre-existing issues with `dispatch()` calls in the input proof
(now takes 4 args: line, sb, hist, ev). Added `EventRing` variable.

### Files changed

| File | Change |
|------|--------|
| `servers/quil/src/main.rs` | Added `OP_RAMFS_OBJECT_ID`, document state, M10 proof gate + function, `pack_name_quil` helper |
| `apps/spindle/src/main.rs` | Fixed dispatch calls, added `EventRing::new()`, updated proof signature |

## Proof Markers

```
[sexobject.m10.quil.sexfiles_object_id] document_id=999 object_id=1 global_ok=1
[sexobject.m10.quil.document.create]    document_id=999 accepted=1
[sexobject.m10.quil.local_id_separate]  document_id=999 global_id=1 separate=1
[sexobject.m10.quil.ref_global]         ref_object_id=1 global_in_ref=1
[sexobject.m10.quil.local_id_reject]    local_leaked=0
[sexobject.m10.pass]                    ok=1
```

| Marker | Meaning | Result |
|--------|---------|--------|
| sexfiles_object_id | Global ID bound via OP_RAMFS_OBJECT_ID | object_id=1, global_ok=1 |
| document.create | Document binding established | accepted=1 |
| local_id_separate | Local 999 ≠ global 1 | separate=1 |
| ref_global | SexObjectRef uses global ID | global_in_ref=1 |
| local_id_reject | Local not in authority ref | local_leaked=0 |
| pass | All checks passed | ok=1 |

## Persistence Path

```
Quil (PD 9)
  └─ quil_save() → OP_RAMFS_OPEN(O_CREATE) + OP_RAMFS_WRITE + OP_RAMFS_CLOSE
       └─ Reopen: OP_RAMFS_OPEN → handle
            └─ OP_RAMFS_OBJECT_ID → global object_id
                 └─ QUIL_DOC_SEXOBJECT_ID = global object_id
```

Quil already has `SLOT_STORAGE` capability (kernel grant). The existing
`quil_save()` / `quil_load()` path uses RamFS persistence for text content.
M10 adds the global object identity binding on top.

## Build & Runtime

```sh
cargo check -p quil --target x86_64-sex.json         # PASS
cargo check -p spindle --target x86_64-sex.json       # PASS
./scripts/entrypoint_build.sh                          # PASS
./scripts/master_runtime_gate.sh                       # GREEN_MASTER
```

## No sex-pdx ABI Changes

No new opcodes in sex-pdx. `OP_RAMFS_OBJECT_ID` already existed.
No kernel edits. No disk format changes.

## Next Step

**SEXOBJECT_FINAL_AUDIT_M11** — Cross-system audit verifying all bindings
are consistent: OQ5 global ids, Collar revocation, Mesh facts, Bell events,
Spindle sessions, Quil documents.
