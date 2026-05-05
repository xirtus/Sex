# J4: Linen Object → Quil Buffer Link

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive link + proof markers only)

## Purpose

Implement the first safe Linen→Quil handoff: open a Linen object reference into
a Quil buffer slot using shell-local IDs only. No real editor, no storage, no
filesystem, no parser/compiler/build, no new PDX ops.

## Changes to `servers/silk-shell/src/main.rs`

### SurfaceAction::OpenObjectInQuil (Change 1)
- New enum variant after `LegacyFocusToggle`, before D3 accessibility actions
- Bound to scancode 0x59 (PrintScreen)

### Scancode mapping (Change 2)
- `0x59 => Some(SurfaceAction::OpenObjectInQuil)` — test trigger, no existing binding

### open_linen_object_in_quil() function (Change 3)
- `unsafe fn open_linen_object_in_quil(object_id: u64) -> bool`
- Shell-local ID linking only — no cross-PD calls
- Lookup: finds LinenObject by ID via LINEN_OBJECTS iteration
- Grant check: if grant_ref == 0, still allow link but emit [linen.quil.open.no_grant]
- Kind mapping: LinenObjectKind → QuilBufferKind (CodeFile→Code, MediaAsset→LinenObjectView, BuildArtifact→BuildOutput, else→Text)
- Buffer: finds existing buffer with matching linen_object_ref, or creates new slot
- Buffer creation uses object_id as buffer_id (deterministic mapping)
- Updates LinenObject.linked_surface_id = SURFACE_ID_QUIL in-place
- Opens Quil surface via open_quil_in_active_scene() if not already visible
- Emits all proof markers

### Action handler (Change 4)
- `SurfaceAction::OpenObjectInQuil` calls `open_linen_object_in_quil(3)`
- Object ID 3 = seed CodeFile "Silk Shell main.rs" from J1

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[linen.quil.open.request]` | open_linen_object_in_quil() | Start of link attempt |
| `[linen.quil.open.reject.missing]` | open_linen_object_in_quil() | Object ID not found in table |
| `[linen.quil.open.no_grant]` | open_linen_object_in_quil() | grant_ref is 0 (no Collar grant yet) |
| `[linen.quil.buffer.linked]` | open_linen_object_in_quil() | Buffer assigned and linked to object |
| `[linen.quil.quil_opened]` | open_linen_object_in_quil() | Quil surface opened (if not already) |
| `[linen.quil.done]` | open_linen_object_in_quil() | Link complete |

## Test Trigger

- **Key:** PrintScreen (scancode 0x59)
- **Action:** Opens Linen object ID 3 (CodeFile "Silk Shell main.rs") into a Quil buffer
- **Effect:** Creates QuilBuffer { buffer_id: 3, kind: Code, state: Open, linen_object_ref: 3 },
  updates LINEN_OBJECTS[3].linked_surface_id = SURFACE_ID_QUIL, opens Quil surface

## Link Model

Shell-local table references (no LinenQuilLink table needed):

```
LINEN_OBJECTS[2] → linen_object_ref=3 → QUIL_BUFFERS (buffer_id=3, kind=Code)
```

Both tables are static mut arrays in silk-shell. The link is established by:
1. Setting QUIL_BUFFERS[slot].linen_object_ref = object_id
2. Setting LINEN_OBJECTS[idx].linked_surface_id = SURFACE_ID_QUIL

No separate LinenQuilLink table required — the existing ref fields suffice.

## Invariants

1. No editor implementation — buffer table only, no text cursor/rendering
2. No sexfiles calls — object data stays in seed table
3. No Collar grant enforcement — grant_ref=0 still allows link
4. No filesystem/storage access
5. No kernel/ABI/sex-pdx/sexdisplay edits
6. No cross-PD communication
7. Deterministic: same object_id always maps to same buffer_id
8. Duplicate-safe: if buffer already exists, updates state instead of creating new

## Deferred

- Real sexfiles object data loading
- Collar grant check before allowing open
- Editor text rendering and input handling
- Parser/compiler/build integration
- Storage/filesystem persistence
- UI selection of which object to open

## Dependencies

- **Requires:** J1 (Linen object table), J3 (Quil buffer table)
- **Blocks:** J5 (Collar-gated operation stubs)
