# Spindle Linen Bridge Commands V1

## Status: PASS
Date: 2026-05-14
Attempts: 1

## Linen Bridge Safety Audit

| Property | Status | Detail |
|----------|--------|--------|
| SLOT_LINEN | 11 | Spindle PD 12 -> linen PD 7 |
| Edge type | AsyncEnqueue | fire-and-forget, pdx_call returns immediately |
| Blocking | None | No synchronous reply wait |
| OP_LINEN_LIST_OBJECTS | 0x42 | Fire-and-forget, reply async |
| OP_LINEN_OPEN_INTENT | 0x46 | Fire-and-forget, Linen replies immediately server-side |
| OP_LINEN_CREATE_OBJECT | 0x41 | Already used at boot for .spn session |
| Sync readback | Unavailable | AsyncEnqueue edge -- pdx_call returns (0,0) |
| Faults | 0 | No crashes |

## Commands Added

| Command | Args | Linen Opcode | Blocking? | Description |
|---------|------|-------------|-----------|-------------|
| `linen-status` | none | — | No (local only) | Bridge configuration report |
| `linen-list` | none | OP_LINEN_LIST_OBJECTS (0x42) | No (fire-and-forget) | Async list request + honest async-limited message |
| `linen-open <id>` | numeric id | OP_LINEN_OPEN_INTENT (0x46) | No (fire-and-forget) | Open intent dispatch to Linen |

## Honest Async-Limited Messages

All commands that require synchronous readback (list results, open confirm)
explicitly state the limitation:
- `linen-list`: "Synchronous listing unavailable: AsyncEnqueue edge.
   Server reply arrives as type=0x1 in main listen loop."
- `linen-open`: "Object open intent dispatched to Linen server.
   Use silk-shell Linen surface to view result."

No fake blocking, no unbounded waits.

## Proof Table

| Stage | Action | ok | Description |
|-------|--------|----|-------------|
| 0 | start | 1 | proof begin |
| 1 | linen-status | 1 | bridge status report |
| 2 | linen-list | 1 | async list request sent |
| 3 | linen-open 1 | 1 | open intent dispatched |
| 4 | linen-open (missing id) | 1 | graceful reject (missing_id) |
| 5 | safety | 1 | no blocking verified |

## Runtime Proof Counts

```
[spindle.linen.audit]    slot=11 safe=1 reason=fire_and_forget_async_enqueue
[spindle.linen.send]     op=list id=0 status=0 err=0
[spindle.linen.send]     op=open id=1 status=0 err=0
[spindle.linen.command]  name=linen-status ok=1 reason=status_report
[spindle.linen.command]  name=linen-list ok=1 reason=async_limited_static_fallback
[spindle.linen.command]  name=linen-open ok=1 reason=fire_and_forget
[spindle.linen.command]  name=linen-open ok=0 reason=missing_id
[spindle.linen.proof]    stage=0-5 all ok=1
[spindle.linen.proof.done] ok=1
faults: 0
```

## Files Changed

`apps/spindle/src/main.rs`
- Added OP_LINEN_LIST_OBJECTS (0x42) and OP_LINEN_OPEN_INTENT (0x46) constants
- Updated `help` command: added linen-status, linen-list, linen-open entries
- Updated `session` command: Linen bridge status
- Added `linen-status` command handler
- Added `linen-list` command handler (fire-and-forget + honest async-limited message)
- Added `linen-open` command handler (fire-and-forget + id parsing + graceful reject)
- Added `run_linen_bridge_proof()` proof function
- Added LINEN_BRIDGE_PROOF_ENABLED gate

`docs/handoff/SPINDLE_LINEN_BRIDGE_COMMANDS_V1.md` (created)

## Build Results
```
SEXOS_SPINDLE_LINEN_BRIDGE_PROOF=1 ./scripts/entrypoint_build.sh -> PASS
./scripts/entrypoint_build.sh -> PASS
```

## Notes
- No kernel, ABI, opcode, sexusb, sexinput, sexdisplay, or Quil edits.
- All Linen pdx_calls use existing opcodes -- no new protocol.
- No synchronous reply wait -- honest about AsyncEnqueue limitation.
- Linen server-side handlers (handle_list_objects, handle_open_intent) reply
  immediately per the linen server codebase audit.
- Spindle's existing pdx_call(SLOT_LINEN, OP_LINEN_CREATE_OBJECT, 0, 0, 0)
  at boot continues to fire for .spn session creation.
