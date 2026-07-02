# LINEN_OPEN_INTENT_TO_QUIL_V1

## Scope
Connect Linen OpenIntent reply path to existing `open_linen_object_in_quil()`.
Enter/Space on selected Linen object opens it in Quil (no file content yet).

## Files Changed
- `servers/silk-shell/src/main.rs` (Linen keyboard intercept block only)
- `docs/handoff/LINEN_OPEN_INTENT_TO_QUIL_V1.md` (this file)

## Implementation

### Change in Linen Keyboard Intercept
In the `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN && (scancode == 0x1C || scancode == 0x39)` block:

**Before:** `pdx_call` → discard reply → log send only.

**After:**
1. `pdx_call(SLOT_LINEN, OP_LINEN_OPEN_INTENT, obj_id, idx, 0)` — fire intent
2. `linen_sync_reply()` — spin-wait for Linen reply
3. If `reply == 0`: call `open_linen_object_in_quil(obj_id)`, log `[linen.open_intent.quil.open] ok=1`
4. If `reply != 0`: log `[linen.open_intent.quil.open] ok=0 err=R`, do NOT open Quil

### Gate Preservation
- **Collar gate**: `open_linen_object_in_quil()` internally calls `collar_check_operation()` for `LinkObjectToBuffer` and `AccessSexFiles`. No bypass.
- **Quil buffer safety**: `open_linen_object_in_quil()` handles buffer collision, full table, missing object. No new failure modes.
- **Linen validation**: `OP_LINEN_OPEN_INTENT` on Linen still validates via `SESSION.get(object_id, 0)`. Non-existent objects reply with error, preventing Quil open.

### Sync Reply Pattern
Uses `linen_sync_reply()` — same pattern as `linen_fetch_remote_snapshot()`.
Briefly blocks the event loop for one PDX round-trip (Linen reply is immediate:
`SESSION.get()` + `pdx_reply()`). Impact: minimal, identical to existing snapshot fetch.

## Proof Markers

### Full Happy Path
```
[linen.object_select.current] id=N            ← selection active
[linen.open_intent.send] id=N idx=I           ← shell sends intent
[linen.open_intent.recv] id=N kind=K ok=1     ← Linen accepts
[linen.open_intent.quil.open] id=N idx=I ok=1 ← shell routes to Quil
[linen.quil.open.request] id=N                ← open_linen_object_in_quil begins
[collar.policy.check] op=6 object_id=N ...    ← Collar gate check (LinkObjectToBuffer)
[collar.policy.check] op=8 ...                ← Collar cap check (AccessSexFiles)
[linen.quil.open.dynamic_id] object_id=N ...  ← buffer allocated
[linen.quil.buffer.linked] object_id=N ...    ← buffer linked to object
[linen.quil.quil_opened] object_id=N          ← Quil surface 201 opened
[linen.quil.done] object_id=N ...             ← complete
```

### Not-Found Path
```
[linen.open_intent.send] id=N idx=I
[linen.open_intent.recv] id=N ok=0            ← Linen: object not found
[linen.open_intent.quil.open] id=N idx=I ok=0 err=... ← shell: Quil NOT opened
```

### No-Selection Path
```
[linen.open_intent.skip] reason=no_object
```

### Collar Block Path (if gate returns Deny)
```
[linen.open_intent.send] id=N idx=I
[linen.open_intent.recv] id=N kind=K ok=1
[linen.open_intent.quil.open] id=N idx=I ok=1
[linen.quil.open.request] id=N
[linen.quil.open.reject.cap] ...              ← Collar blocks
(Quil NOT opened)
```

## Non-Goals (Explicit)
- NO file content transfer (object metadata only: id, kind, name)
- NO real Collar authority grants (AllowStub continues)
- NO new PDs, opcodes, or PDX routes
- NO kernel/ABI/sex-pdx/sexdisplay changes
- NO SESSION owner filter weakening
- NO persistence of Quil buffer links
- NO push invalidation
- NO real exec/spawn

## Build Verification
```
./scripts/entrypoint_build.sh → success
```

## Next Phase
`LINEN_V1_FINAL_RUNTIME_PROOF` — end-to-end runtime proof of the full Linen V1 skeleton:
selection → OpenIntent → Quil open → Collar gate → buffer link → surface render.
Or `LINEN_PUSH_INVALIDATE_PLAN_V1` if live invalidation is priority.
