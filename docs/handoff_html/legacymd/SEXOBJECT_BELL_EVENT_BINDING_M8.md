# SexObject Bell Event Binding M8

**Date:** 2026-05-06
**Status:** PASS
**Gate:** `SEXOS_BELL_SEXOBJECT_PROOF=1`

## Goal

Bell notification/event entries should reference global SexObject ids.
Bell event_id remains Bell-local queue identity. SexObject binding adds
global sexfiles_object_id + generation to the event record.

## Implementation Summary

### BellQueueEntry fields added

Added two fields to `BellQueueEntry` in the Bell server:

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `sexfiles_object_id` | `u64` | 0 | Global SexFiles object_id this event references |
| `sexobject_generation` | `u64` | 0 | SexObject rights_generation at event time |

Both default to 0 (no SexObject reference) for non-object-bound events.
`event_id` remains the Bell-local monotonic queue identifier.

### Files changed

| File | Change |
|------|--------|
| `servers/sexbell/src/main.rs` | Added fields to `BellQueueEntry`, all construction sites, M8 proof gate + function |
| `servers/sexfiles/src/proof.rs` | Fixed pre-existing `loaded_ok` declaration |

### Bell binding path

```
Bell NOTIFY (OP_BELL_NOTIFY, 0xC0)
  └─ BELL_QUEUE.push(...)
       └─ BellQueueEntry stored with:
            event_id = 2 (Bell-local, monotonic)
            sexfiles_object_id = 42 (global SexFiles ID)
            sexobject_generation = 1 (rights_generation)
```

Currently `sexfiles_object_id` is set post-push by the proof. In production,
callers would include it in the NOTIFY payload or set it via a dedicated opcode.

### event_id vs sexfiles_object_id

- `event_id` — Bell-local queue identity (monotonic, starts at 1). Used for
  CLOSE, ACTION, LIST operations within Bell.
- `sexfiles_object_id` — Global SexFiles identity for the object the event
  references. Authoritative across Linen, SexFiles, Collar, Mesh.

These are distinct namespaces and never confused.

## Proof Markers

```
[sexobject.m8.bell.emit]            event_id=2 object_id=42 generation=1
[sexobject.m8.bell.global_id]       local_id=99 global_id=42 event_id=2 global_used=1
[sexobject.m8.bell.local_id_reject] local_leaked=0
[sexobject.m8.bell.observable_only] authority_enforced=0 storage_mutated=0
[sexobject.m8.pass]                 ok=1
```

| Marker | Meaning | Result |
|--------|---------|--------|
| bell.emit | Entry created with global ID | event_id=2, object_id=42 |
| bell.global_id | Global ID stored, event_id separate | global_used=1 |
| bell.local_id_reject | Local ID not in sexfiles_object_id | local_leaked=0 |
| bell.observable_only | Bell doesn't mutate authority | authority_enforced=0 |
| pass | All checks passed | ok=1 |

## Build & Runtime

```sh
cargo check -p sexbell --target x86_64-sex.json       # PASS
./scripts/entrypoint_build.sh                          # PASS
./scripts/master_runtime_gate.sh                       # GREEN_MASTER
```

All spawn, clock, scheduler, fault, and sexfiles gates pass. No panics.

## No sex-pdx ABI Changes

No opcodes added to sex-pdx. No kernel edits. No disk format changes.
The Bell queue entry struct was extended with new fields; existing fields
and wire formats remain backward-compatible.

## Remaining Risks

1. `sexfiles_object_id` is set post-push in the proof. Production code would
   need a dedicated bell opcode or extended NOTIFY wire format to include the
   global ID at push time.

2. `sexobject_generation` is stored but not validated against current
   SexFiles rights_generation. Stale event detection (compare stored
   generation against current) is a future capability.

3. Bell is a separate PD (PD 10). It does not have SLOT_STORAGE or SLOT_LINEN
   capabilities, so it cannot directly resolve or verify global object IDs.

## Next Step

**SEXOBJECT_SPINDLE_SESSION_M9** — Spindle sessions/logs become durable SexObjects.
