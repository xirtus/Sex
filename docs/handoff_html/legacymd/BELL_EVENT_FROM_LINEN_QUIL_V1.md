# BELL_EVENT_FROM_LINEN_QUIL_V1 — Handoff

## Goal
Emit Bell notification events for Linen object workflow milestones and Quil
text save/edit milestones using the existing fire-and-forget Bell notify path.
No notification model redesign.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Bell workflow event proof gate, proof function, wiring in main loop | +36 |

## Architecture
- **Gate**: `BELL_WORKFLOW_EVENT_PROOF_ENABLED` via `SEXOS_BELL_WORKFLOW_EVENT_PROOF=1`
- **Proof function**: `maybe_run_bell_workflow_event_proof()` — one-shot in main loop
- Uses existing `bell_send_app_event()` helper (fire-and-forget via `pdx_call(SLOT_BELL, OP_BELL_NOTIFY, ...)`)

## Events Emitted
| Event ID | Source | Milestone |
|----------|--------|-----------|
| 2001 | linen_workflow | Object create/tag/search workflow complete |
| 2002 | linen_workflow | Object persist async attempt |
| 2003 | quil_workflow | Text edit buffer proof complete |
| 2004 | quil_workflow | Text save async attempt |

## Markers (serial)
```
[bell.workflow.event] source=NAME event_id=N ok=N reason=...
[bell.workflow.event.list] total=N ok=N
[bell.workflow.event.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_BELL_WORKFLOW_EVENT_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `bell_workflow_events`: PASS (4 workflow events)

## Safety / STOP FIRST
- ❌ No notification redesign — uses existing OP_BELL_NOTIFY wire format
- ❌ No blocking waits — fire-and-forget only
- ❌ No new PDX opcodes — uses existing SLOT_BELL (12) + OP_BELL_NOTIFY (0xC0)
- ✅ Bell server independently validates/enqueues; silk-shell is just a sender
- ✅ Event IDs 2001-2004 reserved for workflow milestones (non-overlapping with V2 1001-1004)
- ✅ Zero impact on other proofs

## Known Limitations
- Events are sent but not verified received (no read-back from Bell queue)
- Only 4 synthetic events, hardcoded
- Not wired to real Linen/Quil lifecycle hooks (proof-only)
- No event payload beyond source name + event_id

## Future Follow-up
- Wire to real Linen object create/persist lifecycle hooks
- Wire to real Quil buffer save/edit lifecycle hooks
- Subscribe to Bell generation counter for delivery confirmation
- Per-app event payload (object_id, buffer_len, timestamp)
