# J1: Linen Object Table

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive model only)

## Purpose

Add a minimal in-memory Linen object table as the first real product implementation.
No filesystem, no storage, no PDX ops, no heap allocation. The object table lives
in silk-shell's static data segment.

## Changes to `servers/silk-shell/src/main.rs`

### LinenObjectKind enum (Change 1)
- 11 variants matching H1 §2:
  `Project`, `Document`, `CodeFile`, `MediaAsset`, `BuildArtifact`, `Folder`,
  `Reference`, `ImportPlaceholder`, `BellEventReference`, `QuilWorkspaceReference`,
  `MeshDiagnosticReference`
- `#[repr(u8)]`, `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- Added after `APP_SURFACES`, before A3 lifecycle model section

### LinenObjectState enum (Change 2)
- 5 variants matching H1 §3 lifecycle_state field:
  `Allocated`(0), `Loaded`(1), `Modified`(2), `Saved`(3), `Archived`(4)
- `#[repr(u8)]`

### LinenObject struct (Change 3)
- Fixed-size record with scalar/fixed fields only:
  - `object_id: u64` — unique identifier
  - `kind: LinenObjectKind` — object type
  - `state: LinenObjectState` — lifecycle state
  - `parent_id: u64` — parent folder/project ID (0 = root)
  - `project_id: u64` — project this object belongs to
  - `grant_ref: u64` — Collar grant reference (0 = public/unchecked)
  - `linked_surface_id: u64` — surface this object is open in (0 = none)
  - `flags: u32` — future extensibility
  - `display_name: &'static str` — human-readable name

### Static object table (Change 4)
- `LINEN_OBJECTS: [Option<LinenObject>; 16]` — static array, no heap
- `LINEN_SEED_OBJECTS: [LinenObject; 6]` — compile-time seed data:
  1. Project "SexOS Kernel" (id=1)
  2. Document "Compositor Lifecycle Spec" (id=2, parent=1)
  3. CodeFile "Silk Shell main.rs" (id=3, parent=1, linked to SURFACE_ID_LINEN)
  4. MediaAsset "Desktop Screenshot" (id=4)
  5. BuildArtifact "Current ISO Build" (id=5, parent=1)
  6. Folder "Drafts" (id=6)

### Lookup helpers (Change 5)
- `linen_object_table_init()` — copies seed objects into LINEN_OBJECTS
- `linen_object_count()` — returns number of populated slots
- `linen_object_by_id(id)` — finds object by object_id
- `linen_object_kind_name(kind)` — returns human-readable kind string
- `linen_object_state_name(state)` — returns human-readable state string

### Boot sequence (Change 6)
- `linen_object_table_init()` added after `scene_init_all()`, before `snap_capture_layout()`

### Proof marker in Linen open path (Change 7)
- `[linen.object_table.ready]` emitted after `[linen.placeholder.open]` in `open_linen_in_active_scene()`

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[linen.object_table.init]` | linen_object_table_init() | Boot initialization |
| `[linen.object.seed]` | linen_object_table_init() | Per-seed-object init (x6) |
| `[linen.object_table.ready]` | open_linen_in_active_scene() | Linen surface opened |

## Invariants

1. No filesystem/storage/POSIX path assumptions — all references by object_id
2. No authority enforcement — Collar gate deferred (H6)
3. No renderer/ABI/opcode changes
4. No heap allocation — static array only
5. No cross-PD pointers
6. Table size fixed at 16 slots; seed uses 6
7. All helpers are deterministic and read-only

## Deferred

- J2: Linen object list placeholder UI (render using 0xEC/0xEF)
- J3: Quil buffer table
- J4: Open Linen object into Quil buffer
- Storage/filesystem persistence
- Collar gate integration
- Object create/delete/rename operations

## Dependencies

- **Requires:** D1 (Linen placeholder lifecycle), H1 (Linen object model spec)
- **Blocks:** J2 (list UI), J3 (Quil buffer), J4 (open in Quil)
