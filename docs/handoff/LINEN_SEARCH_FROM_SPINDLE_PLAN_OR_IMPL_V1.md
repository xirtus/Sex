# LINEN_SEARCH_FROM_SPINDLE_PLAN_OR_IMPL_V1 — Handoff

## Goal
Determine whether Spindle can safely request Linen object search without new
ABI or blocking waits.  Document the exact blocker if not possible.

## Result: BLOCKED — Documented

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | `linen-search` dispatch arm with honest audit markers | +16 |

## Architecture Audit
Spindle has `SLOT_LINEN` (PD 7: linen) with fire-and-forget PDX edge.
Existing Linen opcodes available to Spindle:
| Opcode | Name | Safe? | Note |
|--------|------|-------|------|
| 0x41 | OP_LINEN_CREATE_OBJECT | Sync reply | Returns object_id; no async variant |
| 0x42 | OP_LINEN_LIST_OBJECTS | Fire-and-forget | Enumerates; reply async via type=0x1 |
| 0x43 | OP_LINEN_GET_OBJECT | Sync reply | Returns name data; no async variant |
| 0x44 | OP_LINEN_GET_PUBLIC_SNAPSHOT | Sync reply | Per-slot snapshot |
| 0x45 | OP_LINEN_GET_PUBLIC_NAME | Sync reply | Name chunk read |
| 0x46 | OP_LINEN_OPEN_INTENT | Fire-and-forget | Stub — no app launch |

**Missing**: `OP_LINEN_SEARCH_OBJECTS`. Linen's `linen_search_by_token()` is a
local in-memory function, not exposed via PDX.

## Blocker
```
OP_LINEN_SEARCH_OBJECTS does not exist in Linen's PDX opcode table.
Adding it requires:
  1. New opcode definition (0x47 or higher)
  2. Linen server handler that calls linen_search_by_token()
  3. Reply packing for match count + first match object_id
  => New ABI: violates "no ABI edits" hard rule.
```

## Workaround
Use `OP_LINEN_LIST_OBJECTS` (0x42) to enumerate all objects, then filter
client-side in Spindle.  Limited: no substring/tag search, returns one
object per call, requires iterative polling.

## Markers (serial)
```
[spindle.linen.search.send] token=NAME status=N err=N
[spindle.linen.workflow.command] name=linen-search ok=0 reason=no_opcode_abi_blocker
```

## Included In
The `linen-search` command is part of `SPINDLE_LINEN_WORKFLOW_COMMANDS_V1`
and proved by `SEXOS_SPINDLE_LINEN_WORKFLOW_PROOF=1`.

## Safety / STOP FIRST
- ❌ No new ABI — blocker documented, not worked around
- ❌ No fake search — honest ok=0 with exact reason
- ❌ No blocking PDX sync readback attempted
- ✅ Existing Linen opcodes unchanged

## Future Follow-up
- OP_LINEN_SEARCH_OBJECTS (0x47): arg0=token packed, reply=match_count + first_id
- Client-side filter wrapper using OP_LINEN_LIST_OBJECTS loop
- Search index in Linen server (tag table integration)
- PDX opcode reservation protocol for new app opcodes
