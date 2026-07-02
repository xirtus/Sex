# SexObject Spindle Session M9

**Date:** 2026-05-06
**Status:** PASS
**Gate:** `SEXOS_SPINDLE_SEXOBJECT_PROOF=1`

## Goal

Bind Spindle terminal/session state to global SexObject identity.
SpindleSession is a SexObjectKind (kind=5), backed by a persisted Linen/SexFiles
object with a global object_id.

## Implementation Summary

### Spindle app infrastructure

Added to `apps/spindle/src/main.rs`:
- `SPINDLE_SESSION_SEXOBJECT_ID: u64` — global SexFiles object_id (default 0)
- `SPINDLE_SESSION_SEXOBJECT_GENERATION: u64` — rights_generation
- `SPINDLE_LOCAL_SESSION_ID: u64` — local session id (NOT authority-bearing)
- `run_spindle_sexobject_proof()` — synthetic proof using global ID binding

Also fixed 8 pre-existing `dispatch()` calls missing the `hist` argument.

### Linen M9 proof

Added to `servers/linen/src/main.rs`:
- `run_m9_spindle_session_proof()` — canonical proof demonstrating:
  1. Create a Session-kind Linen object (maps to SpindleSession SexObjectKind)
  2. Persist to SexFiles, obtain global sexfiles_object_id
  3. Bind global ID to session
  4. Prove local session_id ≠ global object_id
  5. Prove SexObjectRef uses global ID

Since Spindle is a user-space app (not kernel-spawned, no PDX slots), the
canonical proof runs in Linen where persistence and cross-PD paths exist.

### Files changed

| File | Change |
|------|--------|
| `servers/linen/src/main.rs` | Added M9 gate constant, `run_m9_spindle_session_proof()` |
| `apps/spindle/src/main.rs` | Added session SexObject state, M9 proof function, fixed dispatch calls |

## Proof Markers (Linen — canonical)

```
[sexobject.m9.spindle.session.create]     session_id=1 accepted=1
[sexobject.m9.spindle.sexfiles_object_id] session_id=1 object_id=2 global_ok=1
[sexobject.m9.spindle.local_id_separate]  session_id=1 global_id=2 separate=1
[sexobject.m9.spindle.ref_global]         ref_object_id=2 global_in_ref=1
[sexobject.m9.spindle.local_id_reject]    local_leaked=0
[sexobject.m9.pass]                       ok=1
```

| Marker | Meaning | Result |
|--------|---------|--------|
| session.create | Session object created | accepted=1 |
| sexfiles_object_id | Global ID bound | object_id=2, global_ok=1 |
| local_id_separate | Local ≠ global | separate=1 |
| ref_global | SexObjectRef uses global | global_in_ref=1 |
| local_id_reject | Local not in authority ref | local_leaked=0 |
| pass | All checks passed | ok=1 |

## Session Identity Model

| Field | Scope | Authority |
|-------|-------|-----------|
| `local_session_id` (Spindle-internal) | Spindle app only | None |
| `sexfiles_object_id` (global) | Cross-system | SexFiles |
| `sexobject_generation` | SexObject ref validation | SexFiles (bumped by Collar M6) |

## Build & Runtime

```sh
cargo check -p linen --target x86_64-sex.json       # PASS
./scripts/entrypoint_build.sh                         # PASS
./scripts/master_runtime_gate.sh                      # GREEN_MASTER
```

## No sex-pdx ABI Changes

No new opcodes in sex-pdx. No kernel edits. No disk format changes.

## Remaining Risks

1. Spindle is not kernel-spawned, so `run_spindle_sexobject_proof` never executes.
   The canonical proof lives in Linen. When Spindle gets kernel-spawned, the
   app-level code is in place for binding.

2. Session persistence requires Spindle (or its parent) to have SLOT_LINEN
   capability. This is a kernel cap grant that doesn't exist yet.

3. Only one session per Spindle instance is modeled. Multi-session support
   would require a session table in Spindle.

## Next Step

**SEXOBJECT_QUIL_DOCUMENT_M10** — Quil documents become durable typed SexObjects.
