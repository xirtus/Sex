# Bell From System Events V1

## Status: PASS
Date: 2026-05-14
Attempts: 1

## Summary
Seeded 4 system milestone Bell events into the local ring so the Bell
overlay shows meaningful events beyond the Bell server's demo event.

## System Events Seeded

| event_id | Source | object_id | Description |
|----------|--------|-----------|-------------|
| 0 | keyboard_ready | 8001 | Keyboard GUI daily-driver proven |
| 1 | palette_ready | 8002 | Command palette status proven |
| 2 | spindle_ready | 8003 | Spindle terminal focus proven |
| 3 | atlas_theme_applied | 8004 | Atlas theme apply visual proven |

All events use `BellEventKind::ObjectLinkedToBuffer` with reserved
object_id range 8000-8999 to distinguish system milestones from
J4/J7 Linen→Quil links.

## Runtime Proof Counts

```
[bell.system.event.seed]   event_id=0 source=keyboard_ready       ok=1
[bell.system.event.seed]   event_id=1 source=palette_ready        ok=1
[bell.system.event.seed]   event_id=2 source=spindle_ready        ok=1
[bell.system.event.seed]   event_id=3 source=atlas_theme_applied  ok=1
[bell.system.event.list]   total=4 ok=1
[bell.system.event.detail] event_id=3 ok=1
[bell.system.proof]        stage=0-7 all ok=1
[bell.system.proof.done]   ok=1
faults: 0
```

## Files Changed

`servers/silk-shell/src/main.rs`
- Added BELL_SYSTEM_EVENTS_PROOF_ENABLED const
- Added BELL_SYSTEM_EVENTS_PROOF_DONE static flag
- Added `maybe_run_bell_system_events_proof()` proof function (~70 lines)
- Added proof call site in main loop

`docs/handoff/BELL_FROM_SYSTEM_EVENTS_V1.md` (created)

## Build Results
```
SEXOS_BELL_SYSTEM_EVENTS_PROOF=1 ./scripts/entrypoint_build.sh -> PASS
./scripts/entrypoint_build.sh -> PASS
```

## Notes
- No kernel, ABI, USB, display, Quil, or pointer edits.
- System events use the existing `bell_record_event()` local ring API.
- Reserved object_id range 8000-8999 prevents collision with real
  Linen/Quil object IDs (typically 1-1000).
- Bell detail opens the newest event (atlas_theme_applied, event_id=3)
  at row 0, proving the full seed→list→detail chain.
- No blocking waits — all seeding is local memory writes.
