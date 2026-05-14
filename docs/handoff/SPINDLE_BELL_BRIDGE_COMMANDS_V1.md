# Spindle Bell Bridge Commands V1

## Status: PASS
Date: 2026-05-14
Attempts: 2 (first had non-ASCII byte string literal compile error)

## Bell Bridge Safety Audit

| Property | Status | Detail |
|----------|--------|--------|
| SLOT_BELL | 12 | Spindle PD 12 -> sexbell PD 10 |
| Edge type | AsyncEnqueue | fire-and-forget, returns immediately |
| Blocking | None | pdx_call returns (status, _) immediately |
| OP_BELL_NOTIFY | 0xC0 | Existing opcode, no ABI change |
| Bell server receives | YES | [bell.notify.recv] caller_pd=12 confirmed |
| Category | Info (0) | Safe default, no policy violation |
| Urgency | Normal (1) | Maps to PASSIVE lane |
| Privacy | Public (0) | No redaction needed |
| Faults | 0 | No crashes |

## Commands Added

| Command | Args | Bell Send | Description |
|---------|------|-----------|-------------|
| `notify <msg>` | text (optional) | OP_BELL_NOTIFY | Send Bell notification with optional message |
| `bell-test` | none | OP_BELL_NOTIFY | Send test notification with known parameters |
| `bell-status` | none | — (local only) | Report Bell bridge configuration |
| `bell` (updated) | none | — (local only) | Updated to reflect active bridge |

## Notification Format

All notifications use safe defaults:
- category = 0 (Info)
- urgency_hint = 1 (Normal)
- privacy_level = 0 (Public)
- redaction_class = 0 (StructuralMeta)
- action_count = 0 (no actions)
- object_ref_count = 0 (no object refs)

arg0 = 0x00000100, arg1 = 0, arg2 = 0

## Proof Table

| Stage | Action | ok | Bell Recv | Status |
|-------|--------|----|-----------|--------|
| 0 | start | 1 | — | proof begin |
| 1 | bell-status | 1 | — | status report |
| 2 | bell-test | 1 | [bell.notify.recv] | notification received |
| 3 | notify "spindle-proof" | 1 | [bell.notify.recv] | text notification received |
| 4 | notify (empty) | 1 | [bell.notify.recv] | minimal notification received |
| 5 | bell info | 1 | — | bridge status |
| 6 | safety | 1 | — | no blocking verified |

## Runtime Proof Counts

```
[spindle.bell.audit]    slot=12 safe=1 reason=fire_and_forget_async_enqueue  (x2)
[spindle.bell.send]     command=bell-test len=0 status=0 err=0
[spindle.bell.send]     command=notify len=13 status=0 err=0
[spindle.bell.send]     command=notify len=0 status=0 err=0
[spindle.bell.command]  name=bell-status ok=1 reason=status_report
[spindle.bell.command]  name=bell-test ok=1 reason=fire_and_forget
[spindle.bell.command]  name=notify ok=1 reason=fire_and_forget  (x2)
[spindle.bell.proof]    stage=0-6 all ok=1
[spindle.bell.proof.done] ok=1
[bell.notify.recv]      caller_pd=12 category=0 requested=1  (x3)
faults: 0
```

End-to-end proof: Spindle PD 12 sent 3 Bell notifications, and the sexbell server
(PD 10) received all 3 with caller_pd=12. The bridge works.

## Files Changed

`apps/spindle/src/main.rs`
- Updated `bell` command: active bridge status, SLOT_BELL audit marker
- Updated `help` command: added notify, bell-test, bell-status entries
- Added `notify` command handler: sends OP_BELL_NOTIFY with optional message text
- Added `bell-test` command handler: sends test notification
- Added `bell-status` command handler: non-blocking bridge status report
- Added `run_bell_bridge_proof()` proof function (~50 lines)
- Added `BELL_BRIDGE_PROOF_ENABLED` gate

`docs/handoff/SPINDLE_BELL_BRIDGE_COMMANDS_V1.md` (created)

## Build Results
```
SEXOS_SPINDLE_BELL_BRIDGE_PROOF=1 ./scripts/entrypoint_build.sh -> PASS
./scripts/entrypoint_build.sh -> PASS
```

## Notes
- No kernel, sex-pdx/ABI, sexusb, sexinput, sexdisplay, or Quil edits.
- All Bell sends use existing OP_BELL_NOTIFY (0xC0) — no new opcodes.
- No synchronous reply wait — pdx_call returns immediately.
- Bell text content is limited to the Spindle scrollback display; the Bell
  protocol uses fixed numeric fields (category, urgency, etc.) with no text
  payload in V1.
- The existing auto-notify on Enter (line 1054) continues to fire after each
  recognized command.
