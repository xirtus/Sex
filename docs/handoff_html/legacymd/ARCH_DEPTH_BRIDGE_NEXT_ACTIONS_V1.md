# ARCH_DEPTH_BRIDGE_NEXT_ACTIONS_V1

Status: docs-only bridge

## Purpose
Translate architecture documents into the smallest safe next implementation missions with explicit acceptance gates.

## Source Docs
- `docs/handoff/APP_INSTALL_MODEL_PLAN_V1.md`
- `docs/handoff/USB_SLOT2_MULTI_HID_ARCH_REVIEW_V1.md`

## Next Implementation Mission Candidates

### Candidate 1: APP_REGISTRY_READONLY_VIEW_V1
Goal:
- Implement Phase A read-only app registry surface in Linen using existing object model primitives.

Boundaries:
- no kernel/ABI/sex-pdx edits
- no launch behavior rewiring

Acceptance gate:
- build PASS
- `./scripts/run_daily_driver_proof.sh` remains `18/18 PASS`, `faults=0`
- emits read-only registry row markers for app identity/state

### Candidate 2: SLOT2_EVENT_OWNERSHIP_MARKER_AUDIT_V1
Goal:
- Add diagnostics-only marker coverage for slot ownership at event-demux boundaries.

Boundaries:
- no USB behavior changes
- no scheduler/kernel edits
- stop at first missing invariant marker

Acceptance gate:
- existing daily-driver gate remains green (`18/18 PASS`, `faults=0`)
- slot ownership marker trace is complete for analyzed path

## Sequencing Recommendation
1. Run `APP_REGISTRY_READONLY_VIEW_V1` first (lower risk, daily-driver lane).
2. Run `SLOT2_EVENT_OWNERSHIP_MARKER_AUDIT_V1` second (diagnostics-only, STOP-FIRST guarded).

## STOP FIRST Conditions
Escalate and split into dedicated mission before implementation if either candidate requires:
- kernel/ABI/sex-pdx change
- sexusb runtime behavior change
- shared-memory/backing-buffer contract changes

## Baseline Guardrail
Both candidates must preserve baseline:
- `18/18 PASS`
- `faults=0`
