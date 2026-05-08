# LINEN_OPEN_INTENT_STUB_V1

## Scope
Stub-only OpenIntent round-trip from silk-shell to Linen PD using
bounded PDX call/response. No real app launch, no authority grants.

## Files Changed
- `servers/linen/src/main.rs`
- `servers/silk-shell/src/main.rs`
- `docs/handoff/LINEN_OPEN_INTENT_STUB_V1.md` (this file)

## Opcode
```
OP_LINEN_OPEN_INTENT = 0x46
```

## Protocol

### Request (shell → Linen via SLOT_LINEN)
```
arg0 = object_id       (the selected Linen object id, ≥1)
arg1 = selected_index  (array index of the selected object, or 0)
arg2 = intent_flags    (always 0 in V1)
```

No pointers, no strings, no caps.

### Reply (Linen → shell via pdx_reply)
```
0       = accepted/stubbed (object found in SESSION table)
-3      = object not found
-6      = reserved (owner mismatch, not triggered V1 since lookup uses caller_pd=0)
```

## Linen Implementation (`servers/linen/src/main.rs`)

### Handler: `handle_open_intent(object_id, caller_pd)`
- Calls `SESSION.get(object_id, 0)` — server-internal access, no owner filter.
- On success: logs `[linen.open_intent.recv] id=N kind=K ok=1`, replies 0.
- On failure: logs `[linen.open_intent.recv] id=N ok=0`, replies error code.
- No app launch, no Quil integration, no cap grants.
- No mutation of SESSION.

### Dispatch
Added in main `match msg.type_id`:
```
OP_LINEN_OPEN_INTENT => handle_open_intent(msg.arg0, msg.caller_pd)
```

## Shell Implementation (`servers/silk-shell/src/main.rs`)

### Key Intercept
Added Linen-focused-surface keyboard intercept in the key dispatch chain
(between Spindle intercept and generic `scancode_to_action` dispatch):

```
FOCUSED_SURFACE_ID == SURFACE_ID_LINEN && (scancode == 0x1C || scancode == 0x39)
```

Where:
- `0x1C` = Enter (PS/2 scancode)
- `0x39` = Space (PS/2 scancode)

### Behavior
- Gets selected object via `linen_selected_object_id()` (auto-repairs to first valid).
- If `object_id != 0`: calls `pdx_call(SLOT_LINEN, OP_LINEN_OPEN_INTENT, ...)`, logs `[linen.open_intent.send]`.
- If `object_id == 0`: logs `[linen.open_intent.skip] reason=no_object`.
- Does NOT alter focus.
- Does NOT trigger repaint.
- J/K selection still works via normal `SelectNextLinenObject`/`SelectPrevLinenObject` action dispatch.

### Selected Index
`selected_index` (arg1) is computed by scanning `LINEN_OBJECTS` for the
matching `object_id`. Falls back to 0 if not found (e.g., static UI mode).

## Auth Model
- Caller must have an existing cap edge to Linen (SLOT_LINEN).
- Linen does NOT require caller to own the object (uses `caller_pd=0` for lookup).
- Object must exist in Linen's public SESSION table.
- This is an intent request, not an authority transfer.

## Non-Goals (Explicit)
- NO app/exec/spawn launch
- NO Quil integration
- NO authority/cap grants
- NO file grants
- NO OpenIntent persistence
- NO push/invalidate/notify
- NO SESSION owner filter weakening (public lookup already existed for 0x44/0x45)
- NO sexdisplay edits
- NO sexfiles edits
- NO kernel/sex-pdx edits
- NO cross-PD raw pointers

## Proof Markers
```
[linen.open_intent.send] id=N idx=I     ← shell sends intent
[linen.open_intent.recv] id=N kind=K ok=1  ← Linen accepts valid object
[linen.open_intent.recv] id=N ok=0      ← Linen rejects unknown object
[linen.open_intent.skip] reason=no_object  ← shell skips when nothing selected
```

## Next Phase Options
1. `LINEN_PUSH_INVALIDATE_PLAN_V1` — push-based invalidation when objects change
2. `LINEN_OPEN_INTENT_TO_QUIL_PLAN_V1` — route OpenIntent through Collar to open in Quil buffer
3. `LINEN_PROJECT_NAVIGATION_V1` — project-scoped object lists

## Build Verification
```
./scripts/entrypoint_build.sh → success
```
