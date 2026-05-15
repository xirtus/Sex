# BELL_APP_EVENT_INTEGRATION_V1 — Handoff

## Goal
Have silk-shell emit Bell notification events for app lifecycle actions
(launcher open, Linen workflow done, Quil text proof done, Atlas theme applied)
via fire-and-forget PDX calls to sexbell.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Import SLOT_BELL/OP_BELL_NOTIFY; `bell_send_app_event()` helper; 5-stage proof | +61 |

## Architecture
- **`bell_send_app_event(source, event_id)`**: packs category=Info, urgency=Normal
  into Bell NOTIFY arg0; sends via `pdx_call(SLOT_BELL, OP_BELL_NOTIFY, ...)`
- **Fire-and-forget**: uses AsyncEnqueue edge — returns immediately, no reply wait
- **Event IDs**: 1001 (launcher), 1002 (linen_workflow), 1003 (quil_text), 1004 (atlas_theme)

## Proof Stages (5 stages, callable from main loop)
0. `bell_send_app_event("launcher", 1001)`
1. `bell_send_app_event("linen_workflow", 1002)`
2. `bell_send_app_event("quil_text", 1003)`
3. `bell_send_app_event("atlas_theme", 1004)`
4. Emit `[bell.app.event.list] total=4 ok=1` (sent count, not read back)
5. Done

## Markers (serial)
```
[bell.app.event] source=NAME event_id=N ok=N reason=...
[bell.app.event.list] total=N ok=N
[bell.app.integration.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_BELL_APP_EVENT_INTEGRATION_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `bell_app_events`: PASS (4 events)

## Safety / STOP FIRST
- ❌ No notification redesign — uses existing OP_BELL_NOTIFY wire format
- ❌ No blocking waits — fire-and-forget only
- ❌ No new PDX opcodes — uses existing SLOT_BELL (12) + OP_BELL_NOTIFY (0xC0)
- ✅ Bell server independently validates/enqueues; silk-shell is just a sender
- ✅ zero impact on other proofs

## Known Limitations
- Events are sent but not verified received (no read-back from Bell queue)
- Only 4 synthetic events, hardcoded
- No event payload beyond source name + event_id
- Not wired to real app-lifecycle hooks (proof-only)

## Future Follow-up
- Wire to real app open/close lifecycle hooks
- Add event payload (object_id, timestamp)
- Subscribe to Bell generation counter for delivery confirmation
- App-specific policy override per sender PD
