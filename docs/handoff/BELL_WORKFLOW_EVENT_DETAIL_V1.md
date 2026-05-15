# BELL_WORKFLOW_EVENT_DETAIL_V1 — Handoff

## Goal
Emit detail view markers for each workflow event created from app/Linen/Quil
milestones.  Uses existing Bell fire-and-forget notify path.  No notification
model redesign.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Detail proof gate, function, wiring in main loop | +24 |

## Architecture
- **Gate**: `BELL_WORKFLOW_DETAIL_PROOF_ENABLED` via `SEXOS_BELL_WORKFLOW_DETAIL_PROOF=1`
- **Proof function**: `maybe_run_bell_workflow_detail_proof()` — one-shot in main loop
- Runs after `maybe_run_bell_workflow_event_proof()` to detail the same events

## Detail Markers
| Event ID | Source         | Detail Reason |
|----------|----------------|---------------|
| 2001     | linen_workflow | Object create/tag/search workflow proof (V2) |
| 2002     | linen_workflow | Object persist async audit (V3 fire-and-forget) |
| 2003     | quil_workflow  | Text edit buffer proof (V2 HID stash/replay) |
| 2004     | quil_workflow  | Text save async audit (V3 fire-and-forget) |

## Markers (serial)
```
[bell.workflow.detail] event_id=N source=NAME ok=N reason=...
[bell.workflow.detail.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_BELL_WORKFLOW_DETAIL_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `bell_workflow_detail`: PASS (4 detail markers)

## Safety / STOP FIRST
- ❌ No notification redesign — uses existing Bell event infrastructure
- ❌ No blocking waits — markers are local `serial_println!` only
- ❌ No new PDX opcodes
- ✅ Event IDs 2001-2004 match those from `BELL_WORKFLOW_EVENT_PROOF`
- ✅ Zero impact on other proofs

## Known Limitations
- Detail markers are synthetic — not backed by real Bell queue inspection
- No structured event payload (timestamp, object_id, severity)
- Not wired to user-facing detail view (Bell detail panel uses different event source)
- Detail proof runs unconditionally after workflow event proof

## Future Follow-up
- Read back event details from Bell server queue via PDX opcode
- Structured event payload with timestamp + source metadata
- Wire to Bell detail panel keyboard navigation
- Auto-generate detail from Bell generation counter + source filter
