# J3: Quil Buffer Table

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive model only)

## Purpose

Add a minimal in-memory Quil buffer table as the first real Quil product implementation.
No editor, parser, compiler, build, filesystem, or storage. The buffer table lives
in silk-shell's static data segment, mirroring the J1 Linen object table pattern.

## Changes to `servers/silk-shell/src/main.rs`

### QuilBufferKind enum (Change 1)
- 8 variants mapping to H2 §2 workstation object types:
  `Text`(0), `Code`(1), `DesignNote`(2), `ReviewNote`(3), `Diagnostic`(4),
  `BuildOutput`(5), `AgentTask`(6), `LinenObjectView`(7)
- `#[repr(u8)]`, `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- Added after J2 code, before A3 lifecycle model section

### QuilBufferState enum (Change 2)
- 6 variants: `Allocated`(0), `Open`(1), `Dirty`(2), `ReadOnly`(3), `Closed`(4), `Missing`(5)
- `#[repr(u8)]`

### QuilBuffer struct (Change 3)
- Fixed-size record with scalar/fixed fields only:
  - `buffer_id: u64` — unique identifier
  - `kind: QuilBufferKind` — buffer type (Text, Code, etc.)
  - `state: QuilBufferState` — lifecycle state
  - `linen_object_ref: u64` — referenced Linen object ID (0 = none)
  - `project_id: u64` — project this buffer belongs to
  - `grant_ref: u64` — Collar grant reference (0 = public/unchecked)
  - `linked_surface_id: u64` — surface this buffer is shown on
  - `flags: u32` — future extensibility
  - `display_name: &'static str` — human-readable name

### Static buffer table (Change 4)
- `QUIL_BUFFERS: [Option<QuilBuffer>; 16]` — static array, no heap
- `QUIL_SEED_BUFFERS: [QuilBuffer; 6]` — compile-time seed data:
  1. Code "main.rs" (id=1, linked to SURFACE_ID_QUIL)
  2. Text "Compositor Lifecycle Spec" (id=2, linen_object_ref=2)
  3. DesignNote "Frame Tiling Design" (id=3)
  4. BuildOutput "Current ISO Build" (id=4, linen_object_ref=5, ReadOnly)
  5. ReviewNote "Review: A7 Opcode Audit" (id=5)
  6. AgentTask "Refactor tiling loop" (id=6, Allocated)

### Lookup helpers (Change 5)
- `quil_buffer_table_init()` — copies seed buffers into QUIL_BUFFERS
- `quil_buffer_count()` — returns number of populated slots
- `quil_buffer_by_id(id)` — finds buffer by buffer_id
- `quil_buffer_kind_name(kind)` — returns human-readable kind string
- `quil_buffer_state_name(state)` — returns human-readable state string

### Boot sequence (Change 6)
- `quil_buffer_table_init()` added after `linen_object_table_init()`, before `snap_capture_layout()`

### Proof marker in Quil open path (Change 7)
- `[quil.buffer_table.ready]` emitted after `[quil.placeholder.open]` in `open_quil_in_active_scene()`

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[quil.buffer_table.init]` | quil_buffer_table_init() | Boot initialization |
| `[quil.buffer.seed]` | quil_buffer_table_init() | Per-seed-buffer init (x6) |
| `[quil.buffer_table.ready]` | open_quil_in_active_scene() | Quil surface opened |

## Invariants

1. No editor/parser/compiler/build implementation — buffer table only
2. No filesystem/storage/POSIX path assumptions
3. No authority enforcement — Collar gate deferred
4. No renderer/ABI/opcode changes
5. No heap allocation — static array only
6. No cross-PD pointers
7. Table size fixed at 16 slots; seed uses 6
8. All helpers are deterministic and read-only

## Deferred

- J4: Open Linen object into Quil buffer
- Editor implementation (text rendering, cursor, input)
- Parser/compiler/build integration
- Storage/filesystem persistence
- Collar gate integration
- Buffer create/delete/close operations

## Dependencies

- **Requires:** E1 (Quil placeholder lifecycle), H2 (Quil workstation model spec)
- **Blocks:** J4 (open Linen into Quil), J5 (Collar-gated stubs)
