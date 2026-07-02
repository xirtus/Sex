# J7: Bell Object Link Event Stub

**Status:** Handoff (implemented)
**Commit:** *(to be committed)*
**Build:** *(to be verified)*

## 1. Purpose

Emit a shell-local Bell placeholder event when a Linen object is linked into a
Quil buffer. No real Bell queue, no notification UI, no new PDX ops, no
renderer changes.

### What J7 IS
- `BellEventKind` enum (4 variants) — event kind identifiers for stub events
- `bell_emit_object_link_event()` — shell-local event emission helper
- Wired into J4 link path after successful link + J6 mesh diagnostic

### What J7 IS NOT
- Not a real Bell event queue or delivery system
- Not a notification surface or UI
- Not a PDX send to a Bell server
- Not a new sex-pdx opcode
- Not attention policy implementation
- Not authority enforcement

## 2. Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +BellEventKind enum, +bell_emit_object_link_event() helper, wired into open_linen_object_in_quil() step 9 |
| `docs/handoff/J7_BELL_OBJECT_LINK_EVENT_V1.md` | This document |

## 3. Event Kind Enum

`BellEventKind` defines the kinds of stub events J7 can emit:

| Variant | Value | Description |
|---------|-------|-------------|
| `ObjectLinkedToBuffer` | 0 | Linen object linked into a Quil buffer |
| `ObjectOpenRequested` | 1 | Linen object open requested (future) |
| `OperationNeedsGrant` | 2 | Operation needs a real Collar grant (future) |
| `DiagnosticOnly` | 3 | Diagnostic observation only (future) |

## 4. Event Helper Behavior

`bell_emit_object_link_event(object_id, buffer_id)` performs these steps:

1. Emit `[bell.event.stub]` with kind, object_id, buffer_id
2. Validate object_id via `linen_object_by_id()` — if missing, emit
   `[bell.event.reject.missing]` and return with `[bell.event.done] reason=rejected`
3. Validate buffer_id via `quil_buffer_by_id()` — if missing, emit
   `[bell.event.reject.missing]` and return with `[bell.event.done] reason=rejected`
4. Cross-check that `buf.linen_object_ref == object_id` — if mismatch, emit
   `[bell.event.reject.missing] reason=buffer_ref_mismatch` and return
5. Emit `[bell.event.object_link]` with object_id, object_kind, buffer_id, buffer_kind
6. Emit `[bell.event.done] reason=emitted`

### Event payload (proof-only, IDs and kind names):
- `object_id` — Linen object ID
- `object_kind` — human-readable LinenObjectKind name
- `buffer_id` — Quil buffer ID
- `buffer_kind` — human-readable QuilBufferKind name

No object contents, no file paths, no raw pointers, no authority mutation.

## 5. Wire Point

Wired into `open_linen_object_in_quil()` at step 9, after the J6 mesh
diagnostic call:

```
step 7:  open Quil surface
step 8:  mesh_emit_linen_quil_links()  [J6]
step 9:  bell_emit_object_link_event(object_id, buffer_id)  [J7]
return true
```

This ensures the Bell event fires after:
- collar gate has allowed (J5)
- buffer table mutation is complete (J4)
- mesh diagnostic facts are emitted (J6)

## 6. Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[bell.event.stub]` | bell_emit_object_link_event() | Entry — event kind, object_id, buffer_id |
| `[bell.event.reject.missing]` | bell_emit_object_link_event() | Object or buffer missing, or ref mismatch |
| `[bell.event.object_link]` | bell_emit_object_link_event() | Valid link with kind names |
| `[bell.event.done]` | bell_emit_object_link_event() | Final result (reason=emitted or reason=rejected) |

### Expected log output on successful link:

```
[bell.event.stub] kind=ObjectLinkedToBuffer object_id=3 buffer_id=3
[bell.event.object_link] object_id=3 object_kind=CodeFile buffer_id=3 buffer_kind=Code
[bell.event.done] reason=emitted
```

### Expected log output on stale ref:

```
[bell.event.stub] kind=ObjectLinkedToBuffer object_id=999 buffer_id=999
[bell.event.reject.missing] object_valid=false buffer_valid=false
[bell.event.done] reason=rejected
```

## 7. Future Boundary

| Capability | J7 Status | Real Bell Requires |
|------------|-----------|-------------------|
| Event queue | ❌ Not implemented | STOP FIRST + rapid gate |
| Notification surface | ❌ Not implemented | STOP FIRST + rapid gate |
| PDX send to Bell server | ❌ Not implemented | STOP FIRST + rapid gate |
| Attention policy | ❌ Not implemented | STOP FIRST + rapid gate |
| Category/priority | ✅ Enum defined | Richer model with payload |
| Object link event | ✅ Stub emitted | Real event with full context |
| Event validation | ✅ Object/buffer validated | Full schema validation |

## 8. Safety Invariants Preserved

1. **No real queue.** Events are proof markers only — never queued or delivered.
2. **No notification UI.** No surface creation, no display primitives.
3. **No PDX.** No inter-server communication.
4. **No heap allocation.** Stack locals and static strings only.
5. **Safe degradation.** Missing objects/buffers produce rejection markers, not panics.
6. **Additive only.** Existing lifecycle, focus, tiling, atlas, close paths unchanged.

## 9. Forbidden Areas Untouched

- kernel/: untouched
- crates/sex-pdx/: untouched
- servers/sexdisplay/: untouched
- servers/linen/: untouched
- servers/quil/: untouched
- WINDOWS Vec: untouched
- Lifecycle enum: untouched
- Tombstone ring: untouched
- Real Bell queue/event bus: untouched
- Real notification UI: untouched
- Real Collar grant enforcement: untouched
- Real Mesh graph renderer: untouched

## 10. STOP FIRST Status

**No STOP FIRST triggers hit.**

| Trigger | Status |
|---------|--------|
| Kernel edits | ✅ Not touched |
| sex-pdx ABI/opcode edits | ✅ Not touched |
| sexdisplay changes | ✅ Not touched |
| New PDX ops | ✅ Not added |
| Authority enforcement | ✅ Not touched |
| Secret/key handling | ✅ Not touched |
| Filesystem/storage | ✅ Not touched |
| Editor/parser/compiler/build | ✅ Not touched |
| Cross-PD raw pointers | ✅ Not used |
| Shared-memory/backing-buffer redesign | ✅ Not touched |
| Real Bell event queue/delivery | ✅ Not implemented |
| Notification UI | ✅ Not implemented |
