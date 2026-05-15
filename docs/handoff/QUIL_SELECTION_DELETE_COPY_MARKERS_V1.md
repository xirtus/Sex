# QUIL_SELECTION_DELETE_COPY_MARKERS_V1

## Goal
delete_selection() with undo_push, copy_selection() to 256-byte static clipboard

## Build + Proof
- Build: PASS (9s)
- Proof: 57/57 gates PASS, 0 SKIP, 0 faults

## Safety
No kernel/ABI/USB/pointer changes. Static bounded only.
