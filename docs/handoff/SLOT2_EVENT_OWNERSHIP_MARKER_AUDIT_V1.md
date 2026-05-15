# SLOT2_EVENT_OWNERSHIP_MARKER_AUDIT_V1

Status: implemented (diagnostics-only markers)

## Goal
Add marker coverage for slot ownership at sexusb event-demux boundaries without changing USB behavior.

## Scope
- Marker-only additions in `servers/sexusb/src/main.rs`.
- No kernel/scheduler/ABI changes.
- No endpoint config/poll behavior changes.

## Gate
- `SEXUSB_SLOT2_OWNERSHIP_PROOF=1`

## Markers
- `[sexusb.slot.ownership.event] slot=N ep=N cc=N matched_idx=N devices=N`
- `[sexusb.slot.ownership.invariant.miss] reason=no_device_match slot=N ep=N cc=N`

## Safety
- Marker emission is budgeted and bounded.
- Existing demux logic is unchanged.

## Proof Path
1. `SEXUSB_SLOT2_OWNERSHIP_PROOF=1 ./scripts/entrypoint_build.sh`
2. `./scripts/run_daily_driver_proof.sh /tmp/sexos_slot2_ownership_audit.log`
3. env-boot grep for ownership markers.

## Acceptance
- daily-driver baseline preserved: `18/18 PASS`, `faults=0`
- ownership marker trace present for analyzed transfer events
