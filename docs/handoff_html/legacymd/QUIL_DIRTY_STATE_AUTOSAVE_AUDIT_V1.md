# QUIL_DIRTY_STATE_AUTOSAVE_AUDIT_V1

## Goal
DIRTY flag set in undo_push, clear_dirty() on save, audit markers

## Build + Proof
- Build: PASS (9s)
- Proof: 57/57 gates PASS, 0 SKIP, 0 faults

## Safety
No kernel/ABI/USB/pointer changes. Static bounded only.
